use crate::core::{path::{PathIdx, PathMeta}, types::{Case, Lock, Saga, Tale, Upon}};



pub enum XResultRef<Blob> {
  Unknown,
  Null { lock: Lock },
  Known { lock: Lock, aeon: Blob }
}
/// A snapshot of the store at a specific point in time. READONLY
pub trait Snapshot<'a> {
  type Error: std::error::Error;
  type BlobRef;
  fn resolve_path(&self, path: &str) -> Result<Option<PathIdx>, Self::Error>;
  fn get_meta(&self, path: PathIdx) -> Result<Option<PathMeta>, Self::Error>;
  fn look(&self, path: PathIdx) -> Result<XResultRef<Self::BlobRef>, Self::Error>;
  fn x_asof(&self, p: PathIdx, case: Case) -> Result<XResultRef<Self::BlobRef>, Self::Error>;
  fn read_saga(&self, p: PathIdx, case: Case) -> Result<Saga, Self::Error>;
  fn has_slot(&self, p: PathIdx, slot: PathIdx) -> Result<bool, Self::Error>;

  fn iter_descendants(&self, p: PathIdx) -> Result<impl Iterator<Item = PathIdx>, Self::Error>;

}

pub trait Store {
  type Txn<'a>: Txn<'a> where Self: 'a; // lifetime of the store
  type Error: std::error::Error;
  fn begin<'a>(&'a mut self) -> Result<Self::Txn<'a>, Self::Error> where Self: 'a;
} 

pub trait Txn<'a>: Snapshot<'a> {
  fn alloc_upon(&mut self) -> Result<Upon, Self::Error>;

  fn alloc_path(&mut self, path: &str) -> Result<PathIdx, Self::Error>;
  fn write_x(&mut self, p: PathIdx, saga: Option<Tale>, upon: Upon) -> Result<(), Self::Error>;
  fn commit(self: Box<Self>) -> Result<(), Self::Error>;
}

