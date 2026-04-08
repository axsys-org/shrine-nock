//! Safe wrappers for Urbit noun types.
//!
//! A noun is the fundamental data type in Urbit - it is either:
//! - An atom: an unsigned integer of arbitrary size
//! - A cell: a pair of two nouns `[head tail]`
//!
//! This module provides safe Rust wrappers that manage reference counting
//! and provide type-safe access to the underlying noun data.

use vere_sys::{u3a_atom, u3a_to_ptr_fn, u3s_cue_xeno_done};

use crate::{loob, mote_from_str};
use std::fmt;

/// Sentinel value indicating an invalid or missing noun.
pub const NONE: u32 = 0xffffffff;

/// Nil / null / `~` in Hoon - the atom 0.
pub const NUL: u32 = 0;

pub fn is_direct(noun: u32) -> bool {
  (noun >> 31) == 0
}

pub fn is_indirect(noun: u32) -> bool {
  (noun >> 31) != 0
}

pub fn is_atom(noun: u32) -> bool {
  is_direct(noun) || ((noun >> 30) == 0b10)
}

pub fn is_cell(noun: u32) -> bool {
  (noun >> 30) == 0b11
}

/// An owned noun that manages its own reference count.
///
/// When dropped, the reference count is decremented via `u3z`.
/// Clone increments the reference count via `u3k`.
#[repr(transparent)]
pub struct Noun(u32);

impl !Send for Noun {}

impl !Sync for Noun {}

impl Noun {
    /// Get the null noun
    /// 
    /// # Safety
    /// 0 is always a valid u3_noun
    /// 
    pub fn null() -> Self {
        unsafe { Self::from_raw(NUL) }
    }
    /// Create a noun from a raw u3_noun value.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `raw` is a valid noun reference
    /// with an appropriate reference count (i.e., the caller is transferring
    /// ownership of one reference to this Noun).
    #[inline]
    pub const unsafe fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// Create a noun from a raw u3_noun value.
    /// Returning failing if the noun is u3_weak
    #[inline]
    pub const fn from_weak(weak: u32) -> Option<Self> {
        if weak == NONE {
            None
        } else {
            Some(Self(weak))
        }
    }

    pub fn from_slice(slice: &[Noun]) -> Self {
        let last = slice.last().unwrap();
        let mut res = last.clone();
        for noun in slice.iter().rev().skip(1) {
            res = cell(noun.clone(), res).into_noun();
        }
        res
    }


    /// Get the raw u3_noun value without affecting reference count.
    #[inline]
    pub const fn as_raw(&self) -> u32 {
        self.0
    }

    pub fn refcount(&self) -> u32 {
        unsafe {
            let ptr = u3a_to_ptr_fn(self.0);
            return (*ptr).use_w;
        }
    }

    pub fn from_i128(value: i128) -> Self {
        assert!(value >= 0, "value must be positive");
        Atom::from_bytes(&value.to_le_bytes()).into_noun()
    }


    #[inline]
    pub fn transfer(self) -> u32 {
        self.as_raw()
    }

    #[inline]
    pub fn retain(&self) -> u32 {
        self.clone().into_raw()
    }

    pub fn mote(s: &str) -> Self {
        assert!(s.len() == 4, "mote must be 4 characters");
        Self(mote_from_str(s))
    }
    

