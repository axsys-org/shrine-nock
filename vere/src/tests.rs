//! Tests for the safe nock runtime wrappers.
//!
//! These tests require the nock runtime to be initialized, so they must run
//! serially (not in parallel) to avoid conflicts with the global runtime state.

use crate::{cell, qual, trel, Atom, Cell, Runtime, NONE, NUL};
use std::sync::Mutex;

/// Global mutex to ensure tests run serially (runtime is not thread-safe).
static TEST_MUTEX: Mutex<()> = Mutex::new(());

/// Default loom size for tests (64MB).
const TEST_LOOM_SIZE: usize = 1 << 31;

/// Helper to run a test with the runtime initialized.
fn with_runtime<F, R>(f: F) -> R
where
    F: FnOnce(&Runtime) -> R,
{
    let _guard = TEST_MUTEX.lock().unwrap();
    let runtime = Runtime::new(TEST_LOOM_SIZE).expect("failed to initialize runtime");
    f(&runtime)
}

// =============================================================================
// Loobean tests
// =============================================================================

mod loob_tests {
    use crate::loob;

    #[test]
    fn test_loob_constants() {
        assert_eq!(loob::YES, 0);
        assert_eq!(loob::NO, 1);
    }

    #[test]
    fn test_loob_from_bool() {
        assert_eq!(loob::from_bool(true), loob::YES);
        assert_eq!(loob::from_bool(false), loob::NO);
    }

    #[test]
    fn test_loob_to_bool() {
        assert!(loob::to_bool(loob::YES));
        assert!(!loob::to_bool(loob::NO));
        // Any non-zero value is false
        assert!(!loob::to_bool(2));
        assert!(!loob::to_bool(255));
    }
}

// =============================================================================
// Atom tests
// =============================================================================

mod atom_tests {
    use super::*;

    #[test]
    fn test_atom_from_small_u32() {
        with_runtime(|_rt| {
            // Small values that fit in 31 bits (direct atoms)
            let atom = Atom::from_u32(0);
            assert_eq!(atom.as_u32(), Some(0));
            assert!(atom.as_noun().is_direct());

            let atom = Atom::from_u32(42);
            assert_eq!(atom.as_u32(), Some(42));
            assert!(atom.as_noun().is_direct());

            let atom = Atom::from_u32((1 << 31) - 1);
            assert_eq!(atom.as_u32(), Some((1 << 31) - 1));
            assert!(atom.as_noun().is_direct());
        });
    }

    #[test]
    fn test_atom_from_u64() {
        with_runtime(|_rt| {
            // Small value
            let atom = Atom::from_u64(123);
            assert_eq!(atom.as_u64(), Some(123));
            assert_eq!(atom.as_u32(), Some(123));

            // Value that doesn't fit in 32 bits
            let big_val: u64 = (1 << 40) + 12345;
            let atom = Atom::from_u64(big_val);
            assert_eq!(atom.as_u64(), Some(big_val));
            assert_eq!(atom.as_u32(), None); // Too big for u32

            // Max u64
            let atom = Atom::from_u64(u64::MAX);
            assert_eq!(atom.as_u64(), Some(u64::MAX));
        });
    }

    #[test]
    fn test_atom_from_bytes() {
        with_runtime(|_rt| {
            // Empty bytes = 0
            let atom = Atom::from_bytes(&[]);
            assert_eq!(atom.as_u32(), Some(0));

            // Single byte
            let atom = Atom::from_bytes(&[0x42]);
            assert_eq!(atom.as_u32(), Some(0x42));

            // Multiple bytes (LSB first)
            let atom = Atom::from_bytes(&[0x01, 0x02, 0x03, 0x04]);
            assert_eq!(atom.as_u32(), Some(0x04030201));

            // Bytes with leading zeros are trimmed
            let atom = Atom::from_bytes(&[0x42, 0x00, 0x00, 0x00]);
            assert_eq!(atom.as_u32(), Some(0x42));
        });
    }

    #[test]
    fn test_atom_from_string() {
        with_runtime(|_rt| {
            // "a" = 0x61
            let atom = Atom::from_str("a");
            assert_eq!(atom.as_u32(), Some(0x61));

            // "ab" = 0x6261 (LSB first)
            let atom = Atom::from_str("ab");
            assert_eq!(atom.as_u32(), Some(0x6261));

            // "test"
            let atom = Atom::from_str("test");
            let expected = u32::from_le_bytes(*b"test");
            assert_eq!(atom.as_u32(), Some(expected));
        });
    }

