use std::{
    backtrace::Backtrace,
    collections::{HashMap, HashSet},
    path::Path,
};

use crate::{
    DiskPail,
    core::{
        path::{PathIdx, path_noun_from_string, path_to_segments},
        types::{Care, Case, Epic, Ever, Gift, Mode, Note, Observe},
    },
    overlay::Tray,
    store::lmdb::{
        LmdbError, LmdbStore, StoreCtx,
        key::PathKey,
        saga::{DiskEpic, DiskSaga, strip_prefix_disk_epic},
    },
};
use milton::{
    Noun, food::Chow, jets::{self}, traits::NounConversionError
};
use roaring::RoaringBitmap;
use std::result::Result as StdResult;
use thiserror::Error;
use tracing::{info, warn};

pub mod lmdb;
pub mod traits;

#[derive(Error, Debug)]
pub enum NamespaceError {
    #[error("lmdb error: {source}")]
    LmdbError {
        #[from]
        source: LmdbError,
        #[backtrace]
        backtrace: Backtrace,
    },
}

unsafe impl Send for NamespaceError {}
unsafe impl Sync for NamespaceError {}

impl Into<Box<dyn std::error::Error + Send>> for NamespaceError {
    fn into(self) -> Box<dyn std::error::Error + Send> {
        Box::new(self)
    }
}

impl From<heed::Error> for NamespaceError {
    fn from(error: heed::Error) -> Self {
        NamespaceError::LmdbError {
            source: LmdbError::HeedError(error),
            backtrace: Backtrace::capture(),
        }
    }
}

type Result<T> = std::result::Result<T, NamespaceError>;

pub struct Namespace {
    store: LmdbStore,
    chow: Chow,
}

impl Namespace {
    pub fn new() -> Result<Self> {
        let store = LmdbStore::new()?;
        let chow = Chow::new();
        Ok(Self { store, chow })
    }

    pub fn new_at_path(path: &Path) -> Result<Self> {
        let store = LmdbStore::new_at_path(path)?;
        let chow = Chow::new();
        Ok(Self { store, chow })
    }

    pub fn open_at_path(path: &Path) -> Result<Self> {
        let store = LmdbStore::open_at_path(path)?;
        let chow = Chow::new();
        Ok(Self { store, chow })
    }

    pub fn write(&self, fate: Noun, notes: &mut [Note]) -> Result<Vec<Gift>> {
        notes.sort_by_key(|note| note.path.len());
        let gifts = self.write_presorted(fate, notes)?;
        Ok(gifts)
    }

    pub fn store(&self) -> &LmdbStore {
        &self.store
    }

    pub fn chow(&self) -> &Chow {
        &self.chow
    }

    fn _undisk(&self, res: Observe<DiskEpic>) -> StdResult<Observe<Epic>, NounConversionError> {
        match res {
            Observe::Found(disk) => {
                let mut epic = Epic::new();
                for (key, saga) in disk.iter() {
                    let key = self.path_idx_to_str(key).unwrap().unwrap();
                    let saga = saga.to_saga();
                    epic.insert(key, saga);
                }
                Ok(Observe::Found(epic))
            }
            Observe::Null => Ok(Observe::Null),
            Observe::Unknown => Ok(Observe::Unknown),
        }
    }

    pub fn debug_current(&self) {
        let txn = self.store.env().read_txn().unwrap();
        self.store.current().debug(&txn);
        self.store.ancestry().debug(&txn);
        println!("===== Y ======");
        self.store.why().debug(&txn);
        println!("===== Z ======");
        self.store.zed().debug(&txn);
    }

