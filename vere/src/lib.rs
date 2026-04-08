#![feature(negative_impls)]
//! Safe Rust bindings for the Urbit nock runtime.
//!
//! This crate provides safe wrappers around the low-level C bindings in `vere-sys`,
//! offering idiomatic Rust types for working with nouns, atoms, and cells.

pub use vere_sys as sys;

pub mod jets;
pub mod food;
pub mod slab;
pub mod traits;

mod noun;
mod runtime;

pub use noun::{Atom, AtomRef, Cell, CellRef, NONE, NUL, Noun, NounRef, cell, cue, qual, trel};
pub use runtime::{
    BailMote, IVORY_PILL, Runtime, RuntimeError, debug_noun_print, mote_from_str, wish,
};

/// Loobean constants matching the C runtime.
pub mod loob {
    /// Yes / true in loobean logic (0).
    pub const YES: u8 = 0;
    /// No / false in loobean logic (1).
    pub const NO: u8 = 1;

    /// Convert a Rust bool to a loobean.
    #[inline]
    pub const fn from_bool(b: bool) -> u8 {
        if b { YES } else { NO }
    }

    /// Convert a loobean to a Rust bool.
    #[inline]
    pub const fn to_bool(o: u8) -> bool {
        o == YES
    }
}

pub mod axal;

#[cfg(test)]
mod tests;