    #[test]
    fn test_atom_to_bytes() {
        with_runtime(|_rt| {
            let atom = Atom::from_u32(0x04030201);
            let bytes = atom.to_vec();
            assert_eq!(bytes, vec![0x01, 0x02, 0x03, 0x04]);

            // Zero
            let atom = Atom::from_u32(0);
            let bytes = atom.to_vec();
            assert!(bytes.is_empty());

            // Check byte_len
            let atom = Atom::from_u64(0xFFFF);
            assert_eq!(atom.byte_len(), 2);
        });
    }

    #[test]
    fn test_atom_bit_len() {
        with_runtime(|_rt| {
            assert_eq!(Atom::from_u32(0).bit_len(), 0);
            assert_eq!(Atom::from_u32(1).bit_len(), 1);
            assert_eq!(Atom::from_u32(2).bit_len(), 2);
            assert_eq!(Atom::from_u32(3).bit_len(), 2);
            assert_eq!(Atom::from_u32(255).bit_len(), 8);
            assert_eq!(Atom::from_u32(256).bit_len(), 9);
        });
    }

    #[test]
    fn test_atom_is_atom() {
        with_runtime(|_rt| {
            let atom = Atom::from_u32(42);
            assert!(atom.as_noun().is_atom());
            assert!(!atom.as_noun().is_cell());
        });
    }

    #[test]
    fn test_atom_clone() {
        with_runtime(|_rt| {
            let atom1 = Atom::from_u64(0x123456789ABCDEF0);
            let atom2 = atom1.clone();

            assert_eq!(atom1.as_u64(), atom2.as_u64());
            // Both should still be valid after clone
            assert_eq!(atom1.as_u64(), Some(0x123456789ABCDEF0));
            assert_eq!(atom2.as_u64(), Some(0x123456789ABCDEF0));
        });
    }

    #[test]
    fn test_atom_from_trait() {
        with_runtime(|_rt| {
            // From<u32>
            let atom: Atom = 42u32.into();
            assert_eq!(atom.as_u32(), Some(42));

            // From<u64>
            let atom: Atom = 42u64.into();
            assert_eq!(atom.as_u64(), Some(42));

            // Large u32 that needs u64 path
            let atom: Atom = 0xFFFFFFFFu32.into();
            assert_eq!(atom.as_u64(), Some(0xFFFFFFFF));
        });
    }
}

// =============================================================================
// Cell tests
// =============================================================================

mod cell_tests {
    use super::*;

    #[test]
    fn test_cell_new() {
        with_runtime(|_rt| {
            let head = Atom::from_u32(1).into_noun();
            let tail = Atom::from_u32(2).into_noun();
            let cell = Cell::new(head, tail);

            assert!(cell.as_noun().is_cell());
            assert!(!cell.as_noun().is_atom());
        });
    }

    #[test]
    fn test_cell_head_tail() {
        with_runtime(|_rt| {
            let head = Atom::from_u32(42).into_noun();
            let tail = Atom::from_u32(99).into_noun();
            let cell = Cell::new(head, tail);

            // Access head and tail
            let h = cell.head();
            let t = cell.tail();

            assert!(h.is_atom());
            assert!(t.is_atom());

            // Convert to owned and check values
            let h_owned = h.to_owned();
            let t_owned = t.to_owned();

            assert_eq!(h_owned.as_atom().unwrap().as_u32(), Some(42));
            assert_eq!(t_owned.as_atom().unwrap().as_u32(), Some(99));
        });
    }

    #[test]
    fn test_cell_nested() {
        with_runtime(|_rt| {
            // Create [[1 2] 3]
            let inner = Cell::new(
                Atom::from_u32(1).into_noun(),
                Atom::from_u32(2).into_noun(),
            );
            let outer = Cell::new(inner.into_noun(), Atom::from_u32(3).into_noun());

            // Check structure
            assert!(outer.head().is_cell());
            assert!(outer.tail().is_atom());
        });
    }

    #[test]
    fn test_cell_trel() {
        with_runtime(|_rt| {
            // Create [1 2 3]
            let cell = Cell::trel(
                Atom::from_u32(1).into_noun(),
                Atom::from_u32(2).into_noun(),
                Atom::from_u32(3).into_noun(),
            );

            // Should be [1 [2 3]]
            let head = cell.head();
            assert!(head.is_atom());

            let tail_ref = cell.tail();
            assert!(tail_ref.is_cell());
        });
    }