    #[inline]
    pub fn borrow(&self) -> NounRef<'_> {
        NounRef(self.as_raw(), std::marker::PhantomData)
    }

    /// Consume self and return the raw u3_noun, transferring ownership.
    ///
    /// The caller becomes responsible for managing the reference count.
    #[inline]
    pub fn into_raw(self) -> u32 {
        let raw = self.0;
        std::mem::forget(self);
        raw
    }

    pub fn gain(&self) {
        unsafe {
            vere_sys::u3a_gain(self.0);
        }
    }

    /// Check if this is a direct atom (cat) - fits in 31 bits.
    #[inline]
    pub fn is_direct(&self) -> bool {
        is_direct(self.0)
    }

    pub fn into_some(self) -> Noun {
        cell(Noun::null(), self).into_noun()
    }


    /// Check if this is an indirect noun (dog) - pointer into loom.
    #[inline]
    pub fn is_indirect(&self) -> bool {
        is_indirect(self.0)
    }

    /// Check if this is an atom (direct or indirect).
    #[inline]
    pub fn is_atom(&self) -> bool {
        is_atom(self.0)
    }

    /// Check if this is a cell.
    #[inline]
    pub fn is_cell(&self) -> bool {
        is_cell(self.0)
    }

    /// Try to interpret this noun as an atom.
    ///
    /// Returns `None` if this is a cell.
    #[inline]
    pub fn as_atom(&self) -> Option<AtomRef<'_>> {
        if self.is_atom() {
            Some(AtomRef(self.0, std::marker::PhantomData))
        } else {
            None
        }
    }

    /// Try to interpret this noun as a cell.
    ///
    /// Returns `None` if this is an atom.
    #[inline]
    pub fn as_cell(&self) -> Option<CellRef<'_>> {
        if self.is_cell() {
            Some(CellRef(self.0, std::marker::PhantomData))
        } else {
            None
        }
    }

    /// Convert to an owned Atom if this is an atom.
    pub fn into_atom(self) -> Result<Atom, Self> {
        if self.is_atom() {
            Ok(Atom(self))
        } else {
            Err(self)
        }
    }

    /// Convert to an owned Cell if this is a cell.
    pub fn into_cell(self) -> Result<Cell, Self> {
        if self.is_cell() {
            Ok(Cell(self))
        } else {
            Err(self)
        }
    }

    pub fn into_pair(self) -> Result<(Noun, Noun), Self> {
        let cell = self.into_cell()?;
        Ok((cell.head().to_owned(), cell.tail().to_owned()))
    }

    pub fn into_trel(self) -> Result<(Noun, Noun, Noun), Self> {
        let cell = self.into_cell()?;
        let last = cell.tail().to_owned().into_cell()?;
        Ok((cell.head().to_owned(), last.head().to_owned(), last.tail().to_owned()))
    }

    /// Check structural equality with another noun.
    #[inline]
    pub fn equals(&self, other: &Noun) -> bool {
        // SAFETY: Both are valid nouns
        loob::to_bool(unsafe { vere_sys::u3r_sing(self.0, other.0) })
    }

    /// Compute the mug (31-bit hash) of this noun.
    #[inline]
    pub fn mug(&self) -> u32 {
        // SAFETY: self.0 is a valid noun
        unsafe { vere_sys::u3r_mug(self.0) }
    }

    /// Get a fragment of this noun by axis.
    ///
    /// Returns `None` if the axis doesn't exist in this noun.
    pub fn at(&self, axis: u32) -> Option<NounRef<'_>> {
        // SAFETY: self.0 is a valid noun, axis is a valid atom
        let result = unsafe { vere_sys::u3r_at(axis, self.0) };
        if result == NONE {
            None
        } else {
            Some(NounRef(result, std::marker::PhantomData))
        }
    }

    /// Serialize this noun to jam-encoded bytes.
    ///
    /// Jam is Urbit's standard noun serialization format. The resulting bytes
    /// can be deserialized back to a noun using [`cue`].
    pub fn jam(&self) -> Vec<u8> {
        let mut len: u64 = 0;
        let mut bytes: *mut u8 = std::ptr::null_mut();

        // SAFETY: self.0 is a valid noun, u3s_jam_xeno allocates off-loom
        unsafe {
            vere_sys::u3s_jam_xeno(self.0, &mut len, &mut bytes);
        }

        if bytes.is_null() || len == 0 {
            return Vec::new();
        }

        // Copy bytes into a Rust Vec and free the C-allocated memory
        let result = unsafe { std::slice::from_raw_parts(bytes, len as usize) }.to_vec();

        // Free the off-loom allocated bytes
        unsafe {
            libc::free(bytes as *mut libc::c_void);
        }

        result
    }
}

