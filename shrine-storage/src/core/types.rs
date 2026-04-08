//! Core types for the shrine-storage system.
//!
//! This module contains all the data structures used to interact with the storage:
//!
//! - [`Lock`] - Version tracking with data and shape components
//! - [`Binding`] - A path with its version and slot data
//! - [`ChildRef`] - Reference to a child path with its version
//! - [`XRead`], [`YRead`], [`ZRead`] - Read results for each observation level
//! - [`Care`] - Selector for which observation level to query
//! - [`Observe`] - Tri-state result distinguishing found/null/unknown
//! - [`Note`] - Caller intent for write operations (poke/make/cull)
//! - [`Tale`] - Slot data map (slot name -> binary data)
//! - [`Mode`] - Type of modification (add/dif/del)
//! - [`Gift`] - Result of a write operation with computed locks
//! - [`Ever`] - Locks for all observation levels

use bytemuck::{Pod, Zeroable, pod_read_unaligned};
use milton::food::{jim, mew};
use core::panic;
use derive_everything::derive_everything;
use milton::sys::u3m_p;
use milton::traits::{FromNoun, IntoNoun, NounConversionError};
use milton::{Atom, Noun, cell, cue, debug_noun_print, jets, mote_from_str, trel};
use std::collections::{HashMap, HashSet, hash_map};
use std::ffi::CString;

use crate::NAMESPACE;
use crate::core::path::{path_string_from_noun, path_to_segments};

pub type Case = u32;
pub type Upon = u32;

/// Version information for a binding.
///
/// A `Lock` tracks two independent version numbers:
///
/// - **`data`**: Content version, increments on every write (1, 2, 3, ...)
/// - **`shape`**: Structural version, increments only on creation or deletion
///
/// # Version Rules
///
/// When writing a binding:
/// - `data` must be exactly `previous_data + 1` (or `1` for new paths)
/// - `shape` must be `>= previous_shape` (cannot decrease)
/// - `shape` should increment when creating a new path or deleting (tombstoning)
///
/// # Example
///
/// ```rust
/// use shrine_storage::Lock;
///
/// // First write to a path: data=1, shape=1
/// let initial = Lock::new(1, 1);
///
/// // Content update: data increments, shape stays same
/// let update = Lock::new(2, 1);
///
/// // Deletion (tombstone): both increment
/// let tombstone = Lock::new(3, 2);
/// ```
#[derive(Clone, Copy, Pod, Zeroable, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(C)]
pub struct Lock {
    /// Content version number. Must increment by 1 on each write.
    pub data: u32,
    /// Structural version number. Increments on creation/deletion, stays same on updates.
    pub shape: u32,
}

impl std::fmt::Debug for Lock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({data},{shape})", data = self.data, shape = self.shape)
    }
}

impl IntoNoun for Lock {
    fn into_noun(&self) -> Result<Noun, NounConversionError> {
        Ok(cell(self.data.into_noun()?, self.shape.into_noun()?).into_noun())
    }
}
impl FromNoun for Lock {
    fn from_noun(noun: Noun) -> Result<Self, NounConversionError> {
        let (data, shape) = noun.into_pair().unwrap();
        Ok(Self {
            data: data.as_atom().unwrap().as_u32().unwrap(),
            shape: shape.as_atom().unwrap().as_u32().unwrap(),
        })
    }
}
impl Lock {
    /// Produces a new lock with data incremented by one
    pub fn from_inc(&self) -> Self {
        Self {
            data: self.data + 1,
            shape: self.shape,
        }
    }

    pub fn bump_x(&mut self, shape: bool) {
        if shape {
            self.shape += 1;
        }
        self.data += 1;
    }

    /// Produces a new lock with shape and data incremented by one
    pub fn inc_shape(&self) -> Self {
        Self {
            data: self.data + 1,
            shape: self.shape + 1,
        }
    }

    pub fn to_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        bytemuck::pod_read_unaligned::<Self>(bytes)
    }

    pub fn one() -> Self {
        Self { data: 1, shape: 1 }
    }

    pub fn new(data: u32, shape: u32) -> Self {
        Self { data, shape }
    }
}

/// Write mode for a note.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Mode {
    #[default]
    Add,
    Dif,
    Del,
}

impl IntoNoun for Mode {
    fn into_noun(&self) -> Result<Noun, NounConversionError> {
        match self {
            Mode::Add => Ok(Atom::from_str("add").into_noun()),
            Mode::Dif => Ok(Atom::from_str("dif").into_noun()),
            Mode::Del => Ok(Atom::from_str("del").into_noun()),
        }
    }
}

