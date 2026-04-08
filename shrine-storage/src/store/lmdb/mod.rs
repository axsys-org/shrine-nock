//! A database to back the namespace.
//!
//! Modulo path interning, there are four key value stores that back the namespace.
//!
//! - x: (path, case) -> Saga | Ever
//! - y: (path, case) -> Ever + array[(path, case)] (children)
//! - z: (path, case) -> Ever + array[(path, case)] (descendants)
//! - parents: (path) -> (unit path)

use std::path::Path;
use lmdb::DatabaseFlags;
use roaring::RoaringBitmap;
use thiserror::Error;
use tracing::warn;

use crate::{core::{path::PathIdx, types::{Care, Case, Ever, Observe, Span, Tale}}, store::{lmdb::{current::CurrentDb, family::ChildrenDb, key::PathKey, log::LogDb, parent::AncestryDb, saga::{DiskEpic, DiskSaga, SagaDb}}}};

pub mod key;
pub mod log;
pub mod current;
pub mod parent;
pub mod family;
pub mod saga;

#[cfg(test)]
pub mod tests;

pub struct StoreCtx {}

impl StoreCtx {
  pub fn new() -> Self {
    Self {}
  }

  pub fn span(&self) -> Span{
    Span::new()
  }
}
// use lmdb;
pub struct LmdbStore {
  env: heed::Env,
  exe: SagaDb,
  why: ChildrenDb,
  zed: ChildrenDb,
  ancestry: AncestryDb,
  current: CurrentDb,
  log: LogDb,
  ctx: StoreCtx,
}

