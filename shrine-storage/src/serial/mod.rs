//! Seed serialization library for tree structures with deduplication
//!
//! This is a Rust port of the Plunder seed.c implementation.
//! It provides efficient serialization of tree structures with:
//! - Leaf deduplication via hash tables
//! - Interior node deduplication
//! - Fragment-based compression for repeated subtrees
//! - Efficient bit-packed serialization

use std::collections::HashMap;
use thiserror::Error;

mod hash;
mod serialize;

pub use hash::fmix64;

/// Error types for seed operations
#[derive(Error, Debug)]
pub enum SeedError {
    #[error("Input buffer too small: expected at least {expected} bytes, got {actual}")]
    BufferTooSmall { expected: usize, actual: usize },

    #[error("Invalid seed format: {0}")]
    InvalidFormat(String),

    #[error("Leaf value out of bounds: {0}")]
    LeafOutOfBounds(u64),

    #[error("Empty seed context")]
    EmptyContext,

    #[error("No fragments but multiple leaves")]
    NoFragsMultipleLeaves,

    #[error("Invalid indirect nat: most significant word is zero")]
    InvalidIndirectNat,
}

pub type Result<T> = std::result::Result<T, SeedError>;

/// Index into the treenodes array
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct TreeNodeId(pub u32);

/// Index into the nats array
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct NatId(pub u32);

/// Index into the frags array
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct FragId(pub u32);

/// A leaf value (natural number)
#[derive(Clone, Debug)]
pub struct Leaf {
    /// Most significant word
    pub msw: u64,
    /// Number of extra words (0 for small values)
    pub nex: u64,
    /// Buffer containing extra words (if nex > 0)
    pub buf: Option<Vec<u64>>,
    /// Hash value for deduplication
    pub hax: u64,
}

impl Leaf {
    /// Create a new leaf from a single word
    pub fn from_word(word: u64) -> Self {
        Leaf {
            msw: word,
            nex: 0,
            buf: None,
            hax: fmix64(word),
        }
    }

    /// Check if two leaves are equal
    pub fn equals(&self, other: &Leaf) -> bool {
        if self.hax != other.hax || self.msw != other.msw || self.nex != other.nex {
            return false;
        }
        if self.nex == 0 {
            return true;
        }
        match (&self.buf, &other.buf) {
            (Some(a), Some(b)) => a[..self.nex as usize] == b[..self.nex as usize],
            _ => false,
        }
    }
}

/// Value stored in a tree node
#[derive(Clone, Copy, Debug)]
pub enum TreeNodeValue {
    /// A pair of tree nodes (interior node)
    Pair { head: TreeNodeId, tail: TreeNodeId },
    /// A natural number leaf
    Nat(NatId),
    /// An external reference (hole/pin)
    Pin(u32),
    /// A reference to a fragment
    Frag(FragId),
}

impl TreeNodeValue {
    /// Pack the value into a u64 for storage
    fn pack(&self) -> u64 {
        match self {
            TreeNodeValue::Pair { head, tail } => {
                ((head.0 as u64) << 32) | (tail.0 as u64)
            }
            TreeNodeValue::Nat(nat) => (nat.0 as u64) | (6u64 << 61),
            TreeNodeValue::Pin(pin) => (*pin as u64) | (5u64 << 61),
            TreeNodeValue::Frag(frag) => (frag.0 as u64) | (7u64 << 61),
        }
    }

    /// Unpack a u64 into a TreeNodeValue
    fn unpack(word: u64) -> Self {
        let tag = word >> 61;
        let payload = word & ((1u64 << 32) - 1);

        match tag {
            5 => TreeNodeValue::Pin(payload as u32),
            6 => TreeNodeValue::Nat(NatId(payload as u32)),
            7 => TreeNodeValue::Frag(FragId(payload as u32)),
            _ => {
                // It's a pair (tag bits are 0)
                let head = (word >> 32) as u32;
                let tail = word as u32;
                TreeNodeValue::Pair {
                    head: TreeNodeId(head),
                    tail: TreeNodeId(tail),
                }
            }
        }
    }

    /// Check if this is a back-reference (atom, pin, or fragment)
    fn is_backref(&self) -> bool {
        !matches!(self, TreeNodeValue::Pair { .. })
    }