/// A binding represents data to be written at a path.
///
/// Each binding contains:
/// - A **path** identifying the location in the hierarchy
/// - A **lock** specifying the version for this write
/// - **Slots** mapping slot names to pails (type + data)
///
/// # Tombstones
///
/// A binding with empty `slots` is a tombstone (deletion marker). The path's
/// data is considered deleted, but the version history is preserved. Historical
/// versions can still be read.
///
/// # Example
///
/// ```rust
/// use shrine_storage::{Binding, Lock, Pail};
/// use std::collections::HashMap;
///
/// // Create a binding with data
/// let binding = Binding::new(
///     "/users/alice",
///     Lock::new(1, 1),
///     HashMap::from([
///         ("name".to_string(), Pail::new("/types/text", b"Alice".to_vec())),
///         ("email".to_string(), Pail::new("/types/text", b"alice@example.com".to_vec())),
///     ]),
/// );
///
/// assert!(!binding.is_tombstone());
///
/// // Create a tombstone (deletion)
/// let tombstone = Binding::new(
///     "/users/alice",
///     Lock::new(2, 2),
///     HashMap::new(),
/// );
///
/// assert!(tombstone.is_tombstone());
/// ```
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Note {
    /// The path where this binding will be written.
    pub path: String,
    /// The mode of the write.
    pub mode: Mode,
    /// Named slots mapping to pails (type + data). Empty slots indicate a tombstone (deletion).
    pub slots: Tale,
}

impl Note {
    pub fn to_noun(&self, ns: &Namespace) -> Result<Noun, NounConversionError> {
        let path = path_noun_from_string(&self.path);
        let mode = match self.mode {
            Mode::Add => Noun::mote("make"),
            Mode::Dif => Noun::mote("poke"),
            Mode::Del => Noun::mote("cull"),
        };
        let slots = self.slots.to_noun(ns)?;
        Ok(trel(mode, path, slots).into_noun())
    }
}

pub const MAKE: u32 = mote_from_str("make");
pub const POKE: u32 = mote_from_str("poke");
pub const CULL: u32 = mote_from_str("cull");

impl FromNoun for Note {
    fn from_noun(noun: Noun) -> Result<Self, NounConversionError> {
        let mote = CString::new("test").unwrap();
        unsafe {
            // u3m_p(mote.as_ptr(), noun.as_raw());
        }
        let (path, mode, slots) = noun.into_trel().unwrap();
        let mode = match mode.as_raw() {
            MAKE => Mode::Add,
            POKE => Mode::Dif,
            CULL => Mode::Del,
            _ => return Err(NounConversionError::InvalidNoun),
        };
        // let slots = jets::map::to_hashmap(slots, |k, v| {
        //     let pax = path_string_from_noun(k);
        //     let pail = Pail::from_noun(v).unwrap();
        //     (pax, pail)
        // });
        Ok(Self {
            path: path_string_from_noun(path),
            mode,
            slots: Tale::from_noun(slots)?,
        })
    }
}

impl Note {
    /// Creates a new binding with the given path, lock, and slots.
    pub fn make(path: impl Into<String>, slots: Tale) -> Self {
        Self {
            path: path.into(),
            mode: Mode::Add,
            slots: slots,
        }
    }

    /// Creates a request to delete the path.
    pub fn cull(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            mode: Mode::Del,
            slots: Tale::empty(),
        }
    }

    /// Creates a request to update the path.
    pub fn poke(path: impl Into<String>, slots: Tale) -> Self {
        Self {
            path: path.into(),
            mode: Mode::Dif,
            slots: slots,
        }
    }

    /// Returns `true` if this binding is a tombstone (empty slots = deletion).
    pub fn is_tombstone(&self) -> bool {
        matches!(self.mode, Mode::Del)
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Pod, Zeroable, Copy, Default, PartialOrd, Ord)]
#[repr(C)]
pub struct Span {
    pub earth: i128,
    pub block: u64,
    pub _pad: [u8; 8],
}

impl std::fmt::Debug for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({earth},{block})",
            earth = self.earth,
            block = self.block
        )
    }
}

impl IntoNoun for Span {
    fn into_noun(&self) -> Result<Noun, NounConversionError> {
        Ok(cell(
            Atom::from_u32(self.earth as u32).into_noun(),
            Atom::from_u64(self.block).into_noun(),
        )
        .into_noun())
    }
}
impl FromNoun for Span {
    fn from_noun(noun: Noun) -> Result<Self, NounConversionError> {
        let (earth, block) = noun.into_pair().unwrap();
        Ok(Self {
            earth: earth.as_atom().unwrap().as_u32().unwrap() as i128,
            block: block.as_atom().unwrap().as_u32().unwrap() as u64,
            _pad: [0; 8],
        })
    }
}

impl Span {
    pub fn new() -> Self {
        Self {
            earth: 0,
            block: 0,
            _pad: [0; 8],
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(std::mem::size_of::<Self>());
        bytes.extend_from_slice(&self.earth.to_le_bytes());
        bytes.extend_from_slice(&self.block.to_le_bytes());
        bytes.extend_from_slice(&self._pad);
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        pod_read_unaligned::<Self>(bytes)
    }
}

#[derive(Clone, Pod, Zeroable, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(C)]
pub struct Ever {
    pub span: Span,
    pub x: Lock,
    pub y: Lock,
    pub z: Lock,
    pub _pad: [u8; 8],
}
impl std::fmt::Debug for Ever {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Ever(x:{x:?},y:{y:?},z:{z:?},span:{span:?})",
            x = self.x,
            y = self.y,
            z = self.z,
            span = self.span
        )
    }
}

