use std::borrow::Cow;

use bytemuck::checked::pod_read_unaligned;
use tracing::info;

use crate::{core::{path::PathIdx, types::Ever}, store::lmdb::{LmdbError, Result, key::{PathKey, PathKeyCodec}}};
use std::result::Result as StdResult;


pub struct ChildrenDb {
  db: heed::Database<PathKeyCodec, FamilyCodec>,
}

impl ChildrenDb {
  pub fn create(env: &heed::Env, name: &str) -> Result<Self> {
    let mut wtxn = env.write_txn()?;
    let db = env.create_database(&mut wtxn, Some(name))?;
    wtxn.commit()?;
    Ok(Self { db })
  }

  pub fn open(env: &heed::Env, name: &str) -> Result<Self> {
    let mut wtxn = env.write_txn()?;
    let db = env.open_database(&mut wtxn, Some(name))?.expect(format!("{} database not found", name).as_str());
    wtxn.commit()?;
    Ok(Self { db })
  }

  pub fn put(&self, wtxn: &mut heed::RwTxn, key: PathKey, data: &Family) -> Result<()> {
    self.db.put(wtxn, &key, data)?;
    Ok(())
  }

  pub fn debug(&self, txn: &heed::RoTxn) {
    let iter = self.db.iter(txn).unwrap();
    println!("=====CHILDREN ==============");
    for item in iter {
      let Ok((key, value)) = item else {
        println!("error iterating children {item:?}");
        continue;
      };
      println!("key: {:?}, value: {:?}", key, value);
    }
    println!("=====END CHILDREN ==============");
  }

  pub fn get(&self, txn: &heed::RoTxn, key: PathKey) -> Result<Option<Family>> {
    let res =  self.db.get(txn, &key)?;
    return Ok(res);
  }

  pub fn scan_versions(&self, txn: &heed::RoTxn, key: PathIdx) -> Result<()> {
    let key = PathKey(key, 0);
    let iter = self.db.prefix_iter(txn, &key)?;
    for item in iter {
      let Ok((key, family)) = item else {
        let e = item.unwrap_err();
        tracing::error!("invalid item in iterator {e:?}");
        continue;
      };
      info!("item {key:?} family {family:?}");
    }

    Ok(())

  }

}

pub struct FamilyCodec { }

impl<'a> heed::BytesEncode<'a> for FamilyCodec {
  type EItem = Family;

  fn bytes_encode(family: &Family) -> StdResult<Cow<'_, [u8]>, Box<dyn std::error::Error + Send + Sync>> {
    let bytes = family.to_bytes();
    Ok(Cow::Owned(bytes))
  }
}

impl<'a> heed::BytesDecode<'a> for FamilyCodec {
  type DItem = Family;

  fn bytes_decode(bytes: &'a [u8]) -> StdResult<Family, Box<dyn std::error::Error + Send + Sync>> {
    let family = Family::from_bytes(bytes).map_err(|e| Box::new(e))?;
    Ok(family)
  }
} 

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[repr(C)]
pub struct Family {
  pub parent: Ever,
  pub children: Vec<PathKey>,
}

impl Family {
  pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
    let parent_size = std::mem::size_of::<Ever>();
    let parent = pod_read_unaligned::<Ever>(&bytes[0..parent_size]);
    let children_byte_size = usize::from_le_bytes(bytes[parent_size..parent_size + 8].try_into().unwrap());
    assert_eq!(children_byte_size % 8, 0);
    let children_start = parent_size + 8;
    let children_size = children_byte_size / 8;
    let mut children = Vec::with_capacity(children_size);
    for i in 0..children_size {
      let start = children_start + (i * 8);
      let key = PathKey::from_bytes(&bytes[start..start + 8]).map_err(|_e| LmdbError::InvalidKeyLength(bytes.len()))?;
      children.push(key);
    }
    Ok(Self { parent, children })
  }

  pub fn to_bytes(&self) -> Vec<u8> {
    let total_size = std::mem::size_of::<Ever>() + 8 + self.children.len() * 4;
    let mut bytes = Vec::with_capacity(total_size);
    bytes.extend_from_slice(bytemuck::bytes_of::<Ever>(&self.parent));
    let children_len = self.children.len();
    bytes.extend_from_slice(&(children_len * 8).to_le_bytes());
    for key in &self.children {
      bytes.extend_from_slice(&key.to_bytes());
    }
    bytes
  }
}

impl AsRef<[u8]> for Family {
  fn as_ref(&self) -> &[u8] {
    let len = std::mem::size_of::<Self>();
    let slice = unsafe {
      std::slice::from_raw_parts(self as *const Self as *const u8, len)
    };
    slice
  }
}




#[cfg(test)]
mod tests {
  use std::path::Path;

use heed::EnvOpenOptions;

use crate::core::types::Span;

use super::*;

fn setup_env() -> Result<heed::Env> {
  // At compile time (most common)
  let manifest_dir = env!("CARGO_MANIFEST_DIR");
  let db_path = Path::new(manifest_dir).join("test-family");
  println!("db_path: {:?}", db_path);
  std::fs::remove_dir_all(&db_path).unwrap_or_default();
  std::fs::create_dir_all(&db_path).unwrap();
  let env = unsafe {
    EnvOpenOptions::new()
        .map_size(10 * 1024 * 1024)
        .max_dbs(10)
        .open(&db_path)?
};
  Ok(env)
}

#[test]
  fn test_roundtrip() -> Result<()> {
    let span = Span::new();
    let ever = Ever::new(&span);
    let family = Family { parent: ever.clone(), children: vec![PathKey(PathIdx::new(0), 1)] };
    let env = setup_env()?;
    let mut children_db = ChildrenDb::create(&env, "children").expect("Failed to create children database");
    let mut txn = env.write_txn().expect("Failed to get write transaction");
    let path_idx = PathIdx::new(0);
    let key = PathKey(path_idx, 1);
    children_db.put(&mut txn, key, &family).expect("failed to put family");
    txn.commit().expect("Failed to commit transaction");
    let txn = env.read_txn()?;
    let family_two = children_db.get(&txn, key).expect("failed to get family").expect("family not found");
    assert_eq!(family, family_two);
    Ok(())
  }
  
}

