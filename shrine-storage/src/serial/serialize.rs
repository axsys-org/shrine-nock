//! Serialization and deserialization for seed format

use super::{
    word64_bits, FragId, FragVal, Leaf, NatId, Result, Seed, SeedError, TreeNodeId, TreeNodeValue,
};

/// Calculate the serialized size of a seed
pub fn seed_size(ctx: &Seed) -> usize {
    let num_holes = ctx.holes_count as u64;
    let num_nats = ctx.nats.len() as u64;
    let num_bytes = ctx.num_bytes as u64;
    let num_words = ctx.num_words as u64;
    let num_bigs = num_nats - (num_bytes + num_words);
    let num_frags = ctx.frags.len() as u64;

    // The backreferences table starts off holding all "external
    // references" and all atoms.
    let mut refrs = num_holes + num_nats;

    // Header (40 bytes) + bignat sizes + all bytes + all words + word-width of each bignat
    let mut width = 40 + num_bytes + num_words * 8 + num_bigs * 8;

    // Add the actual bignat data
    for j in 0..num_bigs as usize {
        let ix = ctx.ordering[j] as usize;
        let w = ctx.nats[ix].nex + 1;
        width += 8 * w;
    }

    let mut treebits: u64 = 0;

    for i in 0..num_frags as usize {
        let max_ref = refrs - 1;
        let leaf_width = word64_bits(max_ref) as u64;

        // Calculate the size of the fragment
        // Each leaf requires leaf_width bits
        // Each leaf requires a single tag bit
        // Each interior node requires a single tag bit
        // There are (num_leaves - 1) interior nodes
        // Every fragment is a cell, so outermost cell doesn't need a tag
        let leaves = ctx.frags[i].leaves as u64;
        let frag_bits = (leaves * leaf_width) + (leaves * 2) - 2;

        treebits += frag_bits;
        refrs += 1;
    }

    // Tree-bits is padded to be a multiple of 8
    let hanging_bits = treebits % 8;
    if hanging_bits > 0 {
        treebits += 8 - hanging_bits;
    }

    width += treebits / 8;

    // Result is always a multiple of 8
    let hanging_bytes = width % 8;
    if hanging_bytes > 0 {
        width += 8 - hanging_bytes;
    }

    width as usize
}

/// State for fragment serialization
struct FragState<'a> {
    acc: u64,
    fil: u64,
    out: &'a mut [u64],
    out_idx: usize,
    stack: Vec<TreeNodeId>,
    num_nats: u32,
    num_holes: u32,
    refbits: i32,
}

impl<'a> FragState<'a> {
    fn write_word(&mut self, word: u64) {
        self.out[self.out_idx] = word;
        self.out_idx += 1;
    }
}

/// Serialize a single fragment
fn serialize_frag(ctx: &Seed, st: &mut FragState, frag: &FragVal) {
    st.stack.clear();
    st.stack.push(frag.tail);
    st.stack.push(frag.head);

    let num_leaves = st.num_nats + st.num_holes;
    let refbits = st.refbits;

    // Left-recursion depth tracker
    let mut deep = 0i32;

    while let Some(treeidx) = st.stack.pop() {
        let val = ctx.get_treenode(treeidx);

        // If this is a node, push the tail and recurse into head
        if let TreeNodeValue::Pair { head, tail } = val {
            deep += 1;
            st.stack.push(tail);
            st.stack.push(head);
            continue;
        }

        // Output `deep` one bits
        while deep > 0 {
            let remain = 64 - st.fil as i32;

            if deep < remain {
                st.acc |= ((1u64 << deep) - 1) << st.fil;
                st.fil += deep as u64;
                deep = 0;
                break;
            }

            // Fill rest of accumulator with ones
            st.write_word(st.acc | (u64::MAX << st.fil));
            st.acc = 0;
            st.fil = 0;
            deep -= remain;
        }

        // Calculate the leaf reference
        let bits: u64 = match val {
            TreeNodeValue::Pin(p) => p as u64,
            TreeNodeValue::Nat(n) => st.num_holes as u64 + ctx.rev_ordering[n.0 as usize] as u64,
            TreeNodeValue::Frag(f) => num_leaves as u64 + f.0 as u64,
            TreeNodeValue::Pair { .. } => unreachable!(),
        };

        // Left-shift by one to create a zero-tag at the front
        let bits = bits << 1;
        let new_bits = bits << st.fil;
        let overflow = if st.fil > 0 { bits >> (64 - st.fil) } else { 0 };
        st.acc |= new_bits;
        st.fil += (refbits + 1) as u64;

        // If the leaf data doesn't fit, output and use overflow
        if st.fil >= 64 {
            st.write_word(st.acc);
            st.acc = overflow;
            st.fil -= 64;
        }
    }
}