impl IntoNoun for Ever {
    fn into_noun(&self) -> Result<Noun, NounConversionError> {
        let x = self.x.into_noun()?;
        let y = self.y.into_noun()?;
        let z = self.z.into_noun()?;
        let _null = Noun::null();
        let span = self.span.into_noun()?;
        let res = Noun::from_slice(&[x, y, z, span]);
        Ok(res)
    }
}
impl FromNoun for Ever {
    fn from_noun(noun: Noun) -> Result<Self, NounConversionError> {
        let (x, rest) = noun.into_pair().unwrap();
        let (y, rest) = rest.into_pair().unwrap();
        let (z, span) = rest.into_pair().unwrap();
        let x = Lock::from_noun(x)?;
        let y = Lock::from_noun(y)?;
        let z = Lock::from_noun(z)?;
        let span = Span::from_noun(span)?;
        Ok(Self {
            x,
            y,
            z,
            span,
            _pad: [0; 8],
        })
    }
}

impl Ever {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(std::mem::size_of::<Self>());
        bytes.extend_from_slice(&self.x.to_bytes());
        bytes.extend_from_slice(&self.y.to_bytes());
        bytes.extend_from_slice(&self.z.to_bytes());
        bytes.extend_from_slice(&self.span.to_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let x = Lock::from_bytes(&bytes[0..std::mem::size_of::<Lock>()]);
        let y =
            Lock::from_bytes(&bytes[std::mem::size_of::<Lock>()..2 * std::mem::size_of::<Lock>()]);
        let z = Lock::from_bytes(
            &bytes[2 * std::mem::size_of::<Lock>()..3 * std::mem::size_of::<Lock>()],
        );
        let span = Span::from_bytes(&bytes[3 * std::mem::size_of::<Lock>()..]);
        Self {
            x,
            y,
            z,
            span,
            _pad: [0; 8],
        }
    }

    pub fn bump_x(&mut self, ctx: &StoreCtx, shape: bool) {
        self.x.bump_x(shape);
        self.bump_y(ctx, shape);
        self.bump_z(ctx, shape);
        self.span = ctx.span();
    }

    pub fn bump_y(&mut self, ctx: &StoreCtx, shape: bool) {
        self.y.bump_x(shape);
        self.span = ctx.span();
    }

    pub fn bump_z(&mut self, ctx: &StoreCtx, shape: bool) {
        self.z.bump_x(shape);
        self.span = ctx.span();
    }

    pub fn new(span: &Span) -> Self {
        Self {
            x: Lock::one(),
            y: Lock::one(),
            z: Lock::one(),
            span: span.clone(),
            _pad: [0; 8],
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Pod, Zeroable, Copy, Default, PartialOrd, Ord)]
#[repr(C)]
pub struct Oath {
    pub hash: [u8; 32],
    pub signature: [u8; 32],
}

impl std::fmt::Debug for Oath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Oath()")
    }
}

impl Oath {
    pub fn new() -> Self {
        Self {
            hash: [0; 32],
            signature: [0; 32],
        }
    }

    pub fn sign(&mut self, _ctx: &StoreCtx, _data: &[u8]) {
        // println!("stubbed signing")
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Pod, Zeroable, Copy, Default, PartialOrd, Ord)]
#[repr(C)]
pub struct Aeon {
    pub oath: Oath,
    pub ever: Ever,
}

impl std::fmt::Debug for Aeon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Aeon({ever:?})", ever = self.ever)
    }
}

impl Aeon {
    pub fn bump_x(&mut self, ctx: &StoreCtx, shape: bool) {
        self.ever.bump_x(ctx, shape);
    }

    pub fn bump_y(&mut self, ctx: &StoreCtx, shape: bool) {
        self.ever.bump_y(ctx, shape);
    }

    pub fn bump_z(&mut self, ctx: &StoreCtx, shape: bool) {
        self.ever.bump_z(ctx, shape);
    }
}