#[derive(Error, Debug)]
pub enum LmdbError {
  #[error("lmdb error: {0}")]
  LmdbError(#[from] lmdb::Error),
  #[error("heed error: {0}")]
  HeedError(#[from] heed::Error),
  #[error("Flag mismatch: {expected:?} != {actual:?}")]
  FlagMismatch { expected: DatabaseFlags, actual: DatabaseFlags },
  #[error("Invalid key length: {0}")]
  InvalidKeyLength(usize),
  #[error("DB corrupt {0}")]
  Sane(&'static str)
}

pub type Result<T> = std::result::Result<T, LmdbError>;


impl LmdbStore {
  pub fn new() -> Result<Self> {
    let path = Path::new("/tmp/shrine-store");
    Self::new_at_path(path)
  }

  pub fn new_at_path(path: &Path) -> Result<Self> {
    std::fs::create_dir_all(&path).unwrap();
    let env = unsafe {
      heed::EnvOpenOptions::new()
          .map_size(100 * 1024 * 1024)
          .max_dbs(15).max_readers(10)
          .open(&path)?
    };

    let exe = SagaDb::new(&env)?;
    let why = ChildrenDb::create(&env, "why")?;
    let zed = ChildrenDb::create(&env, "zed")?;
    let ancestry = AncestryDb::create(&env, DatabaseFlags::empty())?;
    let current = CurrentDb::create(&env)?;
    let log = LogDb::new(&env)?;

    Ok(Self {
      env,
      current,
      exe,
      why,
      zed,
      ancestry,
      log,

      ctx: StoreCtx::new(),
    })
  }

  pub fn open_at_path(path: &Path) -> Result<Self> {
    let env = unsafe {
      heed::EnvOpenOptions::new()
          .map_size(10 * 1024 * 1024)
          .max_dbs(10).max_readers(10)
          .open(&path)?
    };
    let exe = SagaDb::open(&env)?;
    let why = ChildrenDb::open(&env, "why")?;
    let zed = ChildrenDb::open(&env, "zed")?;
    let ancestry = AncestryDb::open(&env, DatabaseFlags::empty())?;
    let current = CurrentDb::open(&env)?;
    let log = LogDb::open(&env)?;
    Ok(Self { log, env, exe, why, zed, ancestry, current, ctx: StoreCtx::new() })
  }

  pub fn log(&self) -> &LogDb {
    &self.log
  }

  pub fn ancestry(&self) -> &AncestryDb {
    &self.ancestry
  }

  pub fn current(&self) -> &CurrentDb {
    &self.current
  }

  pub fn exe(&self) -> &SagaDb {
    &self.exe
  }
  pub fn why(&self) -> &ChildrenDb {
    &self.why
  }
  pub fn zed(&self) -> &ChildrenDb {
    &self.zed
  }

  pub fn env(&self) -> &heed::Env {
    &self.env
  }

  pub fn poke(&self, wtxn: &mut heed::RwTxn, ctx: &StoreCtx, pax: &str, tale: &Tale) -> Result<(PathIdx, Ever)> {
    let idx = self.ancestry.resolve_path(&wtxn, pax)?.expect("path not found in poke");
    let latest = self.current.get(wtxn, idx.raw())?.expect("no current for path in poke");
    let mut next = latest.clone();
    next.bump_x(ctx, false);
    let path_key = PathKey(idx, latest.x.data);
    let mut saga = self.exe.get(&wtxn, path_key)?.expect("no saga for path in poke");
    saga.merge_tale(ctx, tale)?;
    self.exe.put(wtxn, PathKey(idx, next.x.data), &saga)?;
    {

      let mut family = self.why.get(wtxn, PathKey(idx, latest.y.data))?.expect("no family for path in poke");
      family.parent = *saga.ever();
      self.why.put(wtxn, PathKey(idx, next.y.data), &family)?;
    }

    {
      let mut family = self.zed.get(wtxn, PathKey(idx, latest.z.data))?.expect("no family for path in poke");
      family.parent = *saga.ever();
      self.zed.put(wtxn, PathKey(idx, next.z.data), &family)?;
    }

    assert_eq!(saga.ever(), &next);


    self.current.put(wtxn, idx.raw(), &saga.ever())?;
    Ok((idx, saga.ever().clone()))
  }

  pub fn make(&self, wtxn: &mut heed::RwTxn, ctx: &StoreCtx, pax: &str, changed: &mut RoaringBitmap, visited_why: &mut RoaringBitmap, visited_zed: &mut RoaringBitmap, tale: &Tale) -> Result<(PathIdx, Ever)> {
    let idx = self.ancestry.upsert_path(wtxn, pax)?;
    let _aaa = self.ancestry.title(wtxn, idx)?;
    let latest = self.current.get(wtxn, idx.raw())?.unwrap_or_else(|| Ever::default());
    let mut next = latest.clone();

    next.bump_x(ctx, true);
    let saga = DiskSaga::new(ctx, &next, tale)?;
    self.exe.put(wtxn, PathKey(idx, next.x.data as u32), &saga)?;

    // update family
    {
      let mut family = self.why.get(wtxn, PathKey(idx, latest.y.data))?.unwrap_or_default();
      family.parent = saga.ever().clone();
      self.why.put(wtxn, PathKey(idx, next.y.data), &family)?;
    }

    {
      let mut family = self.zed.get(wtxn, PathKey(idx, latest.z.data))?.unwrap_or_default();
      family.parent = saga.ever().clone();
      self.zed.put(wtxn, PathKey(idx, next.z.data), &family)?;
    };

    let ancestry = self.ancestry.title(wtxn, idx)?;
    let par = ancestry.last().cloned();



    for anc in &ancestry {

      let cur_anc = self.current.get(wtxn, anc.raw())?.expect("no current for ancestor in make");
      let mut next_anc = cur_anc.clone();
      if !changed.contains(anc.raw()) {
        next_anc.bump_z(ctx, true);
        let str = self.ancestry().path_string(wtxn, anc.clone())?.unwrap_or_else(|| "unknown".to_string());

        self.current.put(wtxn, anc.raw(), &next_anc)?;
        visited_zed.insert(anc.raw());
        changed.insert(anc.raw());
      }

      // SAME HACK AS ABOVE
      let old_key = PathKey(anc.clone(), cur_anc.z.data);
      let mut family = self.zed.get(wtxn, old_key)?.expect("Failed to get family for ancestor");
      family.parent = next_anc.clone();
      let next_idx = PathKey(idx, next.x.data);
      let mut replaced = false;
      for c in &mut family.children {
        if next_idx.0 == c.0 {
          c.1 = next_idx.1;
          replaced = true;
          break;
        }
      }
      if !replaced {
        family.children.push(next_idx);
      }
      let _anc_str = self.ancestry().path_string(wtxn, anc.clone())?;
      let _new_key = PathKey(anc.clone(), next_anc.z.data);
      self.zed.put(wtxn, PathKey(anc.clone(), next_anc.z.data), &family)?;
    }

    if let Some(parent) = par {
      let cur_parent = self.current.get(wtxn, parent.raw())?.unwrap_or_else(|| {
        warn!("parent {:?}", self.ancestry.by_id().get(wtxn, &parent.raw()).unwrap().unwrap());
        warn!("id {parent:?}");
        panic!("no current for parent in make {parent:?}");
      });
      let mut next_parent = cur_parent.clone();
      if visited_zed.contains(parent.raw()) && !visited_why.contains(parent.raw()) {
        // published new case

        next_parent.bump_y(ctx, true);
        let str = self.ancestry().path_string(wtxn, parent.clone())?.unwrap_or_else(|| "unknown".to_string());

        println!("bumping y {str}");

        // will be marked changed when z iterates below
        // changed.insert(parent.raw());
        self.current.put(wtxn, parent.raw(), &next_parent)?;
        visited_why.insert(parent.raw());
      }

      let mut family = self.why.get(wtxn, PathKey(parent, cur_parent.y.data))?.unwrap_or_default();
      family.parent = next_parent.clone();
      let next_idx = PathKey(idx, next.x.data);
      let mut replaced = false;
      for c in &mut family.children {
        if next_idx.0 == c.0 {
          c.1 = next_idx.1;
          replaced = true;
          break;
        }
      }
      if !replaced {
        family.children.push(next_idx);
      }

      self.why.put(wtxn, PathKey(parent, next_parent.y.data), &family)?;
    }
    for anc in &ancestry {
        changed.insert(anc.raw());
    }


    self.current.put(wtxn, idx.raw(), &next)?;

    Ok((idx, next))
  }

  pub fn cull(&self, wtxn: &mut heed::RwTxn, ctx: &StoreCtx, pax: &str) -> Result<(PathIdx, Ever)> {
    let idx = self.ancestry.resolve_path(wtxn, pax)?.expect("path not found in cull");
    let mut descendants = self.ancestry.descendants(wtxn, idx)?;
    descendants.push(idx);
    let mut ret = None;
    for member in descendants {
      let mut current = self.current.get(wtxn, member.raw())?.expect("no current for path in cull");
      current.bump_x(ctx, true);
      self.current.put(wtxn, member.raw(), &current)?;
      ret = Some(current.clone());



    }
    Ok((idx, ret.unwrap()))
  }

  pub fn get_descendants(&self, wtxn: &mut heed::RwTxn, idx: PathIdx) -> Result<Vec<PathIdx>> {
    self.ancestry.descendants(wtxn, idx)
  }

  pub fn bump_why(&self, wtxn: &mut heed::RwTxn, idx: PathIdx, shape: bool) -> Result<Ever> {
    let mut current = self.current.get(wtxn, idx.raw())?.expect("no current for path in bump_why");
    let Some(mut why_family) = self.why.get(wtxn, PathKey(idx, current.y.data))? else {
      self.why.scan_versions(wtxn, idx)?;
      return Err(LmdbError::Sane("missing shit in whydb"));
    };

    let mut zed_family = self.zed.get(wtxn, PathKey(idx, current.z.data))?.unwrap_or_default();
    current.bump_y(&self.ctx, shape);
    current.bump_z(&self.ctx, shape);
    why_family.parent = current.clone();
    zed_family.parent = current.clone();
    self.why.put(wtxn, PathKey(idx, current.y.data), &why_family)?;
    self.zed.put(wtxn, PathKey(idx, current.z.data), &zed_family)?;
    self.current.put(wtxn, idx.raw(), &current)?;
    Ok(current.clone())
  }

  pub fn bump_zed(&self, wtxn: &mut heed::RwTxn, idx: PathIdx, shape: bool) -> Result<Ever> {
    let mut current = self.current.get(wtxn, idx.raw())?.expect("no current for path in bump_zed");
    let mut family = self.zed.get(wtxn, PathKey(idx, current.z.data))?.unwrap_or_default();
    current.bump_z(&self.ctx, shape);
    family.parent = current.clone();
    self.zed.put(wtxn, PathKey(idx, current.z.data), &family)?;
    self.current.put(wtxn, idx.raw(), &current)?;
    Ok(current.clone())
  }

  pub fn look_saga(&self, txn: &heed::RoTxn, path: &str) -> Result<Observe<DiskSaga>> {
    let Some(path) = self.ancestry.resolve_path(txn, path)? else {
      return Ok(Observe::Unknown)
    };

    let latest = self.current.get(txn, path.raw())?.expect("no current for path in look_saga");
    self.peek_saga(txn, latest.x.data, path)

  }

  pub fn scry_saga(&self, txn: &heed::RoTxn, path: &str, case: Case) -> Result<Observe<DiskSaga>> {
    let Some(path) = self.ancestry.resolve_path(txn, path)? else {
      return Ok(Observe::Unknown)
    };
    self.peek_saga(txn, case, path)
  }

  pub fn peek_saga(&self, txn: &heed::RoTxn, case: Case, idx: PathIdx) -> Result<Observe<DiskSaga>> {
    let Some(saga) = self.exe.get(txn, PathKey(idx, case))? else {
      return Ok(Observe::Null)
    };
    Ok(Observe::Found(saga))
  }

  pub fn peek(&self, txn: &heed::RoTxn, case: Case, care: Care, idx: PathIdx) -> Result<Observe<DiskEpic>> {
    match care {
      Care::Y => {
        let Some(family) = self.why.get(txn, PathKey(idx, case))? else {
          warn!("no family for path in peek, definitely weird {idx:?} {case:?}");
          return Ok(Observe::Unknown)
        };
        let Some(fil) = self.exe.get(txn, PathKey(idx, family.parent.x.data))? else {
          return Ok(Observe::Unknown)
        };
        let mut epic = DiskEpic::new();
        epic.insert(idx, fil);
        for child in &family.children {
          let Observe::Found(child_saga) = self.peek_saga(txn, child.1, child.0)? else {
            warn!("no saga for child in peek, definitely weird {child:?}");
            continue;
          };
          epic.insert(child.0, child_saga);
        }
        Ok(Observe::Found(epic))
      },
      Care::Z => {
        let Some(family) = self.zed.get(txn, PathKey(idx, case))? else {
          return Ok(Observe::Null)
        };
        let Some(fil) = self.exe.get(txn, PathKey(idx, family.parent.x.data))? else {
          return Ok(Observe::Unknown)
        };
        let mut epic = DiskEpic::new();
        epic.insert(idx, fil);
        for child in &family.children {
          let Observe::Found(child_saga) = self.peek_saga(txn, child.1, child.0)? else {
            warn!("no saga for child in peek, definitely weird {child:?}");
            continue;
          };
          epic.insert(child.0, child_saga);
        }
        Ok(Observe::Found(epic))
      },
      _ => {
        let mut epic = DiskEpic::new();
        match self.peek_saga(txn, case, idx)? {
          Observe::Found(saga) => {
            epic.insert(idx, saga);
            Ok(Observe::Found(epic))
          },
          Observe::Null => {
            Ok(Observe::Found(epic))
          },
          Observe::Unknown => {
            Ok(Observe::Unknown)
          }
        }
      }
    }
  }


  pub fn look(&self, txn: &heed::RoTxn, care: Care, idx: PathIdx) -> Result<Observe<DiskEpic>> {
    let Some(ever) = self.current.get(txn, idx.raw())? else {
      warn!("missing current for look {idx:?}");

      return Ok(Observe::Unknown)
    };
    let case = match care {
      Care::X => ever.x.data,
      Care::Y => ever.y.data,
      Care::Z => ever.z.data,
    };
    self.peek(txn, case, care, idx)
  }
}