/// Save the seed to a buffer
pub fn seed_save(ctx: &Seed, buf: &mut [u8]) -> usize {
    let num_holes = ctx.holes_count;
    let num_nats = ctx.nats.len() as u32;
    let num_bytes = ctx.num_bytes as u64;
    let num_words = ctx.num_words as u64;
    let num_bigs = num_nats as u64 - (num_bytes + num_words);
    let num_frags = ctx.frags.len() as u64;

    // Write header (40 bytes = 5 u64s)
    let header: &mut [u64] =
        unsafe { std::slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut u64, 5 + num_bigs as usize) };

    header[0] = num_holes as u64;
    header[1] = num_bigs;
    header[2] = num_words;
    header[3] = num_bytes;
    header[4] = num_frags;

    // Write bignat sizes
    for i in 0..num_bigs as usize {
        let ix = ctx.ordering[i] as usize;
        let nw = ctx.nats[ix].nex + 1;
        header[5 + i] = nw;
    }

    let mut out_offset = 40 + (num_bigs as usize * 8);

    // Write bignat data
    for i in 0..num_bigs as usize {
        let ix = ctx.ordering[i] as usize;
        let leaf = &ctx.nats[ix];

        if let Some(ref buf_data) = leaf.buf {
            let bytes = leaf.nex as usize * 8;
            unsafe {
                std::ptr::copy_nonoverlapping(
                    buf_data.as_ptr() as *const u8,
                    buf.as_mut_ptr().add(out_offset),
                    bytes,
                );
            }
        }

        out_offset += (leaf.nex as usize + 1) * 8;

        // Write msw at the end
        let msw_offset = out_offset - 8;
        buf[msw_offset..msw_offset + 8].copy_from_slice(&leaf.msw.to_le_bytes());
    }

    // Write words
    for i in 0..num_words as usize {
        let ix = ctx.ordering[i + num_bigs as usize] as usize;
        let word = ctx.nats[ix].msw;
        buf[out_offset..out_offset + 8].copy_from_slice(&word.to_le_bytes());
        out_offset += 8;
    }

    // Write bytes
    for i in 0..num_bytes as usize {
        let ix = ctx.ordering[i + num_bigs as usize + num_words as usize] as usize;
        buf[out_offset + i] = ctx.nats[ix].msw as u8;
    }
    out_offset += num_bytes as usize;

    if num_frags == 0 {
        let width = out_offset;
        let hanging = width % 8;
        if hanging > 0 {
            return width + (8 - hanging);
        }
        return width;
    }

    // Handle fragment serialization
    let clif = out_offset % 8;
    out_offset -= clif;

    // Read current state of first word
    let words_ptr = unsafe { buf.as_mut_ptr().add(out_offset) as *mut u64 };
    let initial_acc = unsafe { *words_ptr };

    let remaining_words = (buf.len() - out_offset) / 8;
    let out_slice = unsafe { std::slice::from_raw_parts_mut(words_ptr, remaining_words) };

    let mut st = FragState {
        acc: initial_acc,
        fil: (clif * 8) as u64,
        out: out_slice,
        out_idx: 0,
        stack: Vec::with_capacity(64),
        num_nats,
        num_holes,
        refbits: 0,
    };

    let max_ref = (num_holes + num_nats) as i32 - 1;
    st.refbits = word64_bits(max_ref as u64) as i32;

    let mut until_cliff = (1 << st.refbits) - (max_ref + 1);
    let mut current_max_ref = max_ref;

    for i in 0..num_frags as usize {
        serialize_frag(ctx, &mut st, &ctx.frags[i]);

        current_max_ref += 1;
        if until_cliff == 0 {
            st.refbits += 1;
            until_cliff = current_max_ref;
        }
        until_cliff -= 1;
    }

    // Flush remaining accumulator
    if st.fil > 0 {
        st.write_word(st.acc);
    }

    out_offset + st.out_idx * 8
}

/// State for fragment loading
struct FragLoadState<'a> {
    ctx: &'a mut Seed,
    ptr: &'a [u64],
    ptr_idx: usize,
    rem: i32,
    acc: u64,
    red: u64,
    ref_bits: u64,
    max_ref: u64,
    num_holes: u64,
    num_nats: u64,
}