    #[test]
    fn test_cell_qual() {
        with_runtime(|_rt| {
            // Create [1 2 3 4]
            let cell = Cell::qual(
                Atom::from_u32(1).into_noun(),
                Atom::from_u32(2).into_noun(),
                Atom::from_u32(3).into_noun(),
                Atom::from_u32(4).into_noun(),
            );

            // Should be [1 [2 [3 4]]]
            let head = cell.head();
            assert!(head.is_atom());

            let tail = cell.tail();
            assert!(tail.is_cell());
        });
    }

    #[test]
    fn test_cell_into_parts() {
        with_runtime(|_rt| {
            let cell = Cell::new(
                Atom::from_u32(10).into_noun(),
                Atom::from_u32(20).into_noun(),
            );

            let (head, tail) = cell.into_parts();

            assert_eq!(head.as_atom().unwrap().as_u32(), Some(10));
            assert_eq!(tail.as_atom().unwrap().as_u32(), Some(20));
        });
    }

    #[test]
    fn test_cell_clone() {
        with_runtime(|_rt| {
            let cell1 = Cell::new(
                Atom::from_u32(100).into_noun(),
                Atom::from_u32(200).into_noun(),
            );
            let cell2 = cell1.clone();

            // Both should have same structure
            let h1 = cell1.head().to_owned();
            let h2 = cell2.head().to_owned();

            assert_eq!(
                h1.as_atom().unwrap().as_u32(),
                h2.as_atom().unwrap().as_u32()
            );
        });
    }

    #[test]
    fn test_cell_convenience_functions() {
        with_runtime(|_rt| {
            // Test the module-level convenience functions
            let c = cell(
                Atom::from_u32(1).into_noun(),
                Atom::from_u32(2).into_noun(),
            );
            assert!(c.as_noun().is_cell());

            let t = trel(
                Atom::from_u32(1).into_noun(),
                Atom::from_u32(2).into_noun(),
                Atom::from_u32(3).into_noun(),
            );
            assert!(t.as_noun().is_cell());

            let q = qual(
                Atom::from_u32(1).into_noun(),
                Atom::from_u32(2).into_noun(),
                Atom::from_u32(3).into_noun(),
                Atom::from_u32(4).into_noun(),
            );
            assert!(q.as_noun().is_cell());
        });
    }
}

// =============================================================================
// Noun tests
// =============================================================================

mod noun_tests {
    use super::*;

    #[test]
    fn test_noun_type_checks() {
        with_runtime(|_rt| {
            // Atom
            let atom = Atom::from_u32(42).into_noun();
            assert!(atom.is_atom());
            assert!(!atom.is_cell());
            assert!(atom.is_direct());
            assert!(!atom.is_indirect());

            // Cell
            let cell = Cell::new(
                Atom::from_u32(1).into_noun(),
                Atom::from_u32(2).into_noun(),
            )
            .into_noun();
            assert!(!cell.is_atom());
            assert!(cell.is_cell());
        });
    }

    #[test]
    fn test_noun_as_atom_cell() {
        with_runtime(|_rt| {
            let atom = Atom::from_u32(42).into_noun();
            assert!(atom.as_atom().is_some());
            assert!(atom.as_cell().is_none());

            let cell = Cell::new(
                Atom::from_u32(1).into_noun(),
                Atom::from_u32(2).into_noun(),
            )
            .into_noun();
            assert!(cell.as_atom().is_none());
            assert!(cell.as_cell().is_some());
        });
    }

    #[test]
    fn test_noun_into_atom_cell() {
        with_runtime(|_rt| {
            let noun = Atom::from_u32(42).into_noun();
            let atom = noun.into_atom();
            assert!(atom.is_ok());

            let noun = Cell::new(
                Atom::from_u32(1).into_noun(),
                Atom::from_u32(2).into_noun(),
            )
            .into_noun();
            let cell = noun.into_cell();
            assert!(cell.is_ok());
        });
    }

    #[test]
    fn test_noun_into_wrong_type() {
        with_runtime(|_rt| {
            let noun = Atom::from_u32(42).into_noun();
            let result = noun.into_cell();
            assert!(result.is_err());
            // Get the noun back from the error
            let noun = result.unwrap_err();
            assert!(noun.is_atom());

            let noun = Cell::new(
                Atom::from_u32(1).into_noun(),
                Atom::from_u32(2).into_noun(),
            )
            .into_noun();
            let result = noun.into_atom();
            assert!(result.is_err());
        });
    }

