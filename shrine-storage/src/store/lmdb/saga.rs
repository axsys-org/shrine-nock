use std::borrow::Cow;

use bytemuck::checked::pod_read_unaligned;
use milton::{Noun, cell, cue, traits::IntoNoun};

use crate::{Aeon, DiskPail, Saga, Tale, core::{path::PathIdx, types::{Ever, Oath}}, overlay::Tray, slot, store::{Namespace, lmdb::{Result, StoreCtx, key::{PathKey, PathKeyCodec}}}};
use std::result::Result as StdResult;
use std::collections::HashMap;


pub struct SagaDb {
  db: heed::Database<PathKeyCodec, SagaCodec>,
}

impl SagaDb {
  pub fn new(env: &heed::Env) -> Result<Self> {
    let mut wtxn = env.write_txn()?;
    let db = env.create_database(&mut wtxn, Some("saga"))?;
    wtxn.commit()?;
    Ok(Self { db })
  }

  pub fn open(env: &heed::Env) -> Result<Self> {
    let mut wtxn = env.write_txn()?;
    let db = env.open_database(&mut wtxn, Some("saga"))?.expect("saga database not found");
    wtxn.commit()?;
    Ok(Self { db })
  }

  pub fn put(&self, wtxn: &mut heed::RwTxn, key: PathKey, data: &DiskSaga) -> Result<()> {
    self.db.put(wtxn, &key, data)?;
    Ok(())
  }

  pub fn get(&self, txn: &heed::RoTxn, key: PathKey) -> Result<Option<DiskSaga>> {
    let res = self.db.get(txn, &key)?;
    Ok(res)
  }

  pub fn debug(&self, txn: &heed::RoTxn) {
    let iter = self.db.iter(txn).unwrap();
    println!("=====SAGA ==============");
    for item in iter {
      let Ok((key, value)) = item else {
        println!("error iterating saga {item:?}");
        continue;
      };
      println!("key: {:?}, value: {:?}", key, value);
    }
    println!("=====END SAGA ==============");
  }
}

pub struct SagaCodec { }

impl heed::BytesEncode<'_> for SagaCodec {
  type EItem = DiskSaga;

  fn bytes_encode(saga: &DiskSaga) -> StdResult<Cow<'_, [u8]>, Box<dyn std::error::Error + Send + Sync>> {
    let bytes = saga.to_bytes();
    Ok(Cow::Owned(bytes))
  }
}

impl heed::BytesDecode<'_> for SagaCodec {
  type DItem = DiskSaga;

  fn bytes_decode(bytes: &[u8]) -> StdResult<DiskSaga, Box<dyn std::error::Error + Send + Sync>> {
    let saga = DiskSaga::from_bytes(bytes).map_err(Box::new)?;
    Ok(saga)
  }
}

// Binary serialization tag constants for DiskPail variants
const DISK_PAIL_TAG_JAM: u8 = 0;
const DISK_PAIL_TAG_TEXT: u8 = 1;
const DISK_PAIL_TAG_HOON: u8 = 2;
const DISK_PAIL_TAG_ATOM: u8 = 3;
const DISK_PAIL_TAG_MESH: u8 = 4;
const DISK_PAIL_TAG_DUCT: u8 = 5;
const DISK_PAIL_TAG_WAIN: u8 = 6;
const DISK_PAIL_TAG_JIM: u8 = 7;

