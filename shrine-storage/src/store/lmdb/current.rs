use std::borrow::Cow;

use heed::{byteorder::BigEndian, types::U32};

use crate::{core::types::Ever, store::lmdb::Result};
use std::result::Result as StdResult;




pub struct EverCodec { }

impl heed::BytesEncode<'_> for EverCodec {
  type EItem = Ever;

  fn bytes_encode(ever: &Ever) -> StdResult<Cow<'_, [u8]>, Box<dyn std::error::Error + Send + Sync>> {
    let bytes = ever.to_bytes();
    Ok(Cow::Owned(bytes))
  }
}

impl heed::BytesDecode<'_> for EverCodec {
  type DItem = Ever;

  fn bytes_decode(bytes: &[u8]) -> StdResult<Ever, Box<dyn std::error::Error + Send + Sync>> {
    let ever = Ever::from_bytes(bytes);
    Ok(ever)
  }
}

pub struct CurrentDb {
  db: heed::Database<U32<BigEndian>, EverCodec>,
}

impl CurrentDb {
  pub fn create(env: &heed::Env) -> Result<Self> {
    let mut wtxn = env.write_txn()?;
    let db = env.create_database(&mut wtxn, Some("current"))?;
    wtxn.commit()?;
    Ok(Self { db })
  }

  pub fn open(env: &heed::Env) -> Result<Self> {
    let mut wtxn = env.write_txn()?;
    let db = env.open_database(&mut wtxn, Some("current"))?.expect("current database not found");
    wtxn.commit()?;
    Ok(Self { db })
  }

  pub fn put(&self, wtxn: &mut heed::RwTxn, key: u32, data: &Ever) -> Result<()> {
    self.db.put(wtxn, &key, data)?;
    Ok(())
  }

  pub fn debug(&self, txn: &heed::RoTxn) {
    let iter = self.db.iter(txn).unwrap();
    println!("=====CURRENT ==============");
    for item in iter {
      let Ok((key, value)) = item else {
        println!("error iterating current {item:?}");
        continue;
      };
      println!("key: {:?}, value: {:?}", key, value);
    }
    println!("=====END CURRENT ==============");
  }

  pub fn get(&self, txn: &heed::RoTxn, key: u32) -> Result<Option<Ever>> {
    let res = self.db.get(txn, &key)?;
    Ok(res)
  }

  pub fn iter<'txn>(&self, txn: &'txn heed::RoTxn) -> Result<heed::RoIter<'txn, U32<BigEndian>, EverCodec>> {
    let iter = self.db.iter(txn)?;
    Ok(iter)
  }
}