    #[test]
    fn test_noun_equality() {
        with_runtime(|_rt| {
            // Same value atoms are equal
            let a1 = Atom::from_u32(42).into_noun();
            let a2 = Atom::from_u32(42).into_noun();
            assert!(a1.equals(&a2));
            assert_eq!(a1, a2);

            // Different value atoms are not equal
            let a3 = Atom::from_u32(99).into_noun();
            assert!(!a1.equals(&a3));
            assert_ne!(a1, a3);

            // Same structure cells are equal
            let c1 = Cell::new(
                Atom::from_u32(1).into_noun(),
                Atom::from_u32(2).into_noun(),
            )
            .into_noun();
            let c2 = Cell::new(
                Atom::from_u32(1).into_noun(),
                Atom::from_u32(2).into_noun(),
            )
            .into_noun();
            assert!(c1.equals(&c2));

            // Different structure cells are not equal
            let c3 = Cell::new(
                Atom::from_u32(1).into_noun(),
                Atom::from_u32(3).into_noun(),
            )
            .into_noun();
            assert!(!c1.equals(&c3));
        });
    }

    #[test]
    fn test_noun_mug() {
        with_runtime(|_rt| {
            // Same nouns should have same mug
            let a1 = Atom::from_u32(42).into_noun();
            let a2 = Atom::from_u32(42).into_noun();
            assert_eq!(a1.mug(), a2.mug());

            // Different nouns may have different mugs
            let a3 = Atom::from_u32(99).into_noun();
            // Note: collision is possible but unlikely for different values
            let _ = a3.mug(); // Just check it doesn't crash
        });
    }

    #[test]
    fn test_noun_at() {
        with_runtime(|_rt| {
            // Create [1 [2 3]]
            let noun = Cell::trel(
                Atom::from_u32(1).into_noun(),
                Atom::from_u32(2).into_noun(),
                Atom::from_u32(3).into_noun(),
            )
            .into_noun();

            // Axis 1 = whole noun
            let whole = noun.at(1);
            assert!(whole.is_some());
            assert!(whole.unwrap().is_cell());

            // Axis 2 = head (1)
            let head = noun.at(2);
            assert!(head.is_some());
            assert!(head.unwrap().is_atom());

            // Axis 3 = tail ([2 3])
            let tail = noun.at(3);
            assert!(tail.is_some());
            assert!(tail.unwrap().is_cell());

            // Axis 6 = head of tail (2)
            let hot = noun.at(6);
            assert!(hot.is_some());

            // Axis 7 = tail of tail (3)
            let tot = noun.at(7);
            assert!(tot.is_some());

            // Invalid axis on atom
            let atom = Atom::from_u32(42).into_noun();
            assert!(atom.at(2).is_none());
        });
    }

    #[test]
    fn test_noun_clone() {
        with_runtime(|_rt| {
            let noun1 = Cell::new(
                Atom::from_u32(42).into_noun(),
                Atom::from_u32(99).into_noun(),
            )
            .into_noun();
            let noun2 = noun1.clone();

            assert!(noun1.equals(&noun2));
        });
    }

    #[test]
    fn test_constants() {
        assert_eq!(NONE, 0xffffffff);
        assert_eq!(NUL, 0);
    }
}

// =============================================================================
// Runtime tests
// =============================================================================

mod runtime_tests {
    use super::*;
    use crate::RuntimeError;