/// Deserialize jam-encoded bytes to a noun.
///
/// Returns `None` if the bytes are not valid jam encoding.
///
/// # Example
///
/// ```rust,ignore
/// use milton::{Atom, cue};
///
/// let atom = Atom::from_u32(42);
/// let jammed = atom.as_noun().jam();
/// let restored = cue(&jammed).unwrap();
/// assert!(restored.equals(atom.as_noun()));
/// ```
pub fn cue(bytes: &[u8]) -> Option<Noun> {
    if bytes.is_empty() {
        // Empty bytes -> atom 0
        return Some(Noun(0));
    }

    // SAFETY: bytes is a valid slice, u3s_cue_bytes returns a noun or NONE
    let result = unsafe { vere_sys::u3s_cue_bytes(bytes.len() as u64, bytes.as_ptr()) };

    if result == NONE {
        None
    } else {
        // SAFETY: u3s_cue_bytes returns a valid noun with appropriate refcount
        Some(unsafe { Noun::from_raw(result) })
    }
}

/*
**  fibonacci constants, for convenient initialization of
**  objects intended to be reallocated with fibonacci growth
*/

pub mod fibs {
pub const FIB27: u64   = 196418;
pub const FIB28: u64   = 317811;
}
pub fn cue_xeno(buf: &[u8]) -> Option<Noun> {
    let sil_u = unsafe { vere_sys::u3s_cue_xeno_init_with(fibs::FIB27, fibs::FIB28) };
    unsafe {
        let pil= Noun::from_weak(vere_sys::u3s_cue_xeno_with(sil_u, buf.len() as u64, buf.as_ptr() as *const u8));
        u3s_cue_xeno_done(sil_u);
        return pil;
    }
}

impl Clone for Noun {
    fn clone(&self) -> Self {
        // SAFETY: self.0 is a valid noun, u3k increments reference count
        unsafe {
            vere_sys::u3a_gain(self.0);
        }
        Self(self.0)
    }
}

impl Drop for Noun {
    fn drop(&mut self) {
        // SAFETY: self.0 is a valid noun, u3z decrements reference count
        unsafe {
            // let msg = CString::new("drop").unwrap();
            // u3m_p(msg.as_ptr() as *const i8, self.0);
            vere_sys::u3a_lose(self.0);
        }
    }
}

impl PartialEq for Noun {
    fn eq(&self, other: &Self) -> bool {
        self.equals(other)
    }
}

impl Eq for Noun {}

impl fmt::Debug for Noun {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_atom() {
            if self.is_direct() {
                write!(f, "Atom({})", self.0)
            } else {
                write!(f, "Atom(indirect:0x{:08x})", self.0)
            }
        } else {
            write!(f, "Cell(0x{:08x})", self.0)
        }
    }
}

/// A borrowed reference to a noun.
///
/// This does not manage reference counts - the underlying noun must outlive this reference.
#[derive(Clone, Copy)]
pub struct NounRef<'a>(u32, std::marker::PhantomData<&'a Noun>);

impl<'a> NounRef<'a> {
    /// Get the raw u3_noun value.
    #[inline]
    pub const fn as_raw(&self) -> u32 {
        self.0
    }

    /// Check if this is an atom.
    #[inline]
    pub fn is_atom(&self) -> bool {
        ((self.0 >> 31) == 0) || ((self.0 >> 30) == 0b10)
    }

    /// Check if this is a cell.
    #[inline]
    pub fn is_cell(&self) -> bool {
        (self.0 >> 30) == 0b11
    }

    pub fn as_cell(&self) -> Option<CellRef<'_>> {
        if self.is_cell() {
            Some(CellRef(self.0, std::marker::PhantomData))
        } else {
            None
        }
    }

    pub fn as_atom(&self) -> Option<AtomRef<'_>> {
        if self.is_atom() {
            Some(AtomRef(self.0, std::marker::PhantomData))
        } else {
            None
        }
    }

    /// Clone to an owned Noun, incrementing the reference count.
    pub fn to_owned(&self) -> Noun {
        // SAFETY: self.0 is a valid noun reference
        unsafe {
            vere_sys::u3a_gain(self.0);
        }
        Noun(self.0)
    }
}

impl fmt::Debug for NounRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NounRef(0x{:08x})", self.0)
    }
}

/// An owned atom - a noun that is known to be an unsigned integer.
#[repr(transparent)]
pub struct Atom(Noun);

