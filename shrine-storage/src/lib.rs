//! # shrine-storage
//!
//! A versioned, path-based storage system built on SQLite with complete history tracking.
//!
//! `shrine-storage` provides a hierarchical data storage model where every change is
//! versioned and the complete history is preserved. It's designed for applications that
//! need audit trails, time-travel queries, or optimistic concurrency control.
//!
//! ## Core Concepts
//!
//! ### Paths
//!
//! Data is organized using slash-separated paths, similar to a filesystem:
//! - `/` - the root path
//! - `/users` - a top-level path
//! - `/users/alice` - a nested path
//!
//! Paths must start with `/`, cannot end with `/` (except root), and cannot contain
//! empty components (`//`).
//!
//! ### Three Observation Levels
//!
//! The storage system maintains three different "views" of the data hierarchy,
//! each with independent version tracking:
//!
//! | Level | Name | Description |
//! |-------|------|-------------|
//! | **%x** | File-level | The actual data (slots) stored at a specific path |
//! | **%y** | Folder-level | A snapshot of immediate children and their versions |
//! | **%z** | Subtree-level | A snapshot of all descendants and their versions |
//!
//! When you write data at a path, the system automatically propagates version
//! updates through the hierarchy:
//! - The path's %x version increments
//! - The parent's %y version increments (children changed)
//! - All ancestors' %z versions increment (descendants changed)
//!
//! ### Locks (Versioning)
//!
//! Each binding has a [`Lock`] containing two version numbers:
//! - **`data`**: Increments on every content change (must be sequential: 1, 2, 3...)
//! - **`shape`**: Increments on structural changes (creation/deletion), stays same on updates
//!
//! ### Slots and Pails
//!
//! Each path can store multiple named slots. Each slot maps to a [`Pail`] containing
//! a type path (namespace path for validation) and binary data. A binding with empty
//! slots is a tombstone (deletion marker).
//!
//! ## Quick Start
//!
//! ```rust
//! use shrine_storage::{Storage, Binding, Lock, Pail, ReadResult};
//! use std::collections::HashMap;
//!
//! // Create an in-memory database (use Storage::open("path") for persistence)
//! let mut storage = Storage::open_in_memory().unwrap();
//!
//! // Write data at a path
//! // Lock::new(data_version, shape_version) - first write is always (1, 1)
//! let mut slots = HashMap::new();
//! slots.insert("content".to_string(), Pail::new("/types/text", b"Hello, world!".to_vec()));
//!
//! storage.batch_write(&[
//!     Binding::new("/documents/readme", Lock::new(1, 1), slots),
//! ]).unwrap();
//!
//! // Read the current version
//! match storage.read_x("/documents/readme", None).unwrap() {
//!     ReadResult::Found(x) => {
//!         assert_eq!(x.lock.data, 1);
//!         assert_eq!(x.slots.get("content").unwrap().data, b"Hello, world!");
//!     }
//!     _ => panic!("expected data"),
//! }
//! ```
//!
//! ## Complete Workflow Example
//!
//! This example demonstrates creating, updating, reading history, and deleting data:
//!
//! ```rust
//! use shrine_storage::{Storage, Binding, Lock, Care, Pail, ReadResult};
//! use std::collections::HashMap;
//!
//! let mut storage = Storage::open_in_memory().unwrap();
//!
//! // === Step 1: Create initial document ===
//! let mut slots_v1 = HashMap::new();
//! slots_v1.insert("title".to_string(), Pail::new("/types/text", b"Draft".to_vec()));
//! slots_v1.insert("body".to_string(), Pail::new("/types/text", b"Initial content".to_vec()));
//!
//! storage.batch_write(&[
//!     Binding::new("/docs/report", Lock::new(1, 1), slots_v1),
//! ]).unwrap();
//!
//! // === Step 2: Update the document ===
//! // data version must be previous + 1, shape stays same (not a structural change)
//! let mut slots_v2 = HashMap::new();
//! slots_v2.insert("title".to_string(), Pail::new("/types/text", b"Final Report".to_vec()));
//! slots_v2.insert("body".to_string(), Pail::new("/types/text", b"Updated content with more details".to_vec()));
//!
//! storage.batch_write(&[
//!     Binding::new("/docs/report", Lock::new(2, 1), slots_v2),
//! ]).unwrap();
//!
//! // === Step 3: Read current and historical versions ===
//! // Read current (latest) version
//! let current = storage.read_x("/docs/report", None).unwrap();
//! if let ReadResult::Found(x) = current {
//!     assert_eq!(x.lock.data, 2);
//!     assert_eq!(x.slots.get("title").unwrap().data, b"Final Report");
//! }
//!
//! // Read historical version 1
//! let historical = storage.read_x("/docs/report", Some(1)).unwrap();
//! if let ReadResult::Found(x) = historical {
//!     assert_eq!(x.slots.get("title").unwrap().data, b"Draft");
//! }
//!
//! // === Step 4: Check folder-level view (%y) ===
//! // The parent "/docs" automatically tracks its children
//! let folder = storage.read_y("/docs", None).unwrap();
//! if let ReadResult::Found(y) = folder {
//!     assert_eq!(y.children.len(), 1);
//!     assert_eq!(y.children[0].path, "/docs/report");
//!     assert_eq!(y.children[0].lock.data, 2); // child's current version
//! }
//!
//! // === Step 5: Delete the document ===
//! // Deletion = empty slots with incremented versions (shape must increment)
//! storage.batch_write(&[
//!     Binding::new("/docs/report", Lock::new(3, 2), HashMap::new()),
//! ]).unwrap();
//!
//! // Current read returns Found with empty slots (tombstone)
//! if let ReadResult::Found(x) = storage.read_x("/docs/report", None).unwrap() {
//!     assert!(x.slots.is_empty()); // empty slots = deleted
//!     assert_eq!(x.lock.data, 3);
//! }
//!
//! // Historical versions are still accessible with their data!
//! let old_version = storage.read_x("/docs/report", Some(2)).unwrap();
//! if let ReadResult::Found(x) = old_version {
//!     assert!(!x.slots.is_empty()); // had data at v2
//! }
//!
//! // === Step 6: Query current lock for optimistic concurrency ===
//! let lock = storage.get_current_lock("/docs/report", Care::X).unwrap();
//! // Returns Some even for tombstones - you can still write new versions
//! assert!(lock.is_some());
//! assert_eq!(lock.unwrap().data, 3);
//! ```
//!
//! ## ReadResult Semantics
//!
//! The [`ReadResult`] enum distinguishes three states:
//!
//! - **`Found(T)`** - Data exists at the requested version
//! - **`Null`** - The version exists but this path has no binding (deleted or never created)
//! - **`Unknown`** - The requested version doesn't exist yet (future version)
//!
//! ```rust
//! use shrine_storage::{Storage, ReadResult};
//!
//! let storage = Storage::open_in_memory().unwrap();
//!
//! // Path never written - requesting any version is Unknown
//! assert!(matches!(
//!     storage.read_x("/nonexistent", None).unwrap(),
//!     ReadResult::Unknown
//! ));
//!
//! // Requesting a future version is also Unknown
//! assert!(matches!(
//!     storage.read_x("/nonexistent", Some(999)).unwrap(),
//!     ReadResult::Unknown
//! ));
//! ```
//!
//! ## Batch Atomicity
//!
//! All writes in a [`Storage::batch_write`] call are atomic - either all succeed
//! or none do. This enables safe multi-path updates:
//!
//! ```rust
//! use shrine_storage::{Storage, Binding, Lock, Pail};
//! use std::collections::HashMap;
//!
//! let mut storage = Storage::open_in_memory().unwrap();
//!
//! // Atomic multi-path write
//! storage.batch_write(&[
//!     Binding::new("/accounts/alice", Lock::new(1, 1),
//!         HashMap::from([("balance".into(), Pail::new("/types/text", b"100".to_vec()))])),
//!     Binding::new("/accounts/bob", Lock::new(1, 1),
//!         HashMap::from([("balance".into(), Pail::new("/types/text", b"50".to_vec()))])),
//! ]).unwrap();
//! ```
//!
//! ## Version Validation
//!
//! The storage enforces strict version sequencing:
//!
//! - `data` version must be exactly `previous + 1` (or `1` for new paths)
//! - `shape` version must be `>= previous` (can stay same or increase)
//! - Shape must increment on creation (first write) and deletion (empty slots)
//!
//! ```rust
//! use shrine_storage::{Storage, Binding, Lock, StorageError};
//! use std::collections::HashMap;
//!
//! let mut storage = Storage::open_in_memory().unwrap();
//!
//! // First write must be version 1
//! let result = storage.batch_write(&[
//!     Binding::new("/test", Lock::new(5, 1), HashMap::new()), // Wrong! Should be 1
//! ]);
//!
//! assert!(matches!(
//!     result,
//!     Err(StorageError::VersionConflict { expected: 1, actual: 5, .. })
//! ));
//! ```
//!
//! ## Thread Safety
//!
//! `Storage` wraps a SQLite connection and is not `Sync`. For concurrent access,
//! use separate `Storage` instances pointing to the same database file - SQLite's
//! WAL mode handles the coordination.
#![feature(error_generic_member_access)]
pub mod serial;
pub mod blob_store;
pub mod core;
pub mod store;
pub mod overlay;

pub use blob_store::{BLOB_SIZE_THRESHOLD, BlobHash, BlobStore};

pub static NAMESPACE: OnceLock<Arc<store::Namespace>> = OnceLock::new();

pub use core::types::*;
use std::sync::{Arc, OnceLock};

use crate::store::Namespace;

