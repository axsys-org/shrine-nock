use bytemuck::checked::pod_read_unaligned;
use heed::{byteorder::BigEndian, types::U32};
use lmdb::DatabaseFlags;
use std::borrow::Cow;

use crate::{core::{path::{PathIdx, path_to_segments}, types::{Ever, Oath}}, store::lmdb::Result};
use std::result::Result as StdResult;


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ancestry {
  pub depth: u16,
  pub children: Vec<PathIdx>,
  pub title: Vec<PathIdx>,
}

impl Ancestry {
  pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
    let depth = u16::from_le_bytes(bytes[0..2].try_into().unwrap());
    let children_len = u32::from_le_bytes(bytes[2..6].try_into().unwrap());
    let children_start = 6;
    let mut children = Vec::with_capacity(children_len as usize);
    for idx in 0..((children_len as usize) / 4) {
      let start = children_start + (idx * 4);
      let end = start + 4;
      let idx = PathIdx::from_bytes(&bytes[start..end]).unwrap();
      children.push(idx);
    }
    let title_start = children_start + children_len as usize;
    let title_size = bytes.len() - title_start;
    let mut title = Vec::with_capacity((title_size / 4) as usize);
    for idx in 0..(title_size / 4) {
      let start = title_start + (idx * 4);
      let end = start + 4;
      let idx = PathIdx::from_bytes(&bytes[start..end]).unwrap();
      title.push(idx);
    }
    Ok(Self { depth, children, title })
  }

  pub fn to_bytes(&self) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(10 + self.children.len() * 4 + self.title.len() * 4);
    bytes.extend_from_slice(&self.depth.to_le_bytes());
    bytes.extend_from_slice(&((self.children.len() * 4) as u32).to_le_bytes());
    for child in &self.children {
      bytes.extend_from_slice(&child.to_bytes());
    }
    for title in &self.title {
      bytes.extend_from_slice(&title.to_bytes());
    }
    bytes
  }
}

pub struct AncestryCodec { }

impl heed::BytesEncode<'_> for AncestryCodec {
  type EItem = Ancestry;

  fn bytes_encode(ancestry: &Ancestry) -> StdResult<Cow<'_, [u8]>, Box<dyn std::error::Error + Send + Sync>> {
    let bytes = ancestry.to_bytes();
    Ok(Cow::Owned(bytes))
  }
}

impl heed::BytesDecode<'_> for AncestryCodec {
  type DItem = Ancestry;

  fn bytes_decode(bytes: &[u8]) -> StdResult<Ancestry, Box<dyn std::error::Error + Send + Sync>> {
    let ancestry = Ancestry::from_bytes(bytes).map_err(Box::new)?;
    Ok(ancestry)
  }
}

type PathIdxCodec = U32<BigEndian>;

#[derive(Debug, Clone)]
pub struct AncestryDb {
  by_id: heed::Database<PathIdxCodec, AncestryCodec>,
  by_string: heed::Database<heed::types::Str, PathIdxCodec>,
  by_string_reverse: heed::Database<PathIdxCodec, heed::types::Str>,
  flags: DatabaseFlags,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct AncestryRwTxn<'env> {
  txn: lmdb::RwTransaction<'env>,
  by_id: lmdb::Database,
  by_string: lmdb::Database,
}




#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(C)]
pub struct DiskAncestry {
  ever: Ever,
  oath: Oath,
  tale: Vec<u8>,
}

impl DiskAncestry {
  pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
    let ever_size = std::mem::size_of::<Ever>();
    let ever = pod_read_unaligned::<Ever>(&bytes[0..ever_size]);
    let oath_size = std::mem::size_of::<Oath>();
    let oath = pod_read_unaligned::<Oath>(&bytes[ever_size..ever_size + oath_size]);
    let tale_len = bytes.len() - ever_size - oath_size;
    let tale_start = ever_size + oath_size;
    let tale = bytes[tale_start..tale_start + tale_len].to_vec();
    Ok(Self { ever, oath, tale })
  }

  pub fn to_bytes(&self) -> Vec<u8> {
    let total_size = std::mem::size_of::<Ever>() + std::mem::size_of::<Oath>() + self.tale.len();
    let mut bytes = Vec::with_capacity(total_size);
    bytes.extend_from_slice(bytemuck::bytes_of::<Ever>(&self.ever));
    bytes.extend_from_slice(bytemuck::bytes_of::<Oath>(&self.oath));
    bytes.extend_from_slice(&self.tale);
    bytes
  }
}