    /// Get the tag value (for serialization)
    fn _tag(&self) -> u64 {
        match self {
            TreeNodeValue::Pair { .. } => 0,
            TreeNodeValue::Pin(_) => 5,
            TreeNodeValue::Nat(_) => 6,
            TreeNodeValue::Frag(_) => 7,
        }
    }
}

/// A fragment (deduplicated subtree)
#[derive(Clone, Copy, Debug)]
pub struct FragVal {
    pub head: TreeNodeId,
    pub tail: TreeNodeId,
    pub leaves: u32,
}

/// Entry in the leaves deduplication table
#[derive(Clone)]
struct LeafEntry {
    leaf: Leaf,
    ptr: TreeNodeId,
}

/// Entry in the nodes deduplication table
#[allow(dead_code)]
#[derive(Clone, Copy)]
struct NodeEntry {
    val: u64,
    hax: u32,
    ptr: TreeNodeId,
}

/// The main seed context
pub struct Seed {
    /// Tree node values
    treenodes: Vec<u64>,
    /// Reference counts for each node
    refcounts: Vec<u32>,
    /// Tree depth at each node
    depths: Vec<u32>,

    /// Number of external references (holes)
    holes_count: u32,

    /// Array of unique nat leaves
    nats: Vec<Leaf>,

    /// Leaf deduplication table
    leaves_table: HashMap<u64, Vec<LeafEntry>>,

    /// Interior node deduplication table
    nodes_table: HashMap<u32, Vec<NodeEntry>>,

    /// Array of fragments
    frags: Vec<FragVal>,

    /// Ordering for serialization (sorted by size, descending)
    ordering: Vec<u32>,
    /// Reverse ordering
    rev_ordering: Vec<u32>,

    /// Count of byte-sized nats
    num_bytes: i32,
    /// Count of word-sized nats
    num_words: i32,
}

impl Default for Seed {
    fn default() -> Self {
        Self::new()
    }
}

impl Seed {
    /// Create a new seed context
    pub fn new() -> Self {
        Seed {
            treenodes: Vec::with_capacity(64),
            refcounts: Vec::with_capacity(64),
            depths: Vec::with_capacity(64),
            holes_count: 0,
            nats: Vec::with_capacity(16),
            leaves_table: HashMap::new(),
            nodes_table: HashMap::new(),
            frags: Vec::with_capacity(32),
            ordering: Vec::new(),
            rev_ordering: Vec::new(),
            num_bytes: 0,
            num_words: 0,
        }
    }

    /// Clear the context for reuse
    pub fn wipe(&mut self) {
        self.treenodes.clear();
        self.refcounts.clear();
        self.depths.clear();
        self.holes_count = 0;
        self.nats.clear();
        self.leaves_table.clear();
        self.nodes_table.clear();
        self.frags.clear();
        self.ordering.clear();
        self.rev_ordering.clear();
        self.num_bytes = 0;
        self.num_words = 0;
    }

    /// Allocate a new tree node
    fn alloc_treenode(&mut self, val: TreeNodeValue, depth: u32) -> TreeNodeId {
        let idx = self.treenodes.len() as u32;
        self.treenodes.push(val.pack());
        self.refcounts.push(1);
        self.depths.push(depth);
        TreeNodeId(idx)
    }

    /// Get the value of a tree node
    fn get_treenode(&self, id: TreeNodeId) -> TreeNodeValue {
        TreeNodeValue::unpack(self.treenodes[id.0 as usize])
    }

    /// Set the value of a tree node
    fn set_treenode(&mut self, id: TreeNodeId, val: TreeNodeValue) {
        self.treenodes[id.0 as usize] = val.pack();
    }

    /// Allocate a new nat
    fn alloc_nat(&mut self) -> NatId {
        let idx = self.nats.len() as u32;
        self.nats.push(Leaf {
            msw: 0,
            nex: 0,
            buf: None,
            hax: 0,
        });
        NatId(idx)
    }

    /// Create a hole (external reference)
    pub fn hole(&mut self) -> TreeNodeId {
        let i = self.holes_count;
        self.holes_count += 1;
        let val = TreeNodeValue::Pin(i);
        self.alloc_treenode(val, 0)
    }

