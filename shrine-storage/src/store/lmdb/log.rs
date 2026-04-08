use std::borrow::Cow;

use crate::store::lmdb::{LmdbError, Result};

use std::result::{Result as StdResult};


#[derive(Debug, Clone, Copy)]
pub enum LogKey {
    Head,
    Epoch(u32)
}

#[derive(Debug, Clone)]
pub struct LogKeyCodec {}

impl<'a> heed::BytesDecode<'a> for LogKeyCodec {
    type DItem = LogKey;
    fn bytes_decode(bytes: &'a [u8]) -> StdResult<Self::DItem, heed::BoxedError> {
        assert!(bytes.len() <= 4);
        let mut byte_sized = [0u8; 4];
        byte_sized.copy_from_slice(bytes);

        let idx = u32::from_be_bytes(byte_sized);
        if idx == 0 {
            return Ok(LogKey::Head)
        } else {
            return Ok(LogKey::Epoch(idx))
        }
    }
}

impl<'a> heed::BytesEncode<'a> for LogKeyCodec {
    type EItem = LogKey;

    fn bytes_encode(item: &'a Self::EItem) -> StdResult<std::borrow::Cow<'a, [u8]>, heed::BoxedError> {
        let idx = match item {
            LogKey::Head => 0,
            LogKey::Epoch(idx) => *idx
        };
        Ok(std::borrow::Cow::Owned(idx.to_be_bytes().to_vec()))
    }
}


#[derive(Debug, Clone)]
#[repr(C)]
pub enum LogEntry {
    Head(u32),
    Entry(Vec<u8>)
}


#[derive(Debug, Clone)]
pub struct LogEntryCodec {}

impl<'a> heed::BytesDecode<'a> for LogEntryCodec {
    type DItem = LogEntry;
    fn bytes_decode(bytes: &'a [u8]) -> StdResult<Self::DItem, heed::BoxedError> {
        let first = bytes[0];
        let res = match first {
            0 => {
                let mut bytes_sized = [0u8; 4];
                bytes_sized.copy_from_slice(&bytes[1..5]);
                
                return Ok(LogEntry::Head(u32::from_be_bytes(bytes_sized)))
            }
            1 => {
                Ok(LogEntry::Entry((&bytes[1..]).to_vec()))
            }
            _ => Err(Box::new(LmdbError::Sane("bad tag type in logdb")))
        }?;
        println!("res {res:?}");
        return Ok(res)

    }
}

impl<'a> heed::BytesEncode<'a> for LogEntryCodec {
    type EItem = LogEntry;

    fn bytes_encode(item: &'a Self::EItem) -> StdResult<std::borrow::Cow<'a, [u8]>, heed::BoxedError> {
        let res = match item {
            LogEntry::Head(count) => {
                let mut v = vec![0u8; 5];
                v[1..5].copy_from_slice(&count.to_be_bytes());
                Ok::<Cow<'a, [u8]>, heed::BoxedError>(Cow::Owned(v))
            },
            LogEntry::Entry(entry) => {
                let mut res = Vec::with_capacity(entry.len() + 1);
                res.push(1);
                res.extend_from_slice(entry);
                Ok(Cow::Owned(res))
            }
        };
        println!("res {res:?}");
        return res;
    }
}

pub struct LogDb {
    db: heed::Database<LogKeyCodec, LogEntryCodec>
}

impl LogDb {
    pub fn new(env: &heed::Env) -> Result<Self> {
        let mut wtxn = env.write_txn()?;
        let db = env.create_database(&mut wtxn, Some("logger"))?;
        let _ = wtxn.commit();
        Ok(Self {
            db
        })
    }

    pub fn open(env: &heed::Env) -> Result<Self> {
        let mut wtxn = env.write_txn()?;
        let Some(db) = env.open_database(&mut wtxn, Some("logger"))? else {
            return Err(LmdbError::Sane("log does not exist"));
        };
        let _ = wtxn.commit();

        Ok(Self {
            db
        })
    }

    // pub fn get(&self, wtxn: &heed::RoTxn) -> Result<u32> {
    //     let key = LogKey::Head;
    //     let res = self.db.get(wtxn, &key)?.map(|r| r.count).unwrap_or(1);
    //     Ok(res)
    // }

    pub fn bump(&self, wtxn: &mut heed::RwTxn) -> Result<u32> {
        let key = LogKey::Head;
        let Some(LogEntry::Head(mut res)) = self.db.get(wtxn, &key)? else {
            return Err(LmdbError::Sane("key-value type mismatch in log db"));
        };
        res += 1;
        self.db.put(wtxn, &key, &LogEntry::Head(res))?;
        return Ok(res)
    }

    pub fn append(&self, wtxn: &mut heed::RwTxn, jam: Vec<u8>) -> Result<()> {
        let idx = self.bump(wtxn)?;
        self.db.put(wtxn, &LogKey::Epoch(idx), &LogEntry::Entry(jam))?;
        Ok(())
    }


}