impl IntoNoun for Aeon {
    fn into_noun(&self) -> Result<Noun, NounConversionError> {
        Ok(cell(self.ever.into_noun()?, self.oath.into_noun()?).into_noun())
    }
}
impl FromNoun for Aeon {
    fn from_noun(noun: Noun) -> Result<Self, NounConversionError> {
        let (ever, oath) = noun.into_pair().unwrap();
        Ok(Self {
            oath: Oath::from_noun(oath)?,
            ever: Ever::from_noun(ever)?,
        })
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[repr(C)]
pub struct Saga {
    pub aeon: Aeon,
    pub tale: Tale,
}

impl Saga {
    pub fn new(tale: &Tale, span: &Span) -> Self {
        Self {
            aeon: Aeon {
                oath: Oath::new(),
                ever: Ever::new(span),
            },
            tale: tale.clone(),
        }
    }
    // pub fn make(&self, ctx: &StoreCtx, tale: &Tale) -> Self {
    //     let mut new = self.clone();
    //     new.aeon.bump_x(ctx, true);
    //     new.tale = tale.clone();
    //     new
    // }

    // pub fn poke(&self, ctx: &StoreCtx, tale: &Tale) -> Self {

    //     let mut next = self.clone();
    //     for (key, pail) in tale.0.iter() {
    //         next.tale.insert(key.clone(), pail.clone());
    //     }
    //     next.aeon.bump_x(&next.tale, span, false);
    //     next
    // }

    // pub fn cull(&self, tale: &Tale, span: &Span) -> Self {
    //     let mut next = self.clone();
    //     next.tale.0.clear();
    //     next.aeon.bump_x(&next.tale, span, true);
    //     next
    // }

    // pub fn bump_y(&mut self, shape: bool) {
    //     self.aeon.bump_y(shape);
    // }

    // pub fn bump_z(&mut self, shape: bool) {
    //     self.aeon.bump_z(shape);
    // }
}

impl IntoNoun for Oath {
    fn into_noun(&self) -> Result<Noun, NounConversionError> {
        let hash = Atom::from_bytes(&self.hash);
        let signature = Atom::from_bytes(&self.signature);
        Ok(cell(hash.into_noun(), signature.into_noun()).into_noun())
    }
}

impl FromNoun for Oath {
    fn from_noun(noun: Noun) -> Result<Self, NounConversionError> {
        let (hash, signature) = noun.into_pair().unwrap();
        let hash = hash.into_atom().unwrap();
        let signature = signature.into_atom().unwrap();
        let mut oath = Self::new();
        oath.hash.copy_from_slice(bytemuck::cast_slice::<u32, u8>(
            &hash.to_words().unwrap()[0..32],
        ));
        oath.signature
            .copy_from_slice(bytemuck::cast_slice::<u32, u8>(
                &signature.to_words().unwrap()[0..32],
            ));
        Ok(oath)
    }
}

impl Saga {
    pub fn to_noun(&self, ns: &Namespace) -> Result<Noun, NounConversionError> {
        Ok(cell(self.aeon.into_noun()?, self.tale.to_noun(ns)?).into_noun())
    }
}

/// Specifies which observation level to query.
///
/// Used with [`crate::Storage::get_current_lock`] to retrieve version information
/// for a specific observation level.
///
/// # Levels
///
/// - **`X`** - File-level: the actual data stored at a path
/// - **`Y`** - Folder-level: immediate children of a path
/// - **`Z`** - Subtree-level: all descendants of a path
#[derive_everything]
#[derive(Copy)]
pub enum Care {
    /// File-level (%x) - the actual data at a path
    X = 1 << 1,
    /// Folder-level (%y) - immediate children snapshot
    Y = 1 << 2,
    /// Subtree-level (%z) - all descendants snapshot
    #[default]
    Z = 1 << 3,
}
impl IntoNoun for Care {
    fn into_noun(&self) -> Result<Noun, NounConversionError> {
        match self {
            Care::X => Ok(Atom::from_str("x").into_noun()),
            Care::Y => Ok(Atom::from_str("y").into_noun()),
            Care::Z => Ok(Atom::from_str("z").into_noun()),
        }
    }
}
impl FromNoun for Care {
    fn from_noun(noun: Noun) -> Result<Self, NounConversionError> {
        let atom = noun.into_atom().unwrap();
        match atom.as_raw() {
            120 => Ok(Care::X),
            121 => Ok(Care::Y),
            122 => Ok(Care::Z),
            _ => Err(NounConversionError::InvalidNoun),
        }
    }
}

impl Care {
    pub fn packed_bits(&self) -> u8 {
        match self {
            Care::X => 0b01,
            Care::Y => 0b10,
            Care::Z => 0b11,
        }
    }

    pub fn from_ascii(ascii: u32) -> Result<Self, NounConversionError> {
        match ascii {
            120 => Ok(Care::X),
            121 => Ok(Care::Y),
            122 => Ok(Care::Z),
            _ => Err(NounConversionError::InvalidNoun),
        }
    }

    pub fn to_ascii(&self) -> u32 {
        match self {
            Care::X => 120,
            Care::Y => 121,
            Care::Z => 122,
        }
    }
}

use bitflags::bitflags;

use crate::core::path::{PathIdx, path_noun_from_string};
use crate::store::Namespace;
use crate::store::lmdb::StoreCtx;
bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct CareSet: u8 {
        const X = 1 << 1;
        const XX = 0b11;
        const Y = 0b1000;
        const YY = 0b1100;
        const Z = 0b10000;
        const ZZ = 0b110000;
        const NONE = 0;
    }
}

// pub struct Dirtied(HashMap<String, CareSet>);

// impl IntoIterator for Dirtied {
//     type Item = (String, CareSet);
//     type IntoIter = std::collections::hash_map::IntoIter<String, CareSet>;
//     fn into_iter(self) -> Self::IntoIter {
//         self.0.into_iter()
//     }
// }

// impl Dirtied {
//     pub fn new() -> Self {
//         Self(HashMap::new())
//     }

//     pub fn dirty_x(&mut self, path: &str) {
//         let care = self.0.entry(path.to_string()).or_insert(CareSet::NONE);
//         *care |= CareSet::X;
//     }

//     pub fn dirty_z(&mut self, path: &str) {
//         let care = self.0.entry(path.to_string()).or_insert(CareSet::NONE);
//         *care |= CareSet::Z;
//     }

//     pub fn dirty_y(&mut self, path: &str) {
//         let care = self.0.entry(path.to_string()).or_insert(CareSet::NONE);
//         *care |= CareSet::Y;
//     }
// }
//
fn stud_from_str(typ: &str) -> Noun {
if typ.starts_with("/") {
                    path_noun_from_string(typ)
                } else {
                    Atom::from_str(typ).into_noun()
                }

}

/// Format of the data stored in a pail.
///
/// Usually we default to `Jam`, but if we are dealing with a large mime
/// file, we would like to just stick that in the filesystem instead of
/// having it as a noun because of leading zeroes. If the pail contains
/// build outputs i.e. the entire stdlib, then we try and store it
/// lazily
///
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Format {
    Lazy,
    #[default]
    Jam,
    Buf,
}

impl Format {
    pub fn from_stud(stud: &str) -> Self {
        match stud {
            stud::VASE => Self::Lazy,
            stud::GERM => Self::Jam,
            stud::MIME => Self::Buf,
            _ => Self::Jam,
        }
    }
}

pub mod stud {
    pub const VASE: &str = "/types/stud/vase";
    pub const GERM: &str = "/types/stud/germ";
    pub const JAM: &str = "/types/atom";
    pub const MIME: &str = "/types/mime";
}

pub mod slot {
    pub const CONTENT: &str = "/sys/slot/content";
    pub const AUTH: &str = "/sys/slot/auth";
    pub const SRC: &str = "/sys/slot/src";
    pub const BUILT: &str = "/sys/slot/built";
    pub const TYPE: &str = "/sys/slot/type";
    pub const VASE: &str = "/sys/slot/vase";
    pub const CARD: &str = "/sys/slot/card";
    pub const HELP: &str = "/sys/slot/help";
    pub const LEDE: &str = "/sys/slot/lede";
    pub const KOOK: &str = "/sys/slot/kook";
    pub const CREW: &str = "/sys/slot/crew";
    pub const FANS: &str = "/sys/slot/fans";
    pub const REQ: &str = "/std/slot/req";
    pub const RES: &str = "/std/slot/res";
    pub const CLAN: &str = "/sys/slot/clan";
    pub const CANT: &str = "/sys/slot/cant";
    pub const ICON: &str = "/sys/slot/icon";
    pub const AUTHOR: &str = "/sys/slot/author";
    pub const PRODIGY: &str = "/sys/slot/prodigy";
    pub const UNREADS: &str = "/sys/slot/unreads";
    pub const PROV: &str = "/sys/slot/prov";
    pub const ACTION: &str = "/sys/slot/action";
    pub const LIKE: &str = "/sys/slot/like";
}

/// Result of a versioned read operation.
///
/// This tri-state enum is essential for understanding the storage state:
///
/// | Variant | Meaning | When it occurs |
/// |---------|---------|----------------|
/// | `Found(T)` | Data exists | Path has a binding at the requested version (may have empty slots for tombstones) |
/// | `Null` | Known empty | Version exists, but path has no binding at that version |
/// | `Unknown` | No information | Requested version hasn't been written yet, or path was never part of any transaction |
///
/// # Tombstones vs Null
///
/// A deleted path (tombstone) returns `Found` with empty slots, not `Null`.
/// This is because a tombstone is an explicit record that the path was deleted.
///
/// `Null` is returned when:
/// - Reading the current version of a path that was never written
/// - The path had version 0 in the current table (internal tracking state)
///
/// # Unknown
///
/// `Unknown` indicates the storage has no information about this version:
/// - Requesting a version number higher than any written
/// - Reading a path that has never been part of any transaction
///
/// This distinction is important for implementing consistent reads and
/// optimistic concurrency control.
///
/// # Example
///
/// ```rust
/// use shrine_storage::{Storage, Binding, Lock, Pail, Observe};
/// use std::collections::HashMap;
///
/// let mut storage = Storage::open_in_memory().unwrap();
///
/// // Path never written - Unknown
/// assert!(matches!(
///     storage.read_x("/never/written", None).unwrap(),
///     Observe::Unknown
/// ));
///
/// // Write then delete (tombstone)
/// storage.batch_write(&[Binding::new(
///     "/temp", Lock::new(1, 1),
///     HashMap::from([("data".into(), Pail::new("/types/text", b"x".to_vec()))]),
/// )]).unwrap();
///
/// storage.batch_write(&[Binding::new(
///     "/temp", Lock::new(2, 2),
///     HashMap::new(), // empty = tombstone
/// )]).unwrap();
///
/// // Current state is Found with empty slots (tombstone semantics)
/// if let Observe::Found(x) = storage.read_x("/temp", None).unwrap() {
///     assert!(x.slots.is_empty()); // tombstone = empty slots
/// }
///
/// // Version 1 still Found with data
/// if let Observe::Found(x) = storage.read_x("/temp", Some(1)).unwrap() {
///     assert!(!x.slots.is_empty());
/// }
///
/// // Future version is Unknown
/// assert!(matches!(
///     storage.read_x("/temp", Some(999)).unwrap(),
///     Observe::Unknown
/// ));
/// ```
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Observe<T> {
    /// Value found at the requested version.
    Found(T),
    /// The version exists, but the path has no binding (deleted or never created).
    Null,
    /// The requested version doesn't exist yet (future version request).
    #[default]
    Unknown,
}

impl<T> Observe<T> {
    pub fn is_found(&self) -> bool {
        matches!(self, Observe::Found(_))
    }