impl Atom {
    /// Create an atom from a u32 value (direct atom).
    ///
    /// # Panics
    ///
    /// Panics if the value doesn't fit in 31 bits (>= 2^31).
    #[inline]
    pub fn from_u32(value: u32) -> Self {
        assert!(value < (1 << 31), "value must fit in 31 bits for direct atom");
        // Direct atoms don't need reference counting
        Self(Noun(value))
    }

    /// Create an atom from a u64 value.
    #[inline]
    pub fn from_u64(value: u64) -> Self {
        // SAFETY: u3i_chub creates a valid atom
        let raw = unsafe { vere_sys::u3i_chub(value) };
        Self(Noun(raw))
    }

    pub fn is_direct(&self) -> bool {
        self.0.is_direct()
    }

    pub fn is_indirect(&self) -> bool {
        self.0.is_indirect()
    }

    /// Create an atom from a byte slice (LSB first).
    pub fn from_bytes(bytes: &[u8]) -> Self {
        if bytes.is_empty() {
            return Self::from_u32(0);
        }
        // SAFETY: u3i_bytes creates a valid atom from the byte slice
        let raw = unsafe { vere_sys::u3i_bytes(bytes.len() as u32, bytes.as_ptr()) };
        Self(Noun(raw))
    }

    /// Create an atom from a string (treated as bytes, LSB first).
    pub fn from_str(s: &str) -> Self {
        Self::from_bytes(s.as_bytes())
    }

    /// Create an atom from a C-style string.
    pub fn from_cstr(s: &std::ffi::CStr) -> Self {
        // SAFETY: u3i_string creates a valid atom from a null-terminated string
        let raw = unsafe { vere_sys::u3i_string(s.as_ptr()) };
        Self(Noun(raw))
    }

    /// Get the underlying noun.
    #[inline]
    pub fn as_noun(&self) -> &Noun {
        &self.0
    }

    /// Convert to the underlying noun, consuming self.
    #[inline]
    pub fn into_noun(self) -> Noun {
        self.0
    }

    /// Get the raw u3_atom value.
    #[inline]
    pub fn as_raw(&self) -> u32 {
        self.0 .0
    }

    pub fn to_string(&self) -> String {
        fn trunc_zeros(str: &mut String) {
            let mut idx = 0;
            for ch in str.as_bytes() {
                if *ch != 0 {
                    idx += 1;
                } else {
                    break;
                }

            }
            str.truncate(idx);
        }
        if let Some(words) = self.to_words() {
            let bytes = bytemuck::cast_slice::<u32, u8>(&words);
            let mut res =String::from_utf8_lossy(&bytes).to_string();

           trunc_zeros(&mut res);
           return res;
        } else {
           let mote = self.as_u32().unwrap();
           let bytes = mote.to_le_bytes();
           let mut res = String::from_utf8_lossy(&bytes).to_string();
           trunc_zeros(&mut res);
           return res;
        }
    }




    /// Try to get the value as a u32.
    ///
    /// Returns `None` if the atom is larger than 32 bits.
    pub fn as_u32(&self) -> Option<u32> {
        // For direct atoms, the value is just the raw value
        if self.0.is_direct() {
            Some(self.0 .0)
        } else {
            // Check if it fits in a word
            let mut out: u32 = 0;
            // SAFETY: self is a valid atom
            let fits = unsafe { vere_sys::u3r_word_fit(&mut out, self.0 .0) };
            if fits != 0 {
                Some(out)
            } else {
                None
            }
        }
    }

    /// Try to get the value as a u64.
    ///
    /// Returns `None` if the atom is larger than 64 bits.
    pub fn as_u64(&self) -> Option<u64> {
        // SAFETY: self is a valid atom
        let met = unsafe { vere_sys::u3r_met(6, self.0 .0) };
        if met > 1 {
            None
        } else {
            // SAFETY: self is a valid atom and fits in 64 bits
            Some(unsafe { vere_sys::u3r_chub(0, self.0 .0) })
        }
    }

    pub fn to_words(&self) -> Option<Vec<u32>> {
        unsafe { 
            if self.0.is_direct() {
                return None;
            }
            let atom_ptr: *mut u3a_atom = std::mem::transmute(u3a_to_ptr_fn(self.0.0));
            let ptr = (*atom_ptr).buf_w.as_ptr() as *const u32;
            let words = std::slice::from_raw_parts(ptr, (*atom_ptr).len_w as usize);
            Some(words.to_vec())
        }
    }
    