    #[test]
    fn test_runtime_initialization() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let runtime = Runtime::new(TEST_LOOM_SIZE);
        assert!(runtime.is_ok());
        // Runtime is dropped here
    }

    #[test]
    fn test_runtime_double_init_fails() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let rt1 = Runtime::new(TEST_LOOM_SIZE);
        assert!(rt1.is_ok());

        let rt2 = Runtime::new(TEST_LOOM_SIZE);
        assert!(matches!(rt2, Err(RuntimeError::AlreadyInitialized)));

        // Drop rt1 first
        drop(rt1);
    }

    #[test]
    fn test_runtime_is_initialized() {
        let _guard = TEST_MUTEX.lock().unwrap();

        assert!(!Runtime::is_initialized());

        let rt = Runtime::new(TEST_LOOM_SIZE).unwrap();
        assert!(Runtime::is_initialized());

        drop(rt);
        assert!(!Runtime::is_initialized());
    }

    #[test]
    fn test_runtime_nock_identity() {
        with_runtime(|rt| {
            // Nock formula [0 1] = identity (return subject)
            let subject = Atom::from_u32(42).into_noun();
            let formula = Cell::new(
                Atom::from_u32(0).into_noun(),
                Atom::from_u32(1).into_noun(),
            )
            .into_noun();

            let result = rt.nock(subject, formula);
            assert!(result.is_atom());
            assert_eq!(result.as_atom().unwrap().as_u32(), Some(42));
        });
    }

    #[test]
    fn test_runtime_nock_constant() {
        with_runtime(|rt| {
            // Nock formula [1 99] = constant (return 99)
            let subject = Atom::from_u32(0).into_noun();
            let formula = Cell::new(
                Atom::from_u32(1).into_noun(),
                Atom::from_u32(99).into_noun(),
            )
            .into_noun();

            let result = rt.nock(subject, formula);
            assert!(result.is_atom());
            assert_eq!(result.as_atom().unwrap().as_u32(), Some(99));
        });
    }

    #[test]
    fn test_runtime_nock_slot() {
        with_runtime(|rt| {
            // Subject: [10 20]
            // Formula: [0 2] = get axis 2 (head)
            let subject = Cell::new(
                Atom::from_u32(10).into_noun(),
                Atom::from_u32(20).into_noun(),
            )
            .into_noun();
            let formula = Cell::new(
                Atom::from_u32(0).into_noun(),
                Atom::from_u32(2).into_noun(),
            )
            .into_noun();

            let result = rt.nock(subject, formula);
            assert_eq!(result.as_atom().unwrap().as_u32(), Some(10));
        });
    }

    #[test]
    fn test_runtime_nock_increment() {
        with_runtime(|rt| {
            // Subject: 5
            // Formula: [4 [0 1]] = increment subject
            let subject = Atom::from_u32(5).into_noun();
            let formula = Cell::new(
                Atom::from_u32(4).into_noun(),
                Cell::new(
                    Atom::from_u32(0).into_noun(),
                    Atom::from_u32(1).into_noun(),
                )
                .into_noun(),
            )
            .into_noun();

            let result = rt.nock(subject, formula);
            assert_eq!(result.as_atom().unwrap().as_u32(), Some(6));
        });
    }

    #[test]
    fn test_runtime_nock_cell_test() {
        with_runtime(|rt| {
            // Formula: [3 [0 1]] = test if subject is cell
            // Subject is atom -> result is 1 (no)
            let subject = Atom::from_u32(42).into_noun();
            let formula = Cell::new(
                Atom::from_u32(3).into_noun(),
                Cell::new(
                    Atom::from_u32(0).into_noun(),
                    Atom::from_u32(1).into_noun(),
                )
                .into_noun(),
            )
            .into_noun();

            let result = rt.nock(subject, formula);
            assert_eq!(result.as_atom().unwrap().as_u32(), Some(1)); // loobean no

            // Subject is cell -> result is 0 (yes)
            let subject = Cell::new(
                Atom::from_u32(1).into_noun(),
                Atom::from_u32(2).into_noun(),
            )
            .into_noun();
            let formula = Cell::new(
                Atom::from_u32(3).into_noun(),
                Cell::new(
                    Atom::from_u32(0).into_noun(),
                    Atom::from_u32(1).into_noun(),
                )
                .into_noun(),
            )
            .into_noun();

            let result = rt.nock(subject, formula);
            assert_eq!(result.as_atom().unwrap().as_u32(), Some(0)); // loobean yes
        });
    }

    #[test]
    fn test_runtime_nock_equals() {
        with_runtime(|rt| {
            // Formula: [5 [0 2] [0 3]] = test equality of head and tail
            // Subject: [42 42] -> equal -> 0 (yes)
            let subject = Cell::new(
                Atom::from_u32(42).into_noun(),
                Atom::from_u32(42).into_noun(),
            )
            .into_noun();
            let formula = Cell::new(
                Atom::from_u32(5).into_noun(),
                Cell::new(
                    Cell::new(
                        Atom::from_u32(0).into_noun(),
                        Atom::from_u32(2).into_noun(),
                    )
                    .into_noun(),
                    Cell::new(
                        Atom::from_u32(0).into_noun(),
                        Atom::from_u32(3).into_noun(),
                    )
                    .into_noun(),
                )
                .into_noun(),
            )
            .into_noun();

            let result = rt.nock(subject, formula);
            assert_eq!(result.as_atom().unwrap().as_u32(), Some(0)); // equal

            // Subject: [42 99] -> not equal -> 1 (no)
            let subject = Cell::new(
                Atom::from_u32(42).into_noun(),
                Atom::from_u32(99).into_noun(),
            )
            .into_noun();
            let formula = Cell::new(
                Atom::from_u32(5).into_noun(),
                Cell::new(
                    Cell::new(
                        Atom::from_u32(0).into_noun(),
                        Atom::from_u32(2).into_noun(),
                    )
                    .into_noun(),
                    Cell::new(
                        Atom::from_u32(0).into_noun(),
                        Atom::from_u32(3).into_noun(),
                    )
                    .into_noun(),
                )
                .into_noun(),
            )
            .into_noun();

            let result = rt.nock(subject, formula);
            assert_eq!(result.as_atom().unwrap().as_u32(), Some(1)); // not equal
        });
    }

    #[test]
    fn test_runtime_pack() {
        with_runtime(|rt| {
            // Just ensure pack doesn't crash
            let _ = rt.pack();
        });
    }

    #[test]
    fn test_runtime_reclaim() {
        with_runtime(|rt| {
            // Just ensure reclaim doesn't crash
            rt.reclaim();
        });
    }

    #[test]
    fn test_runtime_wish() {
        with_runtime(|rt| {
            let result = rt.wish("(add 1 2)");
            assert_eq!(result.as_atom().unwrap().as_u32(), Some(3));
        });
    }
}