    pub fn desc(&self) -> &'static str {
        match self {
            Observe::Found(_) => "Found",
            Observe::Null => "Null",
            Observe::Unknown => "Unknown"
        }
    }
}

// =============================================================================
// Intent-based Write API Types
// =============================================================================

/// A pail is the value stored at a slot, containing type metadata and binary data.
///
/// `DiskPail` is the canonical pail type. Raw variants (Text, Hoon, Atom, Mesh, Duct)
/// store bytes directly without jam/cue overhead. The `Jam` variant stores jammed noun
/// bytes for arbitrary types.
#[derive(Clone, PartialEq, Eq)]
pub enum DiskPail {
    /// Jammed noun bytes with an arbitrary type string.
    Jam {
        typ: String,
        data: Vec<u8>,
    },
    Jim {
        typ: String,
        data: Vec<u8>
    },
    /// Raw text bytes (UTF-8).
    Text {
        data: Vec<u8>,
    },
    /// Raw hoon source bytes (UTF-8).
    Hoon {
        data: Vec<u8>,
    },
    /// Raw atom bytes (little-endian).
    Atom {
        data: Vec<u8>,
    },
    /// Map from slot to path
    Mesh {
        data: HashMap<String, PathIdx>,
    },
    /// List of path indices.
    Duct {
        data: Vec<PathIdx>,
    },
    Wain {
        data: Vec<String>,
    },
}