    #[tracing::instrument(skip_all)]
    pub fn write_presorted(&self, fate: Noun, notes: &[Note]) -> Result<Vec<Gift>> {
        let mut txn = self.store.env().write_txn()?;
        let mut updated = RoaringBitmap::new();
        let mut created = RoaringBitmap::new();
        let mut deleted = RoaringBitmap::new();
        let mut changed = RoaringBitmap::new();
        let _visited = RoaringBitmap::new();
        let ctx = StoreCtx::new();
        let mut order = vec![];
        let mut gifts = vec![];
        let mut visited_why = RoaringBitmap::new();
        let mut visited_zed = RoaringBitmap::new();
        let jammed = fate.jam();
        // Write all new sagas, keeping track of changed
        // tracing::error!("writing {notes:?}");
        let _ = self.store().log().append(&mut txn, jammed);
        for note in notes {
            let s = &note.path;
            tracing::info!("writing {s:?}");
            match note.mode {
                Mode::Add => {
                    let (idx, ever) = self.store.make(
                        &mut txn,
                        &ctx,
                        &note.path,
                        &mut changed,
                        &mut visited_why,
                        &mut visited_zed,
                        &note.slots,
                    )?;
                    created.insert(idx.raw());
                    order.push(idx);
                    changed.insert(idx.raw());
                    let gift = Gift {
                        care: Care::X,
                        mode: Some(Mode::Add),
                        ever,
                        slots: note.slots.keys().cloned().collect(),
                        path: idx,
                    };
                    info!("inserted {gift:?}");
                    gifts.push(gift);
                }
                Mode::Dif => {
                    let (idx, ever) = self.store.poke(&mut txn, &ctx, &note.path, &note.slots)?;
                    updated.insert(idx.raw());
                    order.push(idx);
                    changed.insert(idx.raw());
                    let gift = Gift {
                        care: Care::X,
                        mode: Some(Mode::Dif),
                        ever,
                        slots: note.slots.keys().cloned().collect(),
                        path: idx,
                    };
                    info!("inserted {gift:?}");
                    gifts.push(gift);
                }
                Mode::Del => {
                    let (idx, ever) = self.store.cull(&mut txn, &ctx, &note.path)?;
                    deleted.insert(idx.raw());
                    order.push(idx);
                    changed.insert(idx.raw());
                    let gift = Gift {
                        care: Care::X,
                        mode: Some(Mode::Del),
                        ever,
                        slots: HashSet::new(),
                        path: idx,
                    };
                    info!("inserted {gift:?}");
                    gifts.push(gift);
                }
            }
        }

        for parent in &visited_why {
            let Some(ever) = self.store.current().get(&txn, parent)? else {
                warn!("weird ever");
                continue;
            };
            gifts.push(Gift {
                care: Care::Y,
                mode: Some(Mode::Add),
                ever,
                slots: HashSet::new(),
                path: PathIdx::new(parent),
            });
        }

        for parent in &visited_zed {
            let Some(ever) = self.store.current().get(&txn, parent)? else {
                warn!("weird ever");
                continue;
            };
            gifts.push(Gift {
                care: Care::Z,
                mode: Some(Mode::Add),
                ever,
                slots: HashSet::new(),
                path: PathIdx::new(parent),
            });
        }

        visited_why.clear();
        visited_zed.clear();
        // self.store.current().debug(&txn);
        // self.store.ancestry().debug(&txn);

        // THEN, visit all paths in order, creating %y and %z update set.
        for idx in order {
            let ancestry: Vec<PathIdx> = self.store.ancestry().title(&mut txn, idx)?;
            for descendant in &ancestry {
                // Required to ignore immediate parent as it will be bumped during
                // iteration of visited_why
                if !changed.contains(descendant.raw()) {
                    visited_zed.insert(descendant.raw());
                }
            }

            // XX, kinda weird that it's last
            if let Some(parent) = ancestry.last().cloned() {
                // if we wrote to it, case is already bumped ignore

                if !changed.contains(parent.raw()) && !visited_zed.contains(parent.raw()) {
                    visited_why.insert(parent.raw());
                }
            }
        }

        for idx in visited_why {
            let ever = self.store.bump_why(&mut txn, PathIdx::new(idx), false)?;
            gifts.push(Gift {
                care: Care::Y,
                mode: Some(Mode::Dif),
                ever,
                slots: HashSet::new(),
                path: PathIdx::new(idx),
            });
            let mode = if created.contains(idx) {
                Some(Mode::Add)
            } else if changed.contains(idx) {
                Some(Mode::Dif)
            } else if deleted.contains(idx) {
                Some(Mode::Del)
            } else {
                None
            };
            gifts.push(Gift {
                care: Care::Y,
                mode,
                ever,
                slots: HashSet::new(),
                path: PathIdx::new(idx),
            });
        }
        for idx in visited_zed {
            let ever = self.store.bump_zed(&mut txn, PathIdx::new(idx), true)?;
            gifts.push(Gift {
                care: Care::Z,
                mode: Some(Mode::Dif),
                ever,
                slots: HashSet::new(),
                path: PathIdx::new(idx),
            });
            let mode = if created.contains(idx) {
                Some(Mode::Add)
            } else if changed.contains(idx) {
                Some(Mode::Dif)
            } else if deleted.contains(idx) {
                Some(Mode::Del)
            } else {
                None
            };
            gifts.push(Gift {
                care: Care::Z,
                mode,
                ever,
                slots: HashSet::new(),
                path: PathIdx::new(idx),
            });
        }
        // self.store.current().debug(&txn);
        // self.store.ancestry().debug(&txn);
        // self.store.exe().debug(&txn);

        txn.commit()?;

        self.store.env().clear_stale_readers()?;
        Ok(gifts)
    }