    /// Insert a leaf, deduplicating if already present
    fn insert_leaf(&mut self, leaf: Leaf) -> TreeNodeId {
        // Check for existing entry first (immutable borrow)
        if let Some(entries) = self.leaves_table.get(&leaf.hax) {
            for entry in entries.iter() {
                if entry.leaf.equals(&leaf) {
                    let ptr = entry.ptr;
                    self.refcounts[ptr.0 as usize] += 1;
                    return ptr;
                }
            }
        }

        // Create new entry (no outstanding borrows here)
        let hax = leaf.hax;
        let nat = self.alloc_nat();
        self.nats[nat.0 as usize] = leaf.clone();
        let ptr = self.alloc_treenode(TreeNodeValue::Nat(nat), 0);

        self.leaves_table.entry(hax).or_default().push(LeafEntry {
            leaf,
            ptr,
        });

        ptr
    }

    /// Insert a word (64-bit natural number)
    pub fn word(&mut self, word: u64) -> TreeNodeId {
        let leaf = Leaf::from_word(word);
        self.insert_leaf(leaf)
    }

    /// Insert a natural number from a byte array
    pub fn barnat(&mut self, bytes: &[u8]) -> TreeNodeId {
        if bytes.len() < 8 {
            let mut word = 0u64;
            let ptr = &mut word as *mut u64 as *mut u8;
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
                *ptr.add(bytes.len()) = 1;
            }
            return self.word(word);
        }

        // Width in bytes (including implicit 1 byte at end)
        let width_bytes = bytes.len() + 1;
        let overflow = width_bytes % 8;
        let wid = (width_bytes / 8) + if overflow > 0 { 1 } else { 0 };
        let nex = wid - 1;

        // Build the most significant word
        let mut msw = 0u64;
        let num = if overflow > 0 { overflow - 1 } else { 7 };
        let src_offset = nex * 8;

        unsafe {
            let dst = &mut msw as *mut u64 as *mut u8;
            if src_offset < bytes.len() {
                let copy_len = std::cmp::min(num, bytes.len() - src_offset);
                std::ptr::copy_nonoverlapping(bytes.as_ptr().add(src_offset), dst, copy_len);
            }
            *dst.add(num) = 1;
        }