impl std::fmt::Debug for DiskPail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DiskPail({})", self.typ_str())?;
        Ok(())
    }

}

impl DiskPail {
    pub fn data(&self) -> &[u8] {
        match self {
            DiskPail::Jam { data, .. }
            | DiskPail::Text { data }
            | DiskPail::Hoon { data }
            | DiskPail::Atom { data } => data,
            DiskPail::Mesh { .. } | DiskPail::Wain { .. } => &[],
            DiskPail::Duct { data } => bytemuck::cast_slice(data),
            Self::Jim { typ, data } => data,
        }
    }
    pub fn typ_str(&self) -> &str {
        match self {
            DiskPail::Jam { typ, .. } |
            DiskPail::Jim { typ, .. } => typ.as_str(),
            DiskPail::Text { .. } => "cord",
            DiskPail::Hoon { .. } => "cord",
            DiskPail::Atom { .. } => "atom",
            DiskPail::Mesh { .. } => "crew",
            DiskPail::Duct { .. } => "duct",
            DiskPail::Wain { .. } => "wain",
        }
    }

    /// Serialize Mesh to bytes: [count: u32] then for each entry [key_len: u32, key_bytes, value: u32].
    pub fn mesh_to_bytes(data: &HashMap<String, PathIdx>) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
        for (key, idx) in data {
            bytes.extend_from_slice(&(key.len() as u32).to_le_bytes());
            bytes.extend_from_slice(key.as_bytes());
            bytes.extend_from_slice(&idx.raw().to_le_bytes());
        }
        bytes
    }

    pub fn text(s: impl Into<String>) -> Self {
        Self::Text {
            data: s.into().into_bytes()
        }
    }

    /// Deserialize Mesh from bytes.
    pub fn mesh_from_bytes(bytes: &[u8]) -> HashMap<String, PathIdx> {
        let mut pos = 0;
        let count = u32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap()) as usize;
        pos += 4;
        let mut map = HashMap::with_capacity(count);
        for _ in 0..count {
            let key_len = u32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap()) as usize;
            pos += 4;
            let key = std::str::from_utf8(&bytes[pos..pos + key_len])
                .unwrap()
                .to_string();
            pos += key_len;
            let idx = u32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap());
            pos += 4;
            map.insert(key, PathIdx::new(idx));
        }
        map
    }

    pub fn noun(noun: Noun) -> Self {
        DiskPail::Jam {
            typ: "noun".to_string(),
            data: noun.jam(),
        }
    }

    pub fn duct(paths: &[PathIdx]) -> Self {
        DiskPail::Duct { data: paths.to_vec() }
    }

    pub fn word(val: u32) -> Self {
        let data = val.to_le_bytes().to_vec();
        Self::Atom { data }
    }

    pub fn hoon(src: impl Into<String>) -> Self {
        Self::Hoon { data: src.into().into_bytes() }
    }

    /// Convert to noun, resolving PathIdx values for Duct via the namespace.
    pub fn to_noun(&self, ns: &Namespace) -> Result<Noun, NounConversionError> {
        match self {
            DiskPail::Jam { typ, data } => {
                let stud = stud_from_str(typ);

                let noun_data = cue(data).expect("invalid jammed data in DiskPail");
                Ok(cell(stud, noun_data).into_noun())
            }
            DiskPail::Jim { typ, data } => {
                let stud = stud_from_str(typ);
                let atom = Atom::from_bytes(data.as_slice());
                let noun = mew(ns.chow(), atom.into_noun()).expect("Invalid jim");
                Ok(cell(stud, noun).into_noun())
            }
            DiskPail::Mesh { data } => {
                let stud = Atom::from_str("crew").into_noun();
                let mut map = Noun::null();
                for (key, idx) in data {
                    // println!("key");
                    let key_noun = Atom::from_str(key).into_noun();
                    let path_str = ns.path_idx_to_str(idx).ok().flatten().unwrap_or_default();
                    let val_noun = path_noun_from_string(&path_str);
                    map = jets::map::insert(map, key_noun, val_noun);
                }
                Ok(cell(stud, map).into_noun())
            }
            DiskPail::Duct { data } => {
                let stud = Atom::from_str("duct").into_noun();
                let mut list = Noun::null();
                for idx in data.iter().rev() {
                    let path_str = ns.path_idx_to_str(idx).ok().flatten().unwrap_or_default();
                    let path_noun = path_noun_from_string(&path_str);
                    list = cell(path_noun, list).into_noun();
                }
                debug_noun_print("duct", list.clone());
                Ok(cell(stud, list).into_noun())
            }
            other => {
                let stud = Atom::from_str(other.typ_str()).into_noun();
                let noun_data = Atom::from_bytes(other.data()).into_noun();
                Ok(cell(stud, noun_data).into_noun())
            }
        }
    }
}

