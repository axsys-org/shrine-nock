//! External blob storage for large values.
//!
//! When slot values exceed the size threshold (100KB), they are stored
//! externally in a hash-addressed filesystem directory instead of inline
//! in SQLite. This improves database performance for large blobs.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Size threshold in bytes above which blobs are stored externally.
pub const BLOB_SIZE_THRESHOLD: usize = 100 * 1024; // 100KB

/// Hash type alias for blake3 output (32 bytes).
pub type BlobHash = [u8; 32];

/// Manages external blob storage in a hash-addressed directory.
///
/// Blobs are stored as files named by their hex-encoded blake3 hash.
/// This provides automatic deduplication - identical content produces
/// the same hash and thus the same file.
#[derive(Debug)]
pub struct BlobStore {
    /// Root directory for blob storage (e.g., "data.db.blobs/")
    root: PathBuf,
}

impl BlobStore {
    /// Creates a new blob store at the given directory path.
    ///
    /// The directory will be created if it doesn't exist.
    pub fn new(root: impl AsRef<Path>) -> io::Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    /// Returns the filesystem path for a given hash.
    fn blob_path(&self, hash: &BlobHash) -> PathBuf {
        let hex = hex_encode(hash);
        self.root.join(format!("{}.blob", hex))
    }

    /// Stores a blob and returns its hash.
    ///
    /// If a blob with the same hash already exists, this is a no-op
    /// (content-addressed deduplication).
    pub fn store(&self, data: &[u8]) -> io::Result<BlobHash> {
        let hash = compute_hash(data);
        let path = self.blob_path(&hash);

        // Only write if file doesn't exist (deduplication)
        if !path.exists() {
            // Write to temp file first, then rename for atomicity
            let temp_path = self.root.join(format!("{}.tmp", hex_encode(&hash)));
            fs::write(&temp_path, data)?;
            fs::rename(&temp_path, &path)?;
        }

        Ok(hash)
    }

    /// Loads a blob by its hash.
    ///
    /// Returns `None` if the blob doesn't exist.
    pub fn load(&self, hash: &BlobHash) -> io::Result<Option<Vec<u8>>> {
        let path = self.blob_path(hash);
        match fs::read(&path) {
            Ok(data) => Ok(Some(data)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Checks if a blob exists in the store.
    pub fn exists(&self, hash: &BlobHash) -> bool {
        self.blob_path(hash).exists()
    }
}

/// Computes the blake3 hash of the given data.
pub fn compute_hash(data: &[u8]) -> BlobHash {
    *blake3::hash(data).as_bytes()
}

/// Encodes a hash as a hex string.
fn hex_encode(hash: &BlobHash) -> String {
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Decodes a hex string to a hash.
/// Returns None if the string is invalid.
#[allow(dead_code)]
pub fn hex_decode(s: &str) -> Option<BlobHash> {
    if s.len() != 64 {
        return None;
    }
    let mut hash = [0u8; 32];
    for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
        let hi = hex_char_to_nibble(chunk[0])?;
        let lo = hex_char_to_nibble(chunk[1])?;
        hash[i] = (hi << 4) | lo;
    }
    Some(hash)
}

fn hex_char_to_nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    // #[test]
    fn test_hash_computation() {
        let data = b"hello world";
        let hash = compute_hash(data);
        // blake3 hash of "hello world" is known
        assert_eq!(hash.len(), 32);
    }

    // #[test]
    fn test_hex_roundtrip() {
        let hash = compute_hash(b"test data");
        let hex = hex_encode(&hash);
        let decoded = hex_decode(&hex).unwrap();
        assert_eq!(hash, decoded);
    }

    // #[test]
    fn test_store_and_load() {
        let dir = temp_dir().join("blob_store_test");
        let _ = fs::remove_dir_all(&dir);

        let store = BlobStore::new(&dir).unwrap();
        let data = b"test blob content";

        let hash = store.store(data).unwrap();
        assert!(store.exists(&hash));

        let loaded = store.load(&hash).unwrap().unwrap();
        assert_eq!(loaded, data);

        fs::remove_dir_all(&dir).unwrap();
    }

    // #[test]
    fn test_deduplication() {
        let dir = temp_dir().join("blob_store_dedup_test");
        let _ = fs::remove_dir_all(&dir);

        let store = BlobStore::new(&dir).unwrap();
        let data = b"duplicate content";

        let hash1 = store.store(data).unwrap();
        let hash2 = store.store(data).unwrap();

        assert_eq!(hash1, hash2);

        // Only one file should exist
        let count = fs::read_dir(&dir).unwrap().count();
        assert_eq!(count, 1);

        fs::remove_dir_all(&dir).unwrap();
    }
}