    pub fn look_fallback(
        &self,
        txn: &heed::RoTxn,
        care: Care,
        pax: &str,
    ) -> Result<Observe<DiskEpic>> {
        let segments = path_to_segments(pax);

        let Some(mut idx) = self.store.ancestry().resolve_path(&txn, "/")? else {
            return Ok(Observe::Null);
        };
        let mut search = vec![];

        for seg in &segments {
            search.push(seg.to_owned());
            let attempt = format!("/{}", search.join("/").as_str());
            if let Some(new_idx) = self.store.ancestry().resolve_path(&txn, &attempt)? {
                idx = new_idx;
            }
        }
        let res = self.store.look(&txn, care, idx)?;
        match res {
            Observe::Found(mut epic) => {
                strip_prefix_disk_epic(&mut epic, pax, &txn, self);
                Ok(Observe::Found(epic))
            }
            r => Ok(r),
        }
        // drop(txn);
    }

    #[tracing::instrument(skip_all)]
    pub fn look(&self, care: Care, path: &str) -> Result<Observe<DiskEpic>> {
        let mut txn = self.store.env().read_txn()?;
        let idx = self.store.ancestry().resolve_path(&mut txn, path)?;
        let _empty = DiskEpic::new();
        let Some(idx) = idx else {
            tracing::warn!("path not found: {path}");
            // look should always
            return self.look_fallback(&txn, care, path);
        };
        match self.store.look(&mut txn, care, idx)? {
            Observe::Unknown => {
                let epic = DiskEpic::new();
                Ok(Observe::Found(epic))
            }
            res => Ok(res),
        }
    }

    pub fn grab(&self, pax: &str) -> Result<Option<DiskSaga>> {
        let mut txn = self.store.env().read_txn()?;
        let Some(idx) = self.store.ancestry().resolve_path(&mut txn, pax)? else {
            return Ok(None);
        };
        let Some(ever) = self.store.current().get(&txn, idx.raw())? else {
            return Ok(None);
        };
        let Some(mut fil) = self.store.exe().get(&txn, PathKey(idx, ever.x.data))? else {
            return Ok(None);
        };

        fil.ever = ever;

        return Ok(Some(fil));
    }