impl FromNoun for DiskPail {
    fn from_noun(noun: Noun) -> Result<Self, NounConversionError> {
        let cell = noun.into_cell().expect("invalid cell for pail");
        let stud = cell
            .head()
            .as_atom()
            .expect("invalid atom for stud")
            .to_owned()
            .into_noun();
        let typ = String::from_noun(stud).expect("invalid stud for pail");
        let tail = cell.tail().to_owned();

        // For known atom-only types, store raw bytes to avoid jam overhead
        match typ.as_str() {
            "cord" | "hoon" | "atom" => {
                if let Ok(atom) = tail.clone().into_atom() {
                    let bytes = atom.to_vec();
                    return Ok(match typ.as_str() {
                        "cord" => DiskPail::Text { data: bytes },
                        "hoon" => DiskPail::Hoon { data: bytes },
                        "atom" => DiskPail::Atom { data: bytes },
                        _ => unreachable!(),
                    });
                }
            }
            "wain" => {
                let wain = jets::list::to_vec(tail, |n| {
                    let Ok(atom) = n.clone().into_atom() else {
                        panic!("aaaa");
                    };
                    return atom.to_string();
                });
                eprintln!("wain {wain:?}");

                return Ok(DiskPail::Wain { data: wain });
            }
            "vase" => {
                let ns = NAMESPACE.get().expect("mising ns ").chow();
                let jimmed = jim(ns, tail);
                return Ok(DiskPail::Jim { typ, data: jimmed })

            }
            _ => {}
        }

        // Fallback: jam the tail for Jam variant
        let data = tail.jam();
        Ok(DiskPail::Jam { typ, data })
    }
}

pub struct Epic(pub(crate) HashMap<String, Saga>);

impl Epic {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn insert(&mut self, key: impl Into<String>, value: Saga) {
        self.0.insert(key.into(), value);
    }

    pub fn get(&self, key: &str) -> Option<&Saga> {
        self.0.get(key)
    }

    pub fn entry(&mut self, key: impl Into<String>) -> hash_map::Entry<'_, String, Saga> {
        self.0.entry(key.into())
    }
}

/// Slot data: parallel vectors of slot names and pails (struct of arrays).
///
/// Used with [`Note`] to specify what data to write at a path.

pub mod tale {
    use super::*;

    pub fn vase(pail: DiskPail) -> Tale {
        let mut tale = Tale::empty();
        tale.insert(slot::VASE, pail);
        tale
    }