impl AncestryDb {
  pub fn create(env: &heed::Env, flags: DatabaseFlags) -> Result<Self> {
    let mut wtxn = env.write_txn()?;
    let by_id = env.create_database(&mut wtxn, Some("by_id"))?;
    let by_string = env.create_database(&mut wtxn, Some("by_string"))?;
    let by_string_reverse = env.create_database(&mut wtxn, Some("by_string_reverse"))?;
    wtxn.commit()?;
    Ok(Self {
      by_string,
      by_string_reverse,
      by_id,
      flags,
    })
  }

  pub fn open(env: &heed::Env, flags: DatabaseFlags) -> Result<Self> {
    let mut wtxn = env.write_txn()?;
    let by_id = env.open_database(&mut wtxn, Some("by_id"))?.expect("by_id database not found");
    let by_string = env.open_database(&mut wtxn, Some("by_string"))?.expect("by_string database not found");
    let by_string_reverse = env.open_database(&mut wtxn, Some("by_string_reverse"))?.expect("by_string_reverse database not found");
    wtxn.commit()?;
    Ok(Self { by_id, by_string, by_string_reverse, flags })
  }

  pub fn by_id(&self) -> heed::Database<PathIdxCodec, AncestryCodec> {
    self.by_id
  }

  pub fn by_string(&self) -> heed::Database<heed::types::Str, PathIdxCodec> {
    self.by_string
  }
  

  pub fn flags(&self) -> DatabaseFlags {
    self.flags
  }

  pub fn debug(&self, txn: &heed::RoTxn) {
    let iter = self.by_string.iter(txn).unwrap();
    println!("=====ANCESTRY ==============");
    for item in iter {
      let Ok((key, value)) = item else {
        println!("error iterating by string {item:?}");
        continue;
      };
      println!("key: {:?}, value: {:?}", key, value);
    }
    let iter = self.by_id.iter(txn).unwrap();
    for item in iter {
      let Ok((key, value)) = item else {
        println!("error iterating by id {item:?}");
        continue;
      };
      println!("key: {:?}, value: {:?}", key, value);
    }
    println!("=====END ANCESTRY ==============");

  }

  pub fn title(&self, wtxn: &mut heed::RwTxn, target: PathIdx) -> Result<Vec<PathIdx>> {
    let Some(ancestry) = self.by_id.get(wtxn, &target.raw())? else {
      return Ok(vec![]);
    };
    Ok(ancestry.title.clone())
  }

  pub fn title_ro(&self, rtxn: &heed::RoTxn, target: PathIdx) -> Result<Vec<PathIdx>> {
    let Some(ancestry) = self.by_id.get(rtxn, &target.raw())? else {
      return Ok(vec![]);
    };
    Ok(ancestry.title.clone())
  }

  pub fn descendants(&self, wtxn: &mut heed::RwTxn, parent: PathIdx) -> Result<Vec<PathIdx>> {
    let mut target = parent;
    let mut descendants = vec![];
    while let Some(ancestry) = self.by_id.get(wtxn, &target.raw())? {
      descendants.extend(ancestry.children.iter().cloned());
      let Some(next) = ancestry.title.last() else {
        break;
      };
      target = *next;
    }
    Ok(descendants)
  }


  pub fn add_child(&self, wtxn: &mut heed::RwTxn, parent: PathIdx, child: PathIdx) -> Result<()> {
    let mut old = self.by_id.get(wtxn, &parent.raw())?.expect("parent not found");
    if !old.children.contains(&child) {
      old.children.push(child);
      self.by_id.put(wtxn, &parent.raw(), &old)?;

    }
    Ok(())
  }

