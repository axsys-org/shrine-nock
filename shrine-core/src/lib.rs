//! # shrine-core
//!
//! An async layer on top of `shrine-storage` that provides channel-based messaging
//! and built-in Nock compute capabilities.
//!
//! ## Overview
//!
//! `shrine-core` wraps `shrine-storage` and adds:
//! - An **async actor model** with channels for message passing
//! - A **compute layer** for executing Nock gates as state transitions
//! - **Protocol types** for the namespace message format (see [`sept`] module)
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use shrine_core::Shrine;
//!
//! // Get the shared shrine instance (singleton with Nock runtime)
//! let shrine = Shrine::shared();
//! let mut shrine = shrine.lock().unwrap();
//!
//! // Read and write operations
//! shrine.make("/config", HashMap::from([
//!     ("setting".to_string(), Pail::new("/types/text", b"enabled".to_vec())),
//! ])).unwrap();
//!
//! let result = shrine.read_x("/config", None).unwrap();
//! ```
//!
//! ## Care Levels
//!
//! Reads support three observation levels:
//!
//! | Care | Returns |
//! |------|---------|
//! | `X` | Data at the exact path |
//! | `Y` | Immediate children of the path |
//! | `Z` | All descendants of the path |

// use shrine_storage::StorageError;

// pub mod handler;
pub mod boot;
pub mod driver;
pub mod oneshot;
pub mod fate;
pub mod http;
pub mod jam_sync;
pub mod news;
pub mod react;
pub mod shrine;
pub mod supervisor;
pub mod types;
pub mod docs;
// pub mod sept;

// Re-export commonly used types from shrine-storage
pub use shrine_storage::core::types::{Care, DiskPail, Lock, Observe, Tale};

use shrine_storage::store::NamespaceError;

// Re-export milton types for convenience
pub use milton::{Atom, Cell, Noun, Runtime, RuntimeError, cue};

/// Errors that can occur in shrine-core operations.
#[derive(Debug)]
pub enum ShrineError {
    /// Error from the underlying storage layer.
    Storage(NamespaceError),
    /// Error from the Nock runtime.
    Runtime(RuntimeError),
    /// Other error with a static message.
    Other(&'static str),
}

impl From<NamespaceError> for ShrineError {
    fn from(e: NamespaceError) -> Self {
        ShrineError::Storage(e)
    }
}

impl std::fmt::Display for ShrineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShrineError::Storage(e) => write!(f, "storage error: {}", e),
            ShrineError::Runtime(e) => write!(f, "runtime error: {}", e),
            ShrineError::Other(e) => write!(f, "other error: {}", e),
        }
    }
}

impl std::error::Error for ShrineError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ShrineError::Storage(e) => Some(e),
            ShrineError::Runtime(e) => Some(e),
            ShrineError::Other(_) => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, ShrineError>;

/// Default loom size for the Nock runtime (256MB).
pub const DEFAULT_LOOM_SIZE: usize = 1 << 33;

/// Test utilities shared across all test modules.
/// The Nock runtime is not thread-safe, so all tests must run serially.
pub mod test_utils {
    use std::sync::LazyLock;

    use tokio::sync::Mutex;

    /// Global mutex to ensure tests run serially (Nock runtime is not thread-safe).
    pub static TEST_MUTEX: LazyLock<Mutex<()>> = std::sync::LazyLock::new(|| Mutex::new(()));
}