        // Copy the buffer words
        let mut buf = vec![0u64; nex];
        unsafe {
            std::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                buf.as_mut_ptr() as *mut u8,
                std::cmp::min(nex * 8, bytes.len()),
            );
        }

        let hax = fmix64(msw) ^ fmix64(nex as u64) ^ hash::xxh3_64bits(&bytes[..nex * 8]);

        let leaf = Leaf {
            msw,
            nex: nex as u64,
            buf: Some(buf),
            hax,
        };

        self.insert_leaf(leaf)
    }

    /// Insert a natural number from a word array
    pub fn nat(&mut self, words: &[u64]) -> Result<TreeNodeId> {
        if words.is_empty() {
            return Ok(self.word(0));
        }

        if words.len() == 1 {
            return Ok(self.word(words[0]));
        }

        let nex = words.len() - 1;
        let msw = words[nex];

        if msw == 0 {
            return Err(SeedError::InvalidIndirectNat);
        }

        let buf: Vec<u64> = words[..nex].to_vec();
        let hax = fmix64(msw)
            ^ fmix64(nex as u64)
            ^ hash::xxh3_64bits(unsafe {
                std::slice::from_raw_parts(buf.as_ptr() as *const u8, nex * 8)
            });

        let leaf = Leaf {
            msw,
            nex: nex as u64,
            buf: Some(buf),
            hax,
        };

        Ok(self.insert_leaf(leaf))
    }

    /// Create a cons cell (pair)
    pub fn cons(&mut self, hed: TreeNodeId, tel: TreeNodeId) -> TreeNodeId {
        let val = TreeNodeValue::Pair { head: hed, tail: tel };
        let packed = val.pack();
        let hax = fmix64(packed) as u32;

        // Check for existing entry first (immutable borrow)
        if let Some(entries) = self.nodes_table.get(&hax) {
            for entry in entries.iter() {
                if entry.val == packed {
                    let ptr = entry.ptr;
                    self.refcounts[ptr.0 as usize] += 1;
                    return ptr;
                }
            }
        }

        // Create new entry (no outstanding borrows here)
        let depth = 1 + std::cmp::max(
            self.depths[hed.0 as usize],
            self.depths[tel.0 as usize],
        );
        let ptr = self.alloc_treenode(val, depth);

        self.nodes_table.entry(hax).or_default().push(NodeEntry {
            val: packed,
            hax,
            ptr,
        });

        ptr
    }

    /// Touch a tree node (bump reference counts for shatter)
    pub fn touch(&mut self, x: TreeNodeId) {
        let depth = self.depths[x.0 as usize];
        let mut stack = Vec::with_capacity((depth + 1) as usize);
        stack.push(x);

        while let Some(node) = stack.pop() {
            let val = self.get_treenode(node);

            if let TreeNodeValue::Pair { head, tail } = val {
                self.refcounts[node.0 as usize] += 1;
                stack.push(head);
                stack.push(tail);
            }
        }
    }

    /// Allocate a new fragment
    fn alloc_frag(&mut self, frag: FragVal) -> FragId {
        let idx = self.frags.len() as u32;
        self.frags.push(frag);
        FragId(idx)
    }

    /// Shatter the tree into fragments based on reference counts
    fn shatter(&mut self, top: TreeNodeId) {
        if top.0 == 0 {
            return; // Just a leaf
        }

        #[derive(Clone, Copy)]
        struct ShatterFrame {
            ix: u32,
            refs: u32,
            leaves: u32,
        }

        let stksz = (self.depths[top.0 as usize] * 2 + 2) as usize;
        let mut stk: Vec<ShatterFrame> = vec![
            ShatterFrame {
                ix: 0,
                refs: 0,
                leaves: 0
            };
            stksz
        ];

        let end = stksz - 1;
        let mut sp = end;

        stk[sp].ix = top.0;
        stk[sp].leaves = 0;

        loop {
            // Unprocessed node
            if stk[sp].leaves == 0 {
                let val = self.get_treenode(TreeNodeId(stk[sp].ix));

                if val.is_backref() {
                    // Leaf
                    stk[sp].leaves = 1;
                    continue;
                }

                // Cell
                if let TreeNodeValue::Pair { head, tail } = val {
                    stk[sp].refs = self.refcounts[stk[sp].ix as usize];
                    stk[sp - 1].ix = tail.0;
                    stk[sp - 1].leaves = 0;
                    stk[sp - 2].ix = head.0;
                    stk[sp - 2].leaves = 0;
                    sp -= 2;
                    continue;
                }
            }

            // sp[1] not processed, swap
            if sp < end && stk[sp + 1].leaves == 0 {
                stk.swap(sp, sp + 1);
                continue;
            }

            // Both processed, combine
            if sp + 2 <= end {
                let hed_leaves = stk[sp].leaves;
                let tel_leaves = stk[sp + 1].leaves;
                let _cel_ix = stk[sp + 2].ix;

                stk[sp + 2].leaves = tel_leaves + hed_leaves;
                sp += 2;

                // Top node, or more refs than parent
                if sp == end || stk[sp].refs > stk[sp + 2].refs {
                    let val = self.get_treenode(TreeNodeId(stk[sp].ix));

                    if let TreeNodeValue::Pair { head, tail } = val {
                        let frag = self.alloc_frag(FragVal {
                            head,
                            tail,
                            leaves: stk[sp].leaves,
                        });

                        self.set_treenode(TreeNodeId(stk[sp].ix), TreeNodeValue::Frag(frag));
                        stk[sp].leaves = 1;
                    }
                }

                if sp == end {
                    return;
                }

                continue;
            }

            break;
        }
    }

    /// Finalize the seed context for serialization
    pub fn done(&mut self) -> Result<()> {
        let num = self.treenodes.len();

        if num == 0 {
            return Err(SeedError::EmptyContext);
        }

        let nats = self.nats.len();
        let top = TreeNodeId((num - 1) as u32);

        self.shatter(top);

        // Build ordering (sorted by size, descending)
        self.ordering = (0..nats as u32).collect();
        self.ordering.sort_by(|&a, &b| {
            let la = &self.nats[a as usize];
            let lb = &self.nats[b as usize];

            // Compare by nex (descending)
            match lb.nex.cmp(&la.nex) {
                std::cmp::Ordering::Equal => {}
                ord => return ord,
            }

            // Compare by msw (descending)
            match lb.msw.cmp(&la.msw) {
                std::cmp::Ordering::Equal => {}
                ord => return ord,
            }

            // Compare buffers if needed
            if la.nex > 0 {
                if let (Some(ba), Some(bb)) = (&la.buf, &lb.buf) {
                    for i in (0..la.nex as usize).rev() {
                        match bb[i].cmp(&ba[i]) {
                            std::cmp::Ordering::Equal => continue,
                            ord => return ord,
                        }
                    }
                }
            }

            std::cmp::Ordering::Equal
        });

        // Build reverse ordering
        self.rev_ordering = vec![0; nats];
        for (i, &idx) in self.ordering.iter().enumerate() {
            self.rev_ordering[idx as usize] = i as u32;
        }

        // Count bytes and words
        let mut num_bytes = 0i32;
        let mut num_words = 0i32;
        for nat in &self.nats {
            if nat.nex == 0 {
                if nat.msw < 256 {
                    num_bytes += 1;
                } else {
                    num_words += 1;
                }
            }
        }
        self.num_bytes = num_bytes;
        self.num_words = num_words;

        Ok(())
    }

    /// Calculate the serialized size
    pub fn size(&self) -> usize {
        serialize::seed_size(self)
    }

    /// Serialize to a buffer
    pub fn save(&self, buf: &mut [u8]) -> usize {
        serialize::seed_save(self, buf)
    }

    /// Load from a buffer
    pub fn load(&mut self, buf: &[u8]) -> Result<()> {
        serialize::seed_load(self, buf)
    }

    /// Get the number of tree nodes
    pub fn treenode_count(&self) -> usize {
        self.treenodes.len()
    }

    /// Get the number of nats
    pub fn nat_count(&self) -> usize {
        self.nats.len()
    }

    /// Get the number of fragments
    pub fn frag_count(&self) -> usize {
        self.frags.len()
    }

    /// Get the number of holes
    pub fn hole_count(&self) -> u32 {
        self.holes_count
    }

    /// Print the tree (for debugging)
    pub fn show(&self) {
        let frags = self.frags.len();

        if frags > 0 {
            self.print_fragment(FragId((frags - 1) as u32));
        } else {
            if self.treenodes.is_empty() {
                println!("No frags and no leaves");
                return;
            }

            if self.treenodes.len() > 1 {
                println!("No frags but {} leaves. Nonsense!", self.treenodes.len());
                return;
            }

            self.print_tree(TreeNodeId(0));
        }

        println!();
    }

    /// Print a nat
    fn print_nat(&self, nat: NatId) {
        let l = &self.nats[nat.0 as usize];

        if l.nex == 0 && l.msw < 256 {
            print!("{}", l.msw);
            return;
        }

        if l.nex == 0 {
            print!("{}", l.msw);
        } else {
            print!("0x{:x}", l.msw);
            if let Some(buf) = &l.buf {
                for i in (0..l.nex as usize).rev() {
                    print!(".{:016x}", buf[i]);
                }
            }
        }
    }

    /// Print a tree node
    fn print_tree(&self, tree: TreeNodeId) {
        let val = self.get_treenode(tree);

        match val {
            TreeNodeValue::Pin(p) => print!("p{}", p),
            TreeNodeValue::Nat(n) => self.print_nat(n),
            TreeNodeValue::Frag(f) => self.print_fragment(f),
            TreeNodeValue::Pair { head, tail } => {
                print!("(");
                self.print_tree_list(head);
                print!(" ");
                self.print_tree(tail);
                print!(")");
            }
        }
    }

    /// Print a tree as a list
    fn print_tree_list(&self, tree: TreeNodeId) {
        let val = self.get_treenode(tree);

        match val {
            TreeNodeValue::Pin(p) => print!("p{}", p),
            TreeNodeValue::Nat(n) => self.print_nat(n),
            TreeNodeValue::Frag(f) => {
                let frag = &self.frags[f.0 as usize];
                self.print_tree_list(frag.head);
                print!(" ");
                self.print_tree(frag.tail);
            }
            TreeNodeValue::Pair { head, tail } => {
                self.print_tree_list(head);
                print!(" ");
                self.print_tree(tail);
            }
        }
    }

    /// Print a fragment
    fn print_fragment(&self, frag: FragId) {
        let f = &self.frags[frag.0 as usize];
        print!("(");
        self.print_tree_list(f.head);
        print!(" ");
        self.print_tree(f.tail);
        print!(")");
    }

    /// Debug dump of the context
    pub fn debug(&self) {
        println!("\nseed_debug():");

        println!(
            "\n\tleaves: (count={})\n",
            self.leaves_table.values().map(|v| v.len()).sum::<usize>()
        );

        println!(
            "\n\tnodes: (count={})\n",
            self.nodes_table.values().map(|v| v.len()).sum::<usize>()
        );

        println!("\n\tnats: (count={})\n", self.nats.len());
        for (i, _) in self.nats.iter().enumerate() {
            print!("\t\tn{} = ", i);
            self.print_nat(NatId(i as u32));
            println!();
        }

        println!("\n\ttreenodes: (count={})\n", self.treenodes.len());
        for i in 0..self.treenodes.len() {
            let refcount = self.refcounts[i];
            let val = self.get_treenode(TreeNodeId(i as u32));
            match val {
                TreeNodeValue::Pin(p) => {
                    println!("\t\tt{} = p{}\t\trefs={}", i, p, refcount);
                }
                TreeNodeValue::Nat(n) => {
                    println!("\t\tt{} = n{}\t\trefs={}", i, n.0, refcount);
                }
                TreeNodeValue::Frag(f) => {
                    println!("\t\tt{} = f{}\t\trefs={}", i, f.0, refcount);
                }
                TreeNodeValue::Pair { head, tail } => {
                    let depth = self.depths[i];
                    println!(
                        "\t\tt{} = ({}, {})\trefs={}\tdeep={}",
                        i, head.0, tail.0, refcount, depth
                    );
                }
            }
        }

        println!("\nSeed Fragments:\n");
        for (i, frag) in self.frags.iter().enumerate() {
            println!("\tFragment[{}]:\n", i);
            println!("\t\t(t{}, t{}) = ", frag.head.0, frag.tail.0);
            self.print_fragment(FragId(i as u32));
            println!("\n");
        }
    }
}