// =============================================================================
// NounRef / AtomRef / CellRef tests
// =============================================================================

mod ref_tests {
    use vere_sys::{u3a_lose, u3a_luse};

    use super::*;

    #[test]
    fn test_noun_ref_to_owned() {
        with_runtime(|_rt| {
            let cell = Cell::new(
                Atom::from_u32(1).into_noun(),
                Atom::from_u32(2).into_noun(),
            );

            let head_ref = cell.head();
            let head_owned = head_ref.to_owned();

            assert!(head_owned.is_atom());
            assert_eq!(head_owned.as_atom().unwrap().as_u32(), Some(1));
        });
    }

    #[test]
    fn test_atom_ref() {
        with_runtime(|_rt| {
            let noun = Atom::from_u32(42).into_noun();
            let atom_ref = noun.as_atom().unwrap();

            assert_eq!(atom_ref.as_u32(), Some(42));
            assert_eq!(atom_ref.as_raw(), noun.as_raw());

            let owned = atom_ref.to_owned();
            assert_eq!(owned.as_u32(), Some(42));
        });
    }

    // #[test]
    fn test_cell_ref() {
        with_runtime(|_rt| {
            let noun = Cell::new(
                Atom::from_u32(10).into_noun(),
                Atom::from_u32(20).into_noun(),
            )
            .into_noun();


            unsafe {
            u3a_luse(noun.as_raw());
            }

            let cell_ref = noun.as_cell().unwrap();
            assert_eq!(cell_ref.as_raw(), noun.as_raw());

            let h = cell_ref.head();
            let t = cell_ref.tail();
            assert!(h.is_atom());
            assert!(t.is_atom());

            let owned = cell_ref.to_owned();
            let new_noun = owned.clone().into_noun();
            let ref_cnt = new_noun.refcount();
            eprintln!("refcount {:?}", new_noun.refcount());
            assert!(owned.as_noun().is_cell());
        });
    }
}

// =============================================================================
// Debug formatting tests
// =============================================================================

mod debug_tests {
    use vere_sys::u3m_p;

    use super::*;

    #[test]
    fn test_atom_debug() {
        with_runtime(|_rt| {
            let atom = Atom::from_u32(42);
            let debug_str = format!("{:?}", atom);
            assert!(debug_str.contains("Atom"));
            assert!(debug_str.contains("42"));
        });
    }

    #[test]
    fn test_cell_debug() {
        with_runtime(|_rt| {
            let cell = Cell::new(
                Atom::from_u32(1).into_noun(),
                Atom::from_u32(2).into_noun(),
            );
            let debug_str = format!("{:?}", cell);
            assert!(debug_str.contains("Cell"));
        });
    }

    #[test]
    fn test_noun_debug() {
        with_runtime(|_rt| {
            let noun = Atom::from_u32(42).into_noun();
            let debug_str = format!("{:?}", noun);
            assert!(debug_str.contains("Atom"));
        });
    }


    #[test]
    fn test_wish() {
        with_runtime(|rt| {
            let pax = rt.wish("(pave:t /foo/1/0x3)");
            unsafe {
                u3m_p("testx".as_ptr() as *const i8, pax.clone().into_raw());
            }
            let expect = rt.wish("[%foo [%ud 1] [%ux 3] ~]");
            assert_eq!(pax, expect);
        })
    }
}
