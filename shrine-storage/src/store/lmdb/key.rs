use std::borrow::Cow;

use crate::{core::{path::PathIdx, types::Case}, store::lmdb::LmdbError};

#[derive(Clone, Debug, PartialEq, Eq, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct PathKey(pub PathIdx, pub Case);

impl PathKey {
  pub fn to_bytes(&self) -> [u8; 8] {
    let mut bytes = [0; 8];
    bytes[0..4].copy_from_slice(&self.0.to_bytes());
    bytes[4..8].copy_from_slice(&self.1.to_be_bytes());
    bytes
  }

  pub fn from_bytes(bytes: &[u8]) -> std::result::Result<Self, LmdbError> {
    if bytes.len() != 8 {
      return Err(LmdbError::InvalidKeyLength(bytes.len()));
    }
    // SAFETY: only returns null if byte len is not 4, which it isn't by construction
    let path = PathIdx::from_bytes(&bytes[0..4]).expect("Invalid path bytes");
    let case = u32::from_be_bytes(bytes[4..8].try_into().unwrap());
    Ok(Self(path, case))
  }
} 

#[derive(Debug, Clone)]
pub struct PathKeyCodec { }

impl<'a> heed::BytesEncode<'a> for PathKeyCodec {
  type EItem = PathKey;

  fn bytes_encode(key: &PathKey) -> Result<Cow<'_, [u8]>, Box<dyn std::error::Error + Send + Sync>> {
    let bytes = key.to_bytes();
    Ok(Cow::Owned(bytes.to_vec()))
  }


}

impl<'a> heed::BytesDecode<'a> for PathKeyCodec {
  type DItem = PathKey;

  fn bytes_decode(bytes: &'a [u8]) -> std::result::Result<PathKey, Box<dyn std::error::Error + Send + Sync>> {
    let key = PathKey::from_bytes(bytes).map_err(|e| Box::new(e))?;
    Ok(key)
  }
}