struct FragResult {
    tree: TreeNodeId,
    leaves: u32,
}

/// Load a fragment tree
fn load_fragtree(s: &mut FragLoadState) -> Result<FragResult> {
    let refbits = s.ref_bits as i32;

    let bit = (s.acc >> s.red) & 1;
    s.red = (s.red + 1) % 64;

    if s.red == 0 {
        if s.rem <= 0 {
            return Err(SeedError::InvalidFormat("not enough space".to_string()));
        }
        s.rem -= 1;
        s.acc = s.ptr[s.ptr_idx];
        s.ptr_idx += 1;
    }

    if bit == 1 {
        // Cell - load both children
        let fv = load_fragment(s)?;
        let depth = 1 + std::cmp::max(
            s.ctx.depths[fv.head.0 as usize],
            s.ctx.depths[fv.tail.0 as usize],
        );
        let tr = s
            .ctx
            .alloc_treenode(TreeNodeValue::Pair { head: fv.head, tail: fv.tail }, depth);
        return Ok(FragResult {
            tree: tr,
            leaves: fv.leaves,
        });
    }

    // Leaf - read refbits
    let leaf_mask = (1u64 << refbits) - 1;
    let mut leaf = (s.acc >> s.red) & leaf_mask;

    let oldred = s.red as i32;
    s.red += refbits as u64;

    if s.red >= 64 {
        let extra = (oldred + refbits) - 64;
        let already = refbits - extra;

        if s.ptr_idx < s.ptr.len() {
            let nex = s.ptr[s.ptr_idx];
            let why = nex & ((1u64 << extra) - 1);
            let more = why << already;
            leaf |= more;

            s.red -= 64;
            s.acc = nex;
            s.rem -= 1;
            s.ptr_idx += 1;
        }
    }

    if leaf > s.max_ref {
        return Err(SeedError::LeafOutOfBounds(leaf));
    }

    let (v, is_frag) = if leaf < s.num_holes {
        (TreeNodeValue::Pin(leaf as u32), false)
    } else {
        let leaf = leaf - s.num_holes;
        if leaf < s.num_nats {
            (TreeNodeValue::Nat(NatId(leaf as u32)), false)
        } else {
            let leaf = leaf - s.num_nats;
            (TreeNodeValue::Frag(FragId(leaf as u32)), true)
        }
    };

    let depth = if is_frag {
        if let TreeNodeValue::Frag(frag_id) = v {
            let fv = &s.ctx.frags[frag_id.0 as usize];
            1 + std::cmp::max(
                s.ctx.depths[fv.head.0 as usize],
                s.ctx.depths[fv.tail.0 as usize],
            )
        } else {
            0
        }
    } else {
        0
    };

    let t = s.ctx.alloc_treenode(v, depth);

    Ok(FragResult { tree: t, leaves: 0 })
}

/// Load a fragment (head and tail)
fn load_fragment(s: &mut FragLoadState) -> Result<FragVal> {
    let hed = load_fragtree(s)?;
    let tel = load_fragtree(s)?;

    Ok(FragVal {
        head: hed.tree,
        tail: tel.tree,
        leaves: hed.leaves + tel.leaves,
    })
}