/// Calculate the bit width needed to represent a value
#[inline]
pub fn word64_bits(word: u64) -> u32 {
    if word == 0 {
        0
    } else {
        64 - word.leading_zeros()
    }
}

/// Internal serialization state (matches struct ser in C)
#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct SerState<'a> {
    pub buf: &'a [u8],
    pub wid: usize,
}

/// Calculate the byte width needed to represent a value
pub fn word64_bytes(word: u64) -> u32 {
    if word == 0 {
        1
    } else {
        (64 - word.leading_zeros() + 7) / 8
    }
}

#[cfg(test)]
mod integration_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_word64_bits() {
        assert_eq!(word64_bits(0), 0);
        assert_eq!(word64_bits(1), 1);
        assert_eq!(word64_bits(2), 2);
        assert_eq!(word64_bits(3), 2);
        assert_eq!(word64_bits(255), 8);
        assert_eq!(word64_bits(256), 9);
    }

    #[test]
    fn test_simple_word() {
        let mut seed = Seed::new();
        let node = seed.word(42);
        assert_eq!(node.0, 0);
        assert_eq!(seed.nat_count(), 1);
    }

    #[test]
    fn test_dedup() {
        let mut seed = Seed::new();
        let a = seed.word(42);
        let b = seed.word(42);
        assert_eq!(a.0, b.0);
        assert_eq!(seed.nat_count(), 1);
    }

    #[test]
    fn test_cons() {
        let mut seed = Seed::new();
        let a = seed.word(1);
        let b = seed.word(2);
        let c = seed.cons(a, b);
        assert_eq!(c.0, 2);
        assert_eq!(seed.treenode_count(), 3);
    }

    #[test]
    fn test_round_trip() {
        let mut seed = Seed::new();
        let a = seed.word(1);
        let b = seed.word(2);
        let c = seed.cons(a, b);
        let d = seed.word(3);
        let _e = seed.cons(c, d);

        seed.done().unwrap();

        let size = seed.size();
        let mut buf = vec![0u8; size];
        let written = seed.save(&mut buf);
        assert_eq!(written, size);

        let mut seed2 = Seed::new();
        seed2.load(&buf).unwrap();

        assert_eq!(seed2.nat_count(), seed.nat_count());
    }
}