fn disk_pail_tag(pail: &DiskPail) -> u8 {
    match pail {
        DiskPail::Jam { .. } => DISK_PAIL_TAG_JAM,
        DiskPail::Text { .. } => DISK_PAIL_TAG_TEXT,
        DiskPail::Hoon { .. } => DISK_PAIL_TAG_HOON,
        DiskPail::Atom { .. } => DISK_PAIL_TAG_ATOM,
        DiskPail::Mesh { .. } => DISK_PAIL_TAG_MESH,
        DiskPail::Duct { .. } => DISK_PAIL_TAG_DUCT,
        DiskPail::Wain { .. } => DISK_PAIL_TAG_WAIN,
        DiskPail::Jim { .. } => DISK_PAIL_TAG_JIM
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiskSaga {
  pub ever: Ever,
  oath: Oath,
  tale: Tale,
}

impl DiskSaga {
  pub fn from_raw(ever: &Ever, tale: Tale) -> Result<Self> {
    let res = Self { ever: ever.clone(), oath: Oath::new(), tale };
    Ok(res)
  }
  pub fn new(ctx: &StoreCtx, ever: &Ever, tale: &Tale) -> Result<Self> {
    let mut res = Self { ever: ever.clone(), oath: Oath::new(), tale: tale.clone() };
    res.oath.sign(ctx, &[]);
    Ok(res)
  }

  pub fn ever(&self) -> &Ever {
    &self.ever
  }

  pub fn tray(&self) -> Option<(Tray, Noun)> {
    let vase = self.tale.get_slot(slot::VASE)?;
    match vase {
      DiskPail::Jam { typ, data } => {
        let tray = Tray::from_string(&typ)?;
        return cue(&data).map(|n| (tray, n))
      }
      _ => {
        return None;
      }
    }
  }

  pub fn tale(&self) -> &Tale {
    &self.tale
  }

  pub fn slots(&self) -> &[String] {
    self.tale.slots()
  }

  pub fn pails(&self) -> &[DiskPail] {
    self.tale.pails()
  }

  pub fn merge_tale(&mut self, ctx: &StoreCtx, tale: &Tale) -> Result<()> {
    for (key, pail) in tale.iter() {
      self.tale.insert(key.clone(), pail.clone());
    }
    self.ever.bump_x(ctx, false);
    self.oath.sign(ctx, &[]);
    Ok(())
  }

  pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
    let ever_size = std::mem::size_of::<Ever>();
    let ever = pod_read_unaligned::<Ever>(&bytes[0..ever_size]);
    let oath_size = std::mem::size_of::<Oath>();
    let oath = pod_read_unaligned::<Oath>(&bytes[ever_size..ever_size + oath_size]);

    let mut pos = ever_size + oath_size;
    let slot_count = u32::from_le_bytes(bytes[pos..pos+4].try_into().unwrap()) as usize;
    pos += 4;

    let mut slots = Vec::with_capacity(slot_count);
    let mut pails = Vec::with_capacity(slot_count);

    for _ in 0..slot_count {
      let name_len = u32::from_le_bytes(bytes[pos..pos+4].try_into().unwrap()) as usize;
      pos += 4;
      let name = std::str::from_utf8(&bytes[pos..pos+name_len]).unwrap().to_string();
      pos += name_len;

      let tag = bytes[pos];
      pos += 1;

      slots.push(name);
      match tag {
        DISK_PAIL_TAG_JIM => {
          let typ_len = u32::from_le_bytes(bytes[pos..pos+4].try_into().unwrap()) as usize;
          pos += 4;
          let typ = std::str::from_utf8(&bytes[pos..pos+typ_len]).unwrap().to_string();
          pos += typ_len;

          let data_len = u32::from_le_bytes(bytes[pos..pos+4].try_into().unwrap()) as usize;
          pos += 4;
          let data = bytes[pos..pos+data_len].to_vec();
          pos += data_len;

          pails.push(DiskPail::Jim { typ, data });
        }
        DISK_PAIL_TAG_JAM => {
          let typ_len = u32::from_le_bytes(bytes[pos..pos+4].try_into().unwrap()) as usize;
          pos += 4;
          let typ = std::str::from_utf8(&bytes[pos..pos+typ_len]).unwrap().to_string();
          pos += typ_len;

          let data_len = u32::from_le_bytes(bytes[pos..pos+4].try_into().unwrap()) as usize;
          pos += 4;
          let data = bytes[pos..pos+data_len].to_vec();
          pos += data_len;

          pails.push(DiskPail::Jam { typ, data });
        }
        DISK_PAIL_TAG_TEXT | DISK_PAIL_TAG_HOON | DISK_PAIL_TAG_ATOM => {
          let data_len = u32::from_le_bytes(bytes[pos..pos+4].try_into().unwrap()) as usize;
          pos += 4;
          let data = bytes[pos..pos+data_len].to_vec();
          pos += data_len;

          pails.push(match tag {
            DISK_PAIL_TAG_TEXT => DiskPail::Text { data },
            DISK_PAIL_TAG_HOON => DiskPail::Hoon { data },
            DISK_PAIL_TAG_ATOM => DiskPail::Atom { data },
            _ => unreachable!(),
          });
        }
        DISK_PAIL_TAG_MESH => {
          let data_len = u32::from_le_bytes(bytes[pos..pos+4].try_into().unwrap()) as usize;
          pos += 4;
          let raw = &bytes[pos..pos+data_len];
          pos += data_len;
          pails.push(DiskPail::Mesh { data: DiskPail::mesh_from_bytes(raw) });
        }
        DISK_PAIL_TAG_DUCT => {
          let data_len = u32::from_le_bytes(bytes[pos..pos+4].try_into().unwrap()) as usize;
          pos += 4;
          let raw = &bytes[pos..pos+data_len];
          pos += data_len;
          pails.push(DiskPail::Duct { data: bytemuck::cast_slice(raw).to_vec() });
        }
        DISK_PAIL_TAG_WAIN => {
          let wain_count = u32::from_le_bytes(bytes[pos..pos+4].try_into().unwrap());
          pos += 4;
          let mut res = Vec::with_capacity(wain_count as usize);
          for _ in 0..wain_count {
            let str_size = u32::from_le_bytes(bytes[pos..pos+4].try_into().unwrap()) as usize;
            pos += 4;
            let next = String::from_utf8(bytes[pos..pos+str_size].to_vec()).unwrap();
            pos += str_size;
            res.push(next);
          }
          pails.push(DiskPail::Wain { data: res });
        }

        _ => panic!("unknown DiskPail tag: {}", tag),
      }
    }

    Ok(Self { ever, oath, tale: Tale::new(slots, pails) })
  }

  pub fn to_bytes(&self) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(bytemuck::bytes_of::<Ever>(&self.ever));
    bytes.extend_from_slice(bytemuck::bytes_of::<Oath>(&self.oath));

    let slots = self.tale.slots();
    let pails = self.tale.pails();
    bytes.extend_from_slice(&(slots.len() as u32).to_le_bytes());

    for (slot, pail) in slots.iter().zip(pails.iter()) {
      bytes.extend_from_slice(&(slot.len() as u32).to_le_bytes());
      bytes.extend_from_slice(slot.as_bytes());

      bytes.push(disk_pail_tag(pail));

      match pail {
        DiskPail::Jim { typ, data } => {
          bytes.extend_from_slice(&(typ.len() as u32).to_le_bytes());
          bytes.extend_from_slice(typ.as_bytes());
          bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
          bytes.extend_from_slice(data);
        }
        DiskPail::Jam { typ, data } => {
          bytes.extend_from_slice(&(typ.len() as u32).to_le_bytes());
          bytes.extend_from_slice(typ.as_bytes());
          bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
          bytes.extend_from_slice(data);
        }
        DiskPail::Mesh { data } => {
          let mesh_bytes = DiskPail::mesh_to_bytes(data);
          bytes.extend_from_slice(&(mesh_bytes.len() as u32).to_le_bytes());
          bytes.extend_from_slice(&mesh_bytes);
        }
        DiskPail::Wain { data } => {
          let wain_count = data.len() as u32;
          bytes.extend_from_slice(&wain_count.to_le_bytes());
          for line in data {
            let str_size = line.len() as u32;
            bytes.extend_from_slice(&str_size.to_le_bytes());
            bytes.extend_from_slice(line.as_bytes());
          }
        }
        other => {
          let data = other.data();
          bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
          bytes.extend_from_slice(data);
        }
      }
    }

    bytes
  }

  pub fn to_saga(&self) -> Saga {
    Saga {
      aeon: Aeon {
        oath: self.oath.clone(),
        ever: self.ever.clone(),
      },
      tale: self.tale.clone(),
    }
  }

  pub fn aeon(&self) -> Aeon {
    Aeon {
      oath: self.oath.clone(),
      ever: self.ever.clone(),
    }
  }

  pub fn to_noun(&self, ns: &Namespace) -> Noun {
    let saga = self.to_saga();
    let tale = saga.tale.to_noun(ns).unwrap();
    let aeon = saga.aeon.into_noun().unwrap();
    cell(aeon, tale).into_noun()
  }
}