/// Load a seed from a buffer
pub fn seed_load(ctx: &mut Seed, buf: &[u8]) -> Result<()> {
    if buf.len() < 40 {
        return Err(SeedError::BufferTooSmall {
            expected: 40,
            actual: buf.len(),
        });
    }

    if buf.len() % 8 != 0 {
        return Err(SeedError::InvalidFormat(
            "buffer must be a multiple of 8 bytes".to_string(),
        ));
    }

    let header: &[u64] = unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const u64, 5) };

    let num_holes = header[0];
    let num_bigs = header[1];
    let num_words = header[2];
    let num_bytes = header[3];
    let num_frags = header[4];

    // Set up holes
    for _ in 0..num_holes {
        ctx.hole();
    }

    let header_size = 40 + (8 * num_bigs as usize);

    if buf.len() < header_size {
        return Err(SeedError::BufferTooSmall {
            expected: header_size,
            actual: buf.len(),
        });
    }

    // Read bignat widths
    let bigwidths: Vec<u64> = (0..num_bigs as usize)
        .map(|i| {
            let offset = 40 + i * 8;
            u64::from_le_bytes(buf[offset..offset + 8].try_into().unwrap())
        })
        .collect();

    let mut offset = header_size;

    // Load bignats
    for i in 0..num_bigs as usize {
        let wid = bigwidths[i] as usize;
        let nix = ctx.alloc_nat();
        let nex = wid - 1;

        let mut buf_data = vec![0u64; nex];
        if nex > 0 {
            unsafe {
                std::ptr::copy_nonoverlapping(
                    buf.as_ptr().add(offset),
                    buf_data.as_mut_ptr() as *mut u8,
                    nex * 8,
                );
            }
        }

        let msw = u64::from_le_bytes(buf[offset + nex * 8..offset + wid * 8].try_into().unwrap());

        ctx.nats[nix.0 as usize] = Leaf {
            msw,
            nex: nex as u64,
            buf: if nex > 0 { Some(buf_data) } else { None },
            hax: 0,
        };

        offset += wid * 8;
    }

    // Load words
    for _ in 0..num_words {
        let word = u64::from_le_bytes(buf[offset..offset + 8].try_into().unwrap());
        let nix = ctx.alloc_nat();
        ctx.nats[nix.0 as usize] = Leaf {
            msw: word,
            nex: 0,
            buf: None,
            hax: 0,
        };
        offset += 8;
    }

    // Load bytes
    for i in 0..num_bytes as usize {
        let byte = buf[offset + i] as u64;
        let nix = ctx.alloc_nat();
        ctx.nats[nix.0 as usize] = Leaf {
            msw: byte,
            nex: 0,
            buf: None,
            hax: 0,
        };
    }
    offset += num_bytes as usize;

    let num_nats = num_bytes + num_words + num_bigs;

    // Load fragments
    if num_frags > 0 {
        let clif = offset % 8;
        offset -= clif;

        let ptr: &[u64] =
            unsafe { std::slice::from_raw_parts(buf.as_ptr().add(offset) as *const u64, (buf.len() - offset) / 8) };

        let red = (clif * 8) as u64;
        let acc = if !ptr.is_empty() { ptr[0] } else { 0 };
        let rem = (ptr.len() as i32) - 1;

        let mut s = FragLoadState {
            ctx,
            ptr,
            ptr_idx: 1,
            rem,
            acc,
            red,
            ref_bits: 0,
            max_ref: 0,
            num_nats,
            num_holes,
        };

        for i in 0..num_frags {
            let num_refs = num_holes + num_nats + i;
            s.max_ref = num_refs - 1;
            s.ref_bits = word64_bits(s.max_ref) as u64;

            let fv = load_fragment(&mut s)?;
            s.ctx.alloc_frag(fv);
        }
    } else {
        let num_leaves = num_holes + num_nats;

        if num_leaves == 0 {
            return Err(SeedError::InvalidFormat("no leaves".to_string()));
        }

        if num_leaves > 1 {
            return Err(SeedError::NoFragsMultipleLeaves);
        }

        if num_holes == 0 {
            let v = TreeNodeValue::Nat(NatId(0));
            ctx.alloc_treenode(v, 0);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size_single_word() {
        let mut seed = Seed::new();
        seed.word(42);
        seed.done().unwrap();

        let size = seed_size(&seed);
        assert!(size >= 40); // At least header size
        assert_eq!(size % 8, 0); // Must be multiple of 8
    }

    #[test]
    fn test_round_trip_single() {
        let mut seed = Seed::new();
        seed.word(42);
        seed.done().unwrap();

        let size = seed_size(&seed);
        let mut buf = vec![0u8; size];
        let written = seed_save(&seed, &mut buf);
        assert_eq!(written, size);

        let mut seed2 = Seed::new();
        seed2.load(&buf).unwrap();

        assert_eq!(seed2.nat_count(), 1);
    }

    #[test]
    fn test_round_trip_tree() {
        let mut seed = Seed::new();
        let a = seed.word(1);
        let b = seed.word(2);
        let c = seed.cons(a, b);
        let d = seed.word(3);
        let _e = seed.cons(c, d);
        seed.done().unwrap();

        let size = seed_size(&seed);
        let mut buf = vec![0u8; size];
        let written = seed_save(&seed, &mut buf);
        assert_eq!(written, size);

        let mut seed2 = Seed::new();
        seed2.load(&buf).unwrap();

        // After loading, we should have the same structure
        assert_eq!(seed2.nat_count(), seed.nat_count());
    }
}