    #[tracing::instrument(skip_all)]
    pub fn peek(&self, care: Care, path: &str, case: Case) -> Result<Observe<DiskEpic>> {
        let mut txn = self.store.env().read_txn()?;
        let idx = self.store.ancestry().resolve_path(&mut txn, path)?;
        let Some(idx) = idx else {
            return Ok(Observe::Unknown);
        };
        let epic = self.store.peek(&mut txn, case, care, idx)?;
        Ok(epic)
    }

    pub fn peek_item(&self, pax: &str, case: Case) -> Result<Option<DiskSaga>> {
        let mut txn = self.store.env().read_txn()?;
        let Some(idx) = self.store.ancestry().resolve_path(&mut txn, pax)? else {
            return Ok(None);
        };

        match self.store.peek_saga(&txn, case, idx)? {
            Observe::Found(saga) => Ok(Some(saga)),
            _ => Ok(None),
        }
    }

    pub fn peek_tray(&self, pax: &str, case: Case) -> Result<Option<(Tray, Noun)>> {
        let _txn = self.store.env().read_txn()?;
        let Some(item) = self.peek_item(pax, case)? else {
            return Ok(None);
        };

        Ok(item.tray())
    }

    pub fn path_idx_to_str(&self, idx: &PathIdx) -> Result<Option<String>> {
        let mut txn = self.store.env().read_txn()?;
        let str = self.store.ancestry().path_string(&mut txn, *idx)?;
        Ok(str)
    }
    pub fn path_idx_to_child_str(&self, idx: &PathIdx, parent: &str) -> Result<Option<String>> {
        let mut txn = self.store.env().read_txn()?;
        let str = self.store.ancestry().child_string(&mut txn, *idx, parent)?;
        Ok(str)
    }

    pub fn ancestors_of(&self, path: PathIdx) -> Result<Vec<PathIdx>> {
        let txn = self.store.env().read_txn()?;
        let title = self.store.ancestry().title_ro(&txn, path)?;
        Ok(title)
    }

    /// Iterate all current sagas and return those containing the named slot.
    pub fn scan_slot(&self, slot_name: &str) -> Result<Vec<(PathIdx, DiskPail)>> {
        let txn = self.store.env().read_txn()?;
        let mut results = Vec::new();
        let iter = self.store.current().iter(&txn)?;
        for item in iter {
            let (path_id, ever) = item?;
            let idx = PathIdx::new(path_id);
            let key = crate::store::lmdb::key::PathKey(idx, ever.x.data);
            if let Some(saga) = self.store.exe().get(&txn, key)? {
                if let Some(pail) = saga.tale().get(slot_name) {
                    results.push((idx, pail.clone()));
                }
            }
        }
        Ok(results)
    }

    pub fn get_path_id(&self, path: &str) -> Result<PathIdx> {
        let mut txn = self.store.env().write_txn()?;
        let idx = self.store.ancestry().upsert_path(&mut txn, path)?;
        Ok(idx)
    }

