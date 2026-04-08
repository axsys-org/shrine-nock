//! Integration tests for the seed library

use super::{Seed, word64_bits, word64_bytes};

#[test]
fn test_word64_bits() {
    assert_eq!(word64_bits(0), 0);
    assert_eq!(word64_bits(1), 1);
    assert_eq!(word64_bits(2), 2);
    assert_eq!(word64_bits(3), 2);
    assert_eq!(word64_bits(4), 3);
    assert_eq!(word64_bits(7), 3);
    assert_eq!(word64_bits(8), 4);
    assert_eq!(word64_bits(255), 8);
    assert_eq!(word64_bits(256), 9);
    assert_eq!(word64_bits(u64::MAX), 64);
}

#[test]
fn test_word64_bytes() {
    assert_eq!(word64_bytes(0), 1);
    assert_eq!(word64_bytes(1), 1);
    assert_eq!(word64_bytes(255), 1);
    assert_eq!(word64_bytes(256), 2);
    assert_eq!(word64_bytes(65535), 2);
    assert_eq!(word64_bytes(65536), 3);
}

#[test]
fn test_empty_seed() {
    let mut seed = Seed::new();
    assert!(seed.done().is_err());
}

#[test]
fn test_single_word() {
    let mut seed = Seed::new();
    let node = seed.word(42);
    assert_eq!(node.0, 0);
    assert_eq!(seed.nat_count(), 1);
    assert_eq!(seed.treenode_count(), 1);
}

#[test]
fn test_multiple_words() {
    let mut seed = Seed::new();
    let a = seed.word(1);
    let b = seed.word(2);
    let c = seed.word(3);
    
    assert_eq!(a.0, 0);
    assert_eq!(b.0, 1);
    assert_eq!(c.0, 2);
    assert_eq!(seed.nat_count(), 3);
}

#[test]
fn test_word_deduplication() {
    let mut seed = Seed::new();
    let a = seed.word(42);
    let b = seed.word(42);
    let c = seed.word(42);
    
    assert_eq!(a.0, b.0);
    assert_eq!(b.0, c.0);
    assert_eq!(seed.nat_count(), 1);
}

#[test]
fn test_cons_basic() {
    let mut seed = Seed::new();
    let a = seed.word(1);
    let b = seed.word(2);
    let c = seed.cons(a, b);
    
    assert_eq!(c.0, 2); // After the two leaves
    assert_eq!(seed.treenode_count(), 3);
}

#[test]
fn test_cons_deduplication() {
    let mut seed = Seed::new();
    let a = seed.word(1);
    let b = seed.word(2);
    
    let c1 = seed.cons(a, b);
    let c2 = seed.cons(a, b);
    
    assert_eq!(c1.0, c2.0);
    assert_eq!(seed.treenode_count(), 3);
}

#[test]
fn test_hole() {
    let mut seed = Seed::new();
    let h1 = seed.hole();
    let h2 = seed.hole();
    
    assert_eq!(seed.hole_count(), 2);
    assert_ne!(h1.0, h2.0);
}

#[test]
fn test_nested_cons() {
    let mut seed = Seed::new();
    let a = seed.word(1);
    let b = seed.word(2);
    let c = seed.word(3);
    
    let ab = seed.cons(a, b);
    let abc = seed.cons(ab, c);
    
    assert_eq!(seed.treenode_count(), 5);
    assert_eq!(abc.0, 4);
}

#[test]
fn test_deep_tree() {
    let mut seed = Seed::new();
    let zero = seed.word(0);
    let mut current = zero;
    
    for i in 1..100u64 {
        let n = seed.word(i);
        current = seed.cons(n, current);
    }
    
    assert_eq!(seed.nat_count(), 100);
    // 100 nats + 99 cons cells
    assert_eq!(seed.treenode_count(), 199);
}

#[test]
fn test_round_trip_single_byte() {
    let mut seed = Seed::new();
    seed.word(42);
    seed.done().unwrap();
    
    let size = seed.size();
    let mut buf = vec![0u8; size];
    let written = seed.save(&mut buf);
    assert_eq!(written, size);
    
    let mut seed2 = Seed::new();
    seed2.load(&buf).unwrap();
    
    assert_eq!(seed2.nat_count(), 1);
}

#[test]
fn test_round_trip_single_word() {
    let mut seed = Seed::new();
    seed.word(0x123456789ABCDEF0);
    seed.done().unwrap();
    
    let size = seed.size();
    let mut buf = vec![0u8; size];
    let written = seed.save(&mut buf);
    assert_eq!(written, size);
    
    let mut seed2 = Seed::new();
    seed2.load(&buf).unwrap();
    
    assert_eq!(seed2.nat_count(), 1);
}

#[test]
fn test_round_trip_pair() {
    let mut seed = Seed::new();
    let a = seed.word(1);
    let b = seed.word(2);
    seed.cons(a, b);
    seed.done().unwrap();
    
    let size = seed.size();
    let mut buf = vec![0u8; size];
    let written = seed.save(&mut buf);
    assert_eq!(written, size);
    
    let mut seed2 = Seed::new();
    seed2.load(&buf).unwrap();
    
    assert_eq!(seed2.nat_count(), 2);
    assert_eq!(seed2.frag_count(), 1);
}