pub type DiskEpic = HashMap<PathIdx, DiskSaga>;

/// XX: total dogshit, fix
pub fn strip_prefix_disk_epic(epic: &mut DiskEpic, pfix: &str, txn: &heed::RoTxn, namespace: &Namespace) {
  let mut to_remove = vec![];
  for (k, _v) in epic.iter() {
    let str = namespace.store().ancestry().path_string(&txn, *k).unwrap().unwrap();
    if !str.starts_with(pfix) {
      to_remove.push(*k);
    }
  }
  for k in to_remove {
    epic.remove(&k);
  }
}

#[cfg(test)]
mod tests {
  use std::path::Path;

  use heed::EnvOpenOptions;

  use crate::core::{path::PathIdx, types::Span};

  use super::*;

  fn setup_env() -> Result<heed::Env> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let db_path = Path::new(manifest_dir).join("test-saga");
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
    let tale = Tale::new(
      vec!["/content".to_string()],
      vec![DiskPail::Jam { typ: "text".to_string(), data: vec![0x1, 0x2, 0x3] }],
    );
    let saga = DiskSaga {
      ever,
      oath: Oath::new(),
      tale,
    };
    let env = setup_env()?;
    let saga_db = SagaDb::new(&env).expect("Failed to create saga database");
    let mut txn = env.write_txn().expect("Failed to get write transaction");
    let path_idx = PathIdx::new(0);
    let key = PathKey(path_idx, 1);
    saga_db.put(&mut txn, key, &saga).expect("failed to put saga");
    txn.commit().expect("Failed to commit transaction");
    let txn = env.read_txn()?;
    let saga_two = saga_db.get(&txn, key).expect("failed to get saga").expect("saga not found");
    assert_eq!(saga, saga_two);
    Ok(())
  }

}