    /// Get the size of this atom in the given bloq size.
    ///
    /// bloq 0 = bits, bloq 3 = bytes, bloq 5 = words
    #[inline]
    pub fn met(&self, bloq: u8) -> u32 {
        // SAFETY: self is a valid atom
        unsafe { vere_sys::u3r_met(bloq, self.0 .0) }
    }

    /// Get the size in bits.
    #[inline]
    pub fn bit_len(&self) -> u32 {
        self.met(0)
    }

    /// Get the size in bytes.
    #[inline]
    pub fn byte_len(&self) -> u32 {
        self.met(3)
    }

    /// Copy the atom's bytes into a buffer (LSB first).
    ///
    /// Returns the number of bytes copied.
    pub fn to_bytes(&self, buf: &mut [u8]) -> usize {
        let len = self.byte_len() as usize;
        let copy_len = buf.len().min(len);
        if copy_len > 0 {
            // SAFETY: self is a valid atom, buf is valid for copy_len bytes
            unsafe {
                vere_sys::u3r_bytes(0, copy_len as u32, buf.as_mut_ptr(), self.0 .0);
            }
        }
        copy_len
    }

    /// Convert the atom to a Vec of bytes (LSB first).
    pub fn to_vec(&self) -> Vec<u8> {
        let len = self.byte_len() as usize;
        let mut buf = vec![0u8; len];
        if len > 0 {
            // SAFETY: self is a valid atom, buf is valid
            unsafe {
                vere_sys::u3r_bytes(0, len as u32, buf.as_mut_ptr(), self.0 .0);
            }
        }
        buf
    }
}

impl Clone for Atom {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl fmt::Debug for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(v) = self.as_u64() {
            write!(f, "Atom({})", v)
        } else {
            write!(f, "Atom({} bits)", self.bit_len())
        }
    }
}

impl From<u32> for Atom {
    fn from(value: u32) -> Self {
        if value < (1 << 31) {
            Self::from_u32(value)
        } else {
            Self::from_u64(value as u64)
        }
    }
}

impl From<u64> for Atom {
    fn from(value: u64) -> Self {
        Self::from_u64(value)
    }
}

/// A borrowed reference to an atom.
#[derive(Clone, Copy)]
pub struct AtomRef<'a>(u32, std::marker::PhantomData<&'a Noun>);

impl<'a> AtomRef<'a> {
    /// Get the raw value.
    #[inline]
    pub fn as_raw(&self) -> u32 {
        self.0
    }

    pub fn is_direct(&self) -> bool {
        is_direct(self.0)
    }

    pub fn is_indirect(&self) -> bool {
        is_indirect(self.0)
    }

    #[inline]
    pub fn as_words(&self) -> Option<&[u32]> {
        unsafe {
            if self.is_direct() {
                return None;
            }
            let atom_ptr: *mut u3a_atom = std::mem::transmute(u3a_to_ptr_fn(self.as_raw()));
            let buf_ptr: *const u32 = (*atom_ptr).buf_w.as_ptr() as *const u32;
            let words: &[u32] = std::slice::from_raw_parts(buf_ptr, (*atom_ptr).len_w as usize);
            Some(words)
        }
    }

    /// Try to get the value as a u32.
    pub fn as_u32(&self) -> Option<u32> {
        if is_direct(self.0) {
            Some(self.0)
        } else {
            let mut out: u32 = 0;
            let fits = unsafe { vere_sys::u3r_word_fit(&mut out, self.0) };
            if fits != 0 {
                Some(out)
            } else {
                None
            }
        }
    }

    /// Clone to an owned Atom.
    pub fn to_owned(&self) -> Atom {
        unsafe {
            let non = Noun::from_raw(self.0);
            return Atom(non.clone());
        }
    }
}

impl fmt::Debug for AtomRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(v) = self.as_u32() {
            write!(f, "AtomRef({})", v)
        } else {
            write!(f, "AtomRef(indirect)")
        }
    }
}

/// An owned cell - a noun that is known to be a pair.
#[repr(transparent)]
pub struct Cell(Noun);

