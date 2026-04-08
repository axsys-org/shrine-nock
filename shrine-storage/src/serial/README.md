# Seed-RS

A Rust port of the Plunder seed serialization library for tree structures with deduplication.

## Overview

Seed provides efficient serialization of tree structures with:

- **Leaf deduplication**: Identical atoms (natural numbers) are stored only once
- **Interior node deduplication**: Identical subtrees are stored only once
- **Fragment-based compression**: Repeated subtrees are extracted as fragments
- **Efficient bit-packed serialization**: Compact binary format

## Usage

```rust
use seed::Seed;

fn main() {
    // Create a new seed context
    let mut seed = Seed::new();
    
    // Insert atoms (natural numbers)
    let a = seed.word(1);
    let b = seed.word(2);
    
    // Build trees with cons cells
    let pair = seed.cons(a, b);
    
    // Create external references (holes)
    let hole = seed.hole();
    let with_hole = seed.cons(pair, hole);
    
    // Finalize for serialization
    seed.done().unwrap();
    
    // Serialize
    let size = seed.size();
    let mut buf = vec![0u8; size];
    let written = seed.save(&mut buf);
    
    // Deserialize
    let mut seed2 = Seed::new();
    seed2.load(&buf).unwrap();
    
    // Display the tree
    seed2.show();
}
```

## API

### Creating Values

- `Seed::new()` - Create a new seed context
- `seed.word(u64)` - Insert a 64-bit natural number
- `seed.nat(&[u64])` - Insert a multi-word natural number
- `seed.barnat(&[u8])` - Insert a natural number from bytes
- `seed.cons(head, tail)` - Create a pair (cons cell)
- `seed.hole()` - Create an external reference

### Serialization

- `seed.done()` - Finalize the context (required before serialization)
- `seed.size()` - Calculate the serialized size
- `seed.save(&mut [u8])` - Serialize to a buffer
- `seed.load(&[u8])` - Deserialize from a buffer

### Utilities

- `seed.wipe()` - Clear the context for reuse
- `seed.show()` - Print the tree (for debugging)
- `seed.debug()` - Print detailed debug information

## Binary Format

The seed format consists of:

1. **Header** (40 bytes):
   - Number of holes (external references)
   - Number of big nats (multi-word)
   - Number of word-sized nats
   - Number of byte-sized nats
   - Number of fragments

2. **Big nat sizes**: Array of word counts for each big nat

3. **Atom data**: 
   - Big nats (largest first)
   - Words
   - Bytes

4. **Tree data**: Bit-packed fragment structure

## Deduplication

The library automatically deduplicates:

- Identical atoms (natural numbers)
- Identical interior nodes (cons cells with same head and tail)
- Shared subtrees are extracted as fragments based on reference counts

## Performance

The library uses hash tables for O(1) average-case deduplication lookups.
Memory usage scales linearly with the number of unique nodes.

## License

BSD-3-Clause (same as the original Plunder implementation)

## Credits

This is a Rust port of the seed.c implementation from the Plunder project.