    pub fn germ(germ: Vec<u8>) -> Tale {
        let mut tale = Tale::empty();
        tale.insert(
            slot::VASE,
            DiskPail::Jam {
                typ: stud::GERM.to_string(),
                data: germ,
            },
        );
        tale
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Gift {
    /// Which observation level(s) were updated.
    pub care: Care,
    /// The type of modification: Add, Dif, Del, or None if no change.
    pub mode: Option<Mode>,
    /// The resulting locks for all observation levels.
    pub ever: Ever,
    /// The slot names that were changed.
    pub slots: HashSet<String>,
    pub path: PathIdx,
}

impl IntoNoun for Gift {
    fn into_noun(&self) -> Result<Noun, NounConversionError> {
        let care = self.care.into_noun()?;
        let mode = match self.mode {
            Some(mode) => mode.into_noun()?,
            None => Noun::null(),
        };
        let ever = self.ever.into_noun()?;
        let slots = jets::set::from_iter_noun(self.slots.iter().map(|slot| {
            let slot = path_noun_from_string(&slot);
            slot
        }));
        Ok(cell(care, cell(mode, cell(ever, slots).into_noun()).into_noun()).into_noun())
    }
}

pub fn path_from_string(str: &str) -> Noun {
    let mut path = Noun::null();
    if str == "/" {
        return path;
    }
    let segments = path_to_segments(str);
    for seg in segments.into_iter().rev() {
        path = cell(Atom::from_str(seg).into_noun(), path).into_noun();
    }
    return path;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Tale {
    slots: Vec<String>,
    pails: Vec<DiskPail>,
}

impl Tale {

    pub fn to_noun(&self, ns: &Namespace) -> Result<Noun, NounConversionError> {
        let mut map = Noun::null();
        for (key, pail) in self.slots.iter().zip(self.pails.iter()) {
            let key = if key.starts_with("/") {
                path_noun_from_string(key)
            } else {
                Atom::from_str(key).into_noun()
            };
            let pail = pail.to_noun(ns)?;
            map = jets::map::insert(map, key, pail);
        }
        Ok(map)
    }

    pub fn get_slot(&self, slot: &str) -> Option<DiskPail> {
        let idx = self.slots.iter().position(|s| s == slot)?;
        let pail = self.pails.get(idx).cloned()?;
        Some(pail)
    }

    pub fn push(&mut self, slot: &str, pail: DiskPail) {
        self.slots.push(slot.to_string());
        self.pails.push(pail);
    }

    pub fn from_docs(lede: impl Into<String>, help: impl Into<String>) -> Self {
        let mut res = Self::empty();
        res.insert(slot::LEDE, DiskPail::Text { data:  lede.into().into_bytes() });
        res.insert(slot::HELP, DiskPail::Text { data:  help.into().into_bytes() });

        res
    }
}

impl FromNoun for Tale {
    fn from_noun(noun: Noun) -> Result<Self, NounConversionError> {
        let map = jets::map::to_hashmap(noun, |k, v| {
            let pax = path_string_from_noun(k);
            let pail = DiskPail::from_noun(v).unwrap();
            (pax, pail)
        });
        let mut slots = Vec::with_capacity(map.len());
        let mut pails = Vec::with_capacity(map.len());
        for (k, v) in map {
            slots.push(k);
            pails.push(v);
        }
        Ok(Self { slots, pails })
    }
}

impl Tale {
    pub fn empty() -> Self {
        Self {
            slots: Vec::new(),
            pails: Vec::new(),
        }
    }

    pub fn new(slots: Vec<String>, pails: Vec<DiskPail>) -> Self {
        Self { slots, pails }
    }

    pub fn insert(&mut self, key: impl Into<String>, value: DiskPail) {
        let key = key.into();
        if let Some(idx) = self.slots.iter().position(|s| s == &key) {
            self.pails[idx] = value;
        } else {
            self.slots.push(key);
            self.pails.push(value);
        }
    }

    pub fn get(&self, key: &str) -> Option<&DiskPail> {
        self.slots
            .iter()
            .position(|s| s == key)
            .map(|i| &self.pails[i])
    }

    pub fn remove(&mut self, key: &str) -> Option<DiskPail> {
        if let Some(idx) = self.slots.iter().position(|s| s == key) {
            self.slots.swap_remove(idx);
            Some(self.pails.swap_remove(idx))
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.slots.iter()
    }

    pub fn slots(&self) -> &[String] {
        &self.slots
    }

    pub fn pails(&self) -> &[DiskPail] {
        &self.pails
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &DiskPail)> {
        self.slots.iter().zip(self.pails.iter())
    }

    pub fn from_content(content: DiskPail) -> Self {
        Self {
            slots: vec![slot::CONTENT.to_string()],
            pails: vec![content],
        }
    }

    pub fn from_pot(content: DiskPail) -> Self {
        Self {
            slots: vec![slot::VASE.to_string()],
            pails: vec![content],
        }
    }

}