  pub fn alloc_path(&self, wtxn: &mut heed::RwTxn, pax: &str) -> Result<PathIdx> {
    let last_id = self.by_id.last(wtxn)?.map(|(key, _)| key).unwrap_or(0);
    let new_id = last_id + 1;

    let mut segments = path_to_segments(pax);
    let mut search = vec![];
    let mut ancestry = Ancestry { depth: segments.len() as u16, children: vec![], title: vec![] };

    let is_root = segments.len() == 0;

    // pop the last segment, we're searching for parents
    let _ = segments.pop();

    // there should always be a /
    if !is_root {
      ancestry.title.push(PathIdx::new(1));
    }

    for segment in segments {
      search.push(segment);
      if let Some(ancestor) = self.by_string.get(wtxn, format!("/{}", search.join("/")).as_str())? {
        ancestry.title.push(PathIdx::new(ancestor));
      } else {
        tracing::info!("ghost ancestor: /{:?}", search.join("/"));
      }
    }

    if let Some(last) = ancestry.title.last() {
      self.add_child(wtxn, *last, PathIdx::new(new_id))?;
    }

    self.by_id.put(wtxn, &new_id, &ancestry)?;
    self.by_string.put(wtxn, &pax, &new_id)?;
    self.by_string_reverse.put(wtxn, &new_id, &pax)?;
    Ok(PathIdx::new(new_id))
  }

  pub fn upsert_path(&self, wtxn: &mut heed::RwTxn, pax: &str) -> Result<PathIdx> {
    if let Some(id) = self.resolve_path(wtxn, pax)? {
      return Ok(id);
    }
    self.alloc_path(wtxn, pax)
  }

  pub fn resolve_path(&self, rtxn: &heed::RoTxn, pax: &str) -> Result<Option<PathIdx>> {
    let id = self.by_string.get(&rtxn, pax)?;
    Ok(id.map(|id| PathIdx::new(id)))
  }

  pub fn path_string(&self, rtxn: &heed::RoTxn, idx: PathIdx) -> Result<Option<String>> {
    let id = self.by_string_reverse.get(rtxn, &idx.raw())?.map(|id| id.to_string());
    Ok(id)
  }

  pub fn child_string(&self, rtxn: &heed::RoTxn, idx: PathIdx, parent: &str) -> Result<Option<String>> {
    let id = self.by_string_reverse.get(rtxn, &idx.raw())?.map(|k| path_trim_parent(parent, k).to_string());
    Ok(id)
  }


}

pub fn path_trim_parent<'a>(parent: &str, pax: &'a str) -> &'a str {
  let len = parent.len();
  let res = &pax[len..];
  println!("trimmed {parent} from {pax}, have {res}");
  return res;
}


#[cfg(test)]
mod tests {
  use std::path::Path;

use heed::Env;

use crate::core::types::Span;

use super::*;

fn setup_env() -> Result<heed::Env> {
    // At compile time (most common)
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let db_path = Path::new(manifest_dir).join("test-parent");
    println!("db_path: {:?}", db_path);
    std::fs::remove_dir_all(&db_path).unwrap_or_default();
    std::fs::create_dir_all(&db_path).unwrap();
    let env = unsafe {
      heed::EnvOpenOptions::new()
          .map_size(10 * 1024 * 1024)
          .max_dbs(10)
          .open(&db_path)?
    };
    Ok(env)
  }

// #[test]
  fn test_roundtrip() -> Result<()> {
    let span = Span::new();
    let ever = Ever::new(&span);
    let _ancestry = DiskAncestry { ever: ever.clone(), oath: Oath::new(), tale: vec![0x1, 0x2, 0x3] };
    let env = setup_env()?;
    let mut wtxn = env.write_txn()?;
    let ancestry_db = AncestryDb::create(&env, DatabaseFlags::empty()).unwrap();
    wtxn.commit()?;
    let mut wtxn = env.write_txn()?;
    let path_idx = ancestry_db.alloc_path(&mut wtxn, "/foo/bar")?;
    let path_idx_two = ancestry_db.alloc_path(&mut wtxn, "/foo/bar/baz")?;
    let path_idx_three = ancestry_db.alloc_path(&mut wtxn, "/foo/bar/baz/qux")?;
    assert_eq!(path_idx, PathIdx::new(1));
    assert_eq!(path_idx_two, PathIdx::new(2));
    assert_eq!(path_idx_three, PathIdx::new(3));
    assert_eq!(ancestry_db.resolve_path(&mut wtxn, "/foo/bar")?, Some(PathIdx::new(1)));
    assert_eq!(ancestry_db.resolve_path(&mut wtxn, "/foo/bar/baz")?, Some(PathIdx::new(2)));
    assert_eq!(ancestry_db.resolve_path(&mut wtxn, "/foo/bar/baz/qux")?, Some(PathIdx::new(3)));
    assert_eq!(ancestry_db.resolve_path(&mut wtxn, "/foo/bar/baz/qux/quux")?, None);
    wtxn.commit()?;
    drop(env);
    Ok(())
  }
  
}