#[test]
fn test_round_trip_tree() {
    let mut seed = Seed::new();
    let a = seed.word(1);
    let b = seed.word(2);
    let c = seed.cons(a, b);
    let d = seed.word(3);
    let e = seed.cons(c, d);
    let f = seed.word(4);
    seed.cons(e, f);
    seed.done().unwrap();
    
    let size = seed.size();
    let mut buf = vec![0u8; size];
    let written = seed.save(&mut buf);
    assert_eq!(written, size);
    
    let mut seed2 = Seed::new();
    seed2.load(&buf).unwrap();
    
    assert_eq!(seed2.nat_count(), seed.nat_count());
}

#[test]
fn test_round_trip_with_holes() {
    let mut seed = Seed::new();
    let h = seed.hole();
    let a = seed.word(42);
    seed.cons(h, a);
    seed.done().unwrap();
    
    let size = seed.size();
    let mut buf = vec![0u8; size];
    let written = seed.save(&mut buf);
    assert_eq!(written, size);
    
    let mut seed2 = Seed::new();
    seed2.load(&buf).unwrap();
    
    assert_eq!(seed2.hole_count(), 1);
    assert_eq!(seed2.nat_count(), 1);
}

#[test]
fn test_round_trip_list() {
    let mut seed = Seed::new();
    let nil = seed.word(0);
    let mut list = nil;
    
    for i in 1..=10u64 {
        let n = seed.word(i);
        list = seed.cons(n, list);
    }
    
    seed.done().unwrap();
    
    let size = seed.size();
    let mut buf = vec![0u8; size];
    let written = seed.save(&mut buf);
    assert_eq!(written, size);
    
    let mut seed2 = Seed::new();
    seed2.load(&buf).unwrap();
    
    assert_eq!(seed2.nat_count(), seed.nat_count());
}

#[test]
fn test_wipe() {
    let mut seed = Seed::new();
    seed.word(1);
    seed.word(2);
    
    assert_eq!(seed.nat_count(), 2);
    
    seed.wipe();
    
    assert_eq!(seed.nat_count(), 0);
    assert_eq!(seed.treenode_count(), 0);
    assert_eq!(seed.hole_count(), 0);
    
    // Can reuse after wipe
    seed.word(42);
    assert_eq!(seed.nat_count(), 1);
}

#[test]
fn test_nat_from_words() {
    let mut seed = Seed::new();
    let words = vec![0x123456789ABCDEF0, 0xFEDCBA9876543210];
    let node = seed.nat(&words).unwrap();
    
    assert_eq!(node.0, 0);
    assert_eq!(seed.nat_count(), 1);
}

#[test]
fn test_nat_empty() {
    let mut seed = Seed::new();
    let words: Vec<u64> = vec![];
    let node = seed.nat(&words).unwrap();
    
    // Empty nat is just 0
    assert_eq!(seed.nat_count(), 1);
}

#[test]
fn test_barnat_small() {
    let mut seed = Seed::new();
    let bytes = [1, 2, 3];
    let node = seed.barnat(&bytes);
    
    assert_eq!(node.0, 0);
    assert_eq!(seed.nat_count(), 1);
}

#[test]
fn test_barnat_large() {
    let mut seed = Seed::new();
    let bytes: Vec<u8> = (0..100).collect();
    let node = seed.barnat(&bytes);
    
    assert_eq!(node.0, 0);
    assert_eq!(seed.nat_count(), 1);
}

#[test]
fn test_shared_subtree() {
    let mut seed = Seed::new();
    
    // Create a subtree
    let a = seed.word(1);
    let b = seed.word(2);
    let subtree = seed.cons(a, b);
    
    // Use it twice
    let c = seed.word(3);
    let left = seed.cons(subtree, c);
    
    let d = seed.word(4);
    let right = seed.cons(subtree, d);
    
    // Combine
    seed.cons(left, right);
    
    seed.done().unwrap();
    
    // The subtree should be deduplicated
    let size = seed.size();
    let mut buf = vec![0u8; size];
    seed.save(&mut buf);
    
    let mut seed2 = Seed::new();
    seed2.load(&buf).unwrap();
    
    // Check structure is preserved
    assert_eq!(seed2.nat_count(), seed.nat_count());
}

#[test]
fn test_size_alignment() {
    let mut seed = Seed::new();
    seed.word(1);
    seed.done().unwrap();
    
    let size = seed.size();
    assert_eq!(size % 8, 0, "Size must be multiple of 8");
}

#[test]
fn test_buffer_too_small() {
    let buf = vec![0u8; 10]; // Too small
    let mut seed = Seed::new();
    
    let result = seed.load(&buf);
    assert!(result.is_err());
}

#[test]
fn test_buffer_wrong_alignment() {
    let buf = vec![0u8; 41]; // Not multiple of 8
    let mut seed = Seed::new();
    
    let result = seed.load(&buf);
    assert!(result.is_err());
}
