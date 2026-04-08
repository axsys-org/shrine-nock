//! Hash functions for seed deduplication

/// MurmurHash3 finalizer (fmix64)
/// Used for mixing hash values
#[inline]
pub fn fmix64(mut k: u64) -> u64 {
    k ^= k >> 33;
    k = k.wrapping_mul(0xff51afd7ed558ccd);
    k ^= k >> 33;
    k = k.wrapping_mul(0xc4ceb9fe1a85ec53);
    k ^= k >> 33;
    k
}

/// XXH3 64-bit hash (simplified implementation)
/// For proper production use, consider using the xxhash-rust crate
pub fn xxh3_64bits(data: &[u8]) -> u64 {
    // This is a simplified implementation
    // For full correctness, use the actual xxh3 algorithm

    const PRIME64_1: u64 = 0x9E3779B185EBCA87;
    const PRIME64_2: u64 = 0xC2B2AE3D27D4EB4F;
    const PRIME64_3: u64 = 0x165667B19E3779F9;
    #[allow(dead_code)]
    const PRIME64_4: u64 = 0x85EBCA77C2B2AE63;
    const PRIME64_5: u64 = 0x27D4EB2F165667C5;

    let len = data.len();
    let mut hash: u64;

    if len >= 32 {
        // Process 32-byte chunks
        let mut v1 = PRIME64_1.wrapping_add(PRIME64_2);
        let mut v2 = PRIME64_2;
        let mut v3 = 0u64;
        let mut v4 = PRIME64_1.wrapping_neg();

        let chunks = len / 32;
        for i in 0..chunks {
            let base = i * 32;
            
            let k1 = read_u64_le(&data[base..]);
            let k2 = read_u64_le(&data[base + 8..]);
            let k3 = read_u64_le(&data[base + 16..]);
            let k4 = read_u64_le(&data[base + 24..]);

            v1 = round(v1, k1);
            v2 = round(v2, k2);
            v3 = round(v3, k3);
            v4 = round(v4, k4);
        }

        hash = v1.rotate_left(1)
            .wrapping_add(v2.rotate_left(7))
            .wrapping_add(v3.rotate_left(12))
            .wrapping_add(v4.rotate_left(18));

        hash = merge_round(hash, v1);
        hash = merge_round(hash, v2);
        hash = merge_round(hash, v3);
        hash = merge_round(hash, v4);

        // Process remaining 8-byte chunks
        let remaining = &data[chunks * 32..];
        hash = process_remaining(hash, remaining, len);
    } else if len >= 8 {
        hash = PRIME64_5.wrapping_add(len as u64);
        hash = process_remaining(hash, data, len);
    } else {
        hash = PRIME64_5.wrapping_add(len as u64);
        
        // Process remaining bytes
        let mut i = 0;
        while i + 4 <= len {
            let k = read_u32_le(&data[i..]) as u64;
            hash ^= k.wrapping_mul(PRIME64_1);
            hash = hash.rotate_left(23).wrapping_mul(PRIME64_2).wrapping_add(PRIME64_3);
            i += 4;
        }

        while i < len {
            hash ^= (data[i] as u64).wrapping_mul(PRIME64_5);
            hash = hash.rotate_left(11).wrapping_mul(PRIME64_1);
            i += 1;
        }

        hash = finalize(hash);
    }

    hash
}

#[inline]
fn round(mut acc: u64, input: u64) -> u64 {
    const PRIME64_1: u64 = 0x9E3779B185EBCA87;
    const PRIME64_2: u64 = 0xC2B2AE3D27D4EB4F;
    
    acc = acc.wrapping_add(input.wrapping_mul(PRIME64_2));
    acc = acc.rotate_left(31);
    acc.wrapping_mul(PRIME64_1)
}

#[inline]
fn merge_round(mut acc: u64, val: u64) -> u64 {
    const PRIME64_1: u64 = 0x9E3779B185EBCA87;
    const PRIME64_4: u64 = 0x85EBCA77C2B2AE63;
    
    let val = round(0, val);
    acc ^= val;
    acc.wrapping_mul(PRIME64_1).wrapping_add(PRIME64_4)
}

fn process_remaining(mut hash: u64, data: &[u8], _total_len: usize) -> u64 {
    const PRIME64_1: u64 = 0x9E3779B185EBCA87;
    const PRIME64_2: u64 = 0xC2B2AE3D27D4EB4F;
    const PRIME64_3: u64 = 0x165667B19E3779F9;
    const PRIME64_4: u64 = 0x85EBCA77C2B2AE63;
    const PRIME64_5: u64 = 0x27D4EB2F165667C5;

    let len = data.len();
    let mut i = 0;

    // Process 8-byte chunks
    while i + 8 <= len {
        let k = read_u64_le(&data[i..]);
        hash ^= round(0, k);
        hash = hash.rotate_left(27).wrapping_mul(PRIME64_1).wrapping_add(PRIME64_4);
        i += 8;
    }

    // Process 4-byte chunks
    while i + 4 <= len {
        let k = read_u32_le(&data[i..]) as u64;
        hash ^= k.wrapping_mul(PRIME64_1);
        hash = hash.rotate_left(23).wrapping_mul(PRIME64_2).wrapping_add(PRIME64_3);
        i += 4;
    }

    // Process remaining bytes
    while i < len {
        hash ^= (data[i] as u64).wrapping_mul(PRIME64_5);
        hash = hash.rotate_left(11).wrapping_mul(PRIME64_1);
        i += 1;
    }

    finalize(hash)
}

#[inline]
fn finalize(mut hash: u64) -> u64 {
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xC2B2AE3D27D4EB4F);
    hash ^= hash >> 29;
    hash = hash.wrapping_mul(0x165667B19E3779F9);
    hash ^= hash >> 32;
    hash
}

#[inline]
fn read_u64_le(data: &[u8]) -> u64 {
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&data[..8]);
    u64::from_le_bytes(buf)
}

#[inline]
fn read_u32_le(data: &[u8]) -> u32 {
    let mut buf = [0u8; 4];
    buf.copy_from_slice(&data[..4]);
    u32::from_le_bytes(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fmix64() {
        // Test that fmix64 produces consistent results
        assert_eq!(fmix64(0), fmix64(0));
        assert_ne!(fmix64(0), fmix64(1));
        
        // Known test vectors could be added here
        let hash = fmix64(12345);
        assert_ne!(hash, 12345);
    }

    #[test]
    fn test_xxh3_empty() {
        let hash = xxh3_64bits(&[]);
        assert_ne!(hash, 0);
    }

    #[test]
    fn test_xxh3_small() {
        let hash1 = xxh3_64bits(b"hello");
        let hash2 = xxh3_64bits(b"hello");
        let hash3 = xxh3_64bits(b"world");
        
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_xxh3_large() {
        let data: Vec<u8> = (0..100).collect();
        let hash = xxh3_64bits(&data);
        assert_ne!(hash, 0);
    }
}