    /// Enumerate all relationship constraint violations in the namespace.
    ///
    /// Checks (derived from spec layers 4, 6, 7, 8):
    ///
    /// - `VERSION_ORDER`: z.data >= y.data >= x.data
    /// - `X_SAGA_MISSING`: no saga at current x-case (may be tombstone from cull)
    /// - `Y_FAMILY_MISSING` / `Z_FAMILY_MISSING`: no family at current y/z-case
    /// - `Y_PARENT_MISMATCH` / `Z_PARENT_MISMATCH`: family.parent != current Ever
    /// - `Y_CHILD_SAGA_MISSING` / `Z_CHILD_SAGA_MISSING`: child ref -> no saga
    /// - `Y_CHILD_STALE` / `Z_CHILD_STALE`: child stored x-case != child current x-case
    /// - `Y_CHILD_NO_CURRENT` / `Z_CHILD_NO_CURRENT`: child not in current db
    /// - `Y_INCOMPLETE`: live direct child missing from y-family
    /// - `Z_INCOMPLETE`: live descendant missing from z-family
    pub fn check_invariants(&self) -> Result<Vec<String>> {
        let txn = self.store.env().read_txn()?;
        let mut violations = Vec::new();

        // Collect all current entries
        let mut current_map: HashMap<u32, Ever> = HashMap::new();
        for item in self.store.current().iter(&txn)? {
            let (id, ever) = item?;
            current_map.insert(id, ever);
        }

        // Resolve path idx to string for readable output
        let resolve = |id: u32| -> String {
            self.store
                .ancestry()
                .path_string(&txn, PathIdx::new(id))
                .ok()
                .flatten()
                .unwrap_or_else(|| format!("?{}", id))
        };

        // Precompute set of live nodes (have saga at current x-case)
        let live_nodes: HashSet<u32> = current_map
            .iter()
            .filter(|(id, ever)| {
                ever.x.data > 0
                    && self
                        .store
                        .exe()
                        .get(&txn, PathKey(PathIdx::new(**id), ever.x.data))
                        .ok()
                        .flatten()
                        .is_some()
            })
            .map(|(id, _)| *id)
            .collect();

        for (&id, ever) in &current_map {
            let idx = PathIdx::new(id);
            let name = resolve(id);

            // --- Version ordering: z >= y >= x ---
            if ever.z.data < ever.y.data || ever.y.data < ever.x.data {
                violations.push(format!(
                    "VERSION_ORDER: {name} (idx={id}) x={} y={} z={} — expected z >= y >= x",
                    ever.x.data, ever.y.data, ever.z.data
                ));
            }

            // --- X-saga existence ---
            if ever.x.data > 0 && !live_nodes.contains(&id) {
                violations.push(format!(
                    "X_SAGA_MISSING: {name} (idx={id}) x.data={} — no saga (may be tombstone)",
                    ever.x.data
                ));
            }

            // --- Y-family ---
            if ever.y.data > 0 {
                match self.store.why().get(&txn, PathKey(idx, ever.y.data))? {
                    None => {
                        violations.push(format!(
                            "Y_FAMILY_MISSING: {name} (idx={id}) y.data={}",
                            ever.y.data
                        ));
                    }
                    Some(family) => {
                        // y-parent records Ever at time of last y-bump.
                        // z can advance independently via bump_zed, so only x and y must match.
                        // z must not be ahead of current (but can lag).
                        if family.parent.x != ever.x || family.parent.y != ever.y {
                            violations.push(format!(
                "Y_PARENT_MISMATCH: {name} (idx={id})\n  current:  {:?}\n  y-parent: {:?}",
                ever, family.parent
              ));
                        }
                        if family.parent.z.data > ever.z.data {
                            violations.push(format!(
                "Y_PARENT_Z_AHEAD: {name} (idx={id}) y-parent z={:?} > current z={:?}",
                family.parent.z, ever.z
              ));
                        }
                        for child in &family.children {
                            let cn = resolve(child.0.raw());
                            if self.store.exe().get(&txn, *child)?.is_none() {
                                violations.push(format!(
                                    "Y_CHILD_SAGA_MISSING: {name} -> {cn} at x={}",
                                    child.1
                                ));
                            }
                            if let Some(child_ever) = current_map.get(&child.0.raw()) {
                                if child.1 != child_ever.x.data {
                                    violations.push(format!(
                                        "Y_CHILD_STALE: {name} -> {cn} stored_x={} current_x={}",
                                        child.1, child_ever.x.data
                                    ));
                                }
                            } else {
                                violations.push(format!(
                                    "Y_CHILD_NO_CURRENT: {name} -> {cn} (idx={}) not in current db",
                                    child.0.raw()
                                ));
                            }
                        }
                    }
                }
            }

            // --- Z-family ---
            if ever.z.data > 0 {
                match self.store.zed().get(&txn, PathKey(idx, ever.z.data))? {
                    None => {
                        violations.push(format!(
                            "Z_FAMILY_MISSING: {name} (idx={id}) z.data={}",
                            ever.z.data
                        ));
                    }
                    Some(family) => {
                        if family.parent != *ever {
                            violations.push(format!(
                "Z_PARENT_MISMATCH: {name} (idx={id})\n  current:  {:?}\n  z-parent: {:?}",
                ever, family.parent
              ));
                        }
                        for child in &family.children {
                            let cn = resolve(child.0.raw());
                            if self.store.exe().get(&txn, *child)?.is_none() {
                                violations.push(format!(
                                    "Z_CHILD_SAGA_MISSING: {name} -> {cn} at x={}",
                                    child.1
                                ));
                            }
                            if let Some(child_ever) = current_map.get(&child.0.raw()) {
                                if child.1 != child_ever.x.data {
                                    violations.push(format!(
                                        "Z_CHILD_STALE: {name} -> {cn} stored_x={} current_x={}",
                                        child.1, child_ever.x.data
                                    ));
                                }
                            } else {
                                violations.push(format!(
                                    "Z_CHILD_NO_CURRENT: {name} -> {cn} (idx={}) not in current db",
                                    child.0.raw()
                                ));
                            }
                        }
                    }
                }
            }

            // --- Completeness checks (requires ancestry) ---
            if let Some(ancestry) = self.store.ancestry().by_id().get(&txn, &id)? {
                // Y-completeness: each live direct child should be in y-family
                if ever.y.data > 0 {
                    if let Some(y_family) = self.store.why().get(&txn, PathKey(idx, ever.y.data))? {
                        let y_ids: HashSet<u32> =
                            y_family.children.iter().map(|pk| pk.0.raw()).collect();
                        for ac in &ancestry.children {
                            if live_nodes.contains(&ac.raw()) && !y_ids.contains(&ac.raw()) {
                                let cn = resolve(ac.raw());
                                violations.push(format!(
                                    "Y_INCOMPLETE: {name} missing live child {cn} (idx={})",
                                    ac.raw()
                                ));
                            }
                        }
                    }
                }

                // Z-completeness: all live descendants should be in z-family
                if ever.z.data > 0 {
                    if let Some(z_family) = self.store.zed().get(&txn, PathKey(idx, ever.z.data))? {
                        let z_ids: HashSet<u32> =
                            z_family.children.iter().map(|pk| pk.0.raw()).collect();
                        let mut stack: Vec<PathIdx> = ancestry.children.clone();
                        while let Some(desc) = stack.pop() {
                            if live_nodes.contains(&desc.raw()) && !z_ids.contains(&desc.raw()) {
                                let dn = resolve(desc.raw());
                                violations.push(format!(
                                    "Z_INCOMPLETE: {name} missing live descendant {dn} (idx={})",
                                    desc.raw()
                                ));
                            }
                            if let Some(desc_anc) =
                                self.store.ancestry().by_id().get(&txn, &desc.raw())?
                            {
                                stack.extend(desc_anc.children.iter().cloned());
                            }
                        }
                    }
                }
            }
        }

        Ok(violations)
    }
}

impl Epic {
    pub fn to_noun(&self, ns: &Namespace) -> StdResult<Noun, NounConversionError> {
        let mut map = Noun::null();
        for (key, saga) in self.0.iter() {
            let key = path_noun_from_string(key);
            let saga = saga.to_noun(ns).unwrap();
            map = jets::axal::insert(map, key, saga);
        }
        Ok(map)
    }
}

impl Observe<Epic> {
    pub fn to_noun(&self, ns: &Namespace) -> StdResult<Noun, NounConversionError> {
        match self {
            Observe::Found(epic) => {
                let epic_noun = epic.to_noun(ns)?;
                Ok(epic_noun.into_some())
            }
            Observe::Null => {
                let epic_noun = jets::axal::empty();
                Ok(epic_noun.into_some())
            }
            Observe::Unknown => Ok(Noun::null()),
        }
    }
}