impl Cell {
    /// Create a cell from two nouns `[head tail]`.
    ///
    /// Consumes both arguments.
    pub fn new(head: Noun, tail: Noun) -> Self {
        let h = head.into_raw();
        let t = tail.into_raw();
        // SAFETY: u3i_cell creates a valid cell, consuming the references
        let raw = unsafe { vere_sys::u3i_cell(h, t) };
        Self(Noun(raw))
    }

    /// Create a triple `[a b c]`.
    pub fn trel(a: Noun, b: Noun, c: Noun) -> Self {
        let a = a.into_raw();
        let b = b.into_raw();
        let c = c.into_raw();
        // SAFETY: u3i_trel creates a valid cell
        let raw = unsafe { vere_sys::u3i_trel(a, b, c) };
        Self(Noun(raw))
    }

    /// Create a quad `[a b c d]`.
    pub fn qual(a: Noun, b: Noun, c: Noun, d: Noun) -> Self {
        let a = a.into_raw();
        let b = b.into_raw();
        let c = c.into_raw();
        let d = d.into_raw();
        // SAFETY: u3i_qual creates a valid cell
        let raw = unsafe { vere_sys::u3i_qual(a, b, c, d) };
        Self(Noun(raw))
    }

    /// Get the underlying noun.
    #[inline]
    pub fn as_noun(&self) -> &Noun {
        &self.0
    }

    /// Convert to the underlying noun.
    #[inline]
    pub fn into_noun(self) -> Noun {
        self.0
    }

    /// Get the raw value.
    #[inline]
    pub fn as_raw(&self) -> u32 {
        self.0 .0
    }

    /// Get the head of the cell (borrows).
    #[inline]
    pub fn head(&self) -> NounRef<'_> {
        // SAFETY: self is a valid cell
        let h = unsafe { vere_sys::u3a_head(self.0 .0) };
        NounRef(h, std::marker::PhantomData)
    }

    /// Get the tail of the cell (borrows).
    #[inline]
    pub fn tail(&self) -> NounRef<'_> {
        // SAFETY: self is a valid cell
        let t = unsafe { vere_sys::u3a_tail(self.0 .0) };
        NounRef(t, std::marker::PhantomData)
    }

    /// Get owned copies of head and tail.
    pub fn into_parts(self) -> (Noun, Noun) {
        let head = self.head().to_owned();
        let tail = self.tail().to_owned();
        // Drop self last (after we've gained refs to head/tail)
        drop(self);
        (head, tail)
    }
}

impl Clone for Cell {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl fmt::Debug for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Cell(0x{:08x})", self.0 .0)
    }
}

/// A borrowed reference to a cell.
#[derive(Clone, Copy)]
pub struct CellRef<'a>(u32, std::marker::PhantomData<&'a Noun>);

impl<'a> CellRef<'a> {
    /// Get the raw value.
    #[inline]
    pub fn as_raw(&self) -> u32 {
        self.0
    }

    /// Get the head.
    #[inline]
    pub fn head(&self) -> NounRef<'a> {
        // SAFETY: self.0 is a valid cell
        let h = unsafe { vere_sys::u3a_head(self.0) };
        NounRef(h, std::marker::PhantomData)
    }

    /// Get the tail.
    #[inline]
    pub fn tail(&self) -> NounRef<'a> {
        // SAFETY: self.0 is a valid cell
        let t = unsafe { vere_sys::u3a_tail(self.0) };
        NounRef(t, std::marker::PhantomData)
    }

    /// Clone to an owned Cell.
    pub fn to_owned(&self) -> Cell {
        unsafe {
            let non = Noun::from_raw(self.0);
            return Cell(non.clone());
        }
    }
}

impl fmt::Debug for CellRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CellRef(0x{:08x})", self.0)
    }
}

/// Convenience function to create a cell.
#[inline]
pub fn cell(head: Noun, tail: Noun) -> Cell {
    Cell::new(head, tail)
}

/// Convenience function to create a triple.
#[inline]
pub fn trel(a: Noun, b: Noun, c: Noun) -> Cell {
    Cell::trel(a, b, c)
}

/// Convenience function to create a quad.
#[inline]
pub fn qual(a: Noun, b: Noun, c: Noun, d: Noun) -> Cell {
    Cell::qual(a, b, c, d)
}
