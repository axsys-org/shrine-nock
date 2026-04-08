//! Runtime initialization and management for the Urbit nock interpreter.
//!
//! The runtime must be initialized before any noun operations can be performed.
//! This module provides a safe interface for managing the runtime lifecycle.


use libc::c_char;
use tracing::info;
use vere_sys::{u3m_p, u3m_pack, u3v_Home, u3v_home};

use crate::noun::{self, Noun};
use std::ffi::{CStr, CString};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

/// Global flag to track if the runtime is initialized.
static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// The Urbit nock runtime.
///
/// Only one runtime can exist at a time. The runtime must be initialized
/// before any noun operations can be performed.
///
/// This type is neither `Send` nor `Sync` because the underlying C runtime
/// uses thread-local storage and is not thread-safe.
///
/// # Example
///
/// ```ignore
/// let runtime = Runtime::new(1 << 30)?; // 1GB loom
/// // ... perform noun operations ...
/// // Runtime is cleaned up when dropped
/// ```
pub struct Runtime {
    // Marker to make Runtime !Send and !Sync
    // _marker: PhantomData<UnsafeCell<()>>,
}



/// Error type for runtime operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    /// The runtime is already initialized.
    AlreadyInitialized,
    /// The runtime is not initialized.
    NotInitialized,
    /// The runtime failed to initialize.
    InitFailed(String),
    /// The specified path is invalid.
    InvalidPath(String),
    /// Nock execution failed.
    NockFailed,
    /// A bail occurred during execution.
    Bail(BailMote),
    /// The ivory pill is invalid.
    InvalidPill,
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotInitialized => write!(f, "runtime not initialized"),
            Self::AlreadyInitialized => write!(f, "runtime already initialized"),
            Self::InitFailed(msg) => write!(f, "runtime initialization failed: {}", msg),
            Self::InvalidPath(path) => write!(f, "invalid path: {}", path),
            Self::NockFailed => write!(f, "nock execution failed"),
            Self::Bail(mote) => write!(f, "bail: {:?}", mote),
            Self::InvalidPill => write!(f, "invalid stem pill"),
        }
    }
}

impl std::error::Error for RuntimeError {}

/// Bail motes - reasons for computation failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BailMote {
    /// Semantic failure (e.g., invalid input).
    Exit,
    /// Bad cryptographic operation.
    Evil,
    /// Interrupted.
    Intr,
    /// Execution failure.
    Fail,
    /// Assert failure.
    Foul,
    /// Network block.
    Need,
    /// Out of memory.
    Meme,
    /// Timed out.
    Time,
    /// Assertion failure.
    Oops,
    /// Unknown mote.
    Unknown(u32),
}

impl From<u32> for BailMote {
    fn from(mote: u32) -> Self {
        // Convert 4-character mote to known variants
        // These are little-endian 4-char strings
        match mote {
            m if m == mote_from_str("exit") => Self::Exit,
            m if m == mote_from_str("evil") => Self::Evil,
            m if m == mote_from_str("intr") => Self::Intr,
            m if m == mote_from_str("fail") => Self::Fail,
            m if m == mote_from_str("foul") => Self::Foul,
            m if m == mote_from_str("need") => Self::Need,
            m if m == mote_from_str("meme") => Self::Meme,
            m if m == mote_from_str("time") => Self::Time,
            m if m == mote_from_str("oops") => Self::Oops,
            _ => Self::Unknown(mote),
        }
    }
}

pub fn wish(str: &str) -> Noun {
    let txt = CString::new(str).unwrap();
    // SAFETY: Runtime is initialized, str is a valid C string
    let result = unsafe { vere_sys::u3v_wish(txt.as_ptr() as *mut _) };
    // SAFETY: u3v_wish returns a valid noun
    unsafe { Noun::from_raw(result) }
}

pub fn debug_noun_print(str: &str, noun: Noun) {
    unsafe {
        let cstr = CString::new(str).unwrap();
        u3m_p(cstr.as_ptr(), noun.as_raw());
    }
}


pub extern "C" fn ivory_slog(non: u32) {
    unsafe {
        let cstr = CString::new("slog").unwrap();
        u3m_p(cstr.as_ptr(), non);
    }
}

/// Convert a 4-character string to a mote (little-endian).
pub const fn mote_from_str(s: &str) -> u32 {
    let bytes = s.as_bytes();
    let b0 = if !bytes.is_empty() { bytes[0] as u32 } else { 0 };
    let b1 = if bytes.len() > 1 { bytes[1] as u32 } else { 0 };
    let b2 = if bytes.len() > 2 { bytes[2] as u32 } else { 0 };
    let b3 = if bytes.len() > 3 { bytes[3] as u32 } else { 0 };
    b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
}

pub unsafe extern "C" fn stderr_log(str: *mut c_char) {
    unsafe {
        let string = CStr::from_ptr(str).to_string_lossy().to_string();

        // let fil = stderr();
        // fprintf(S, str);
        if string.contains("fund") {
        //   tracing::trace!("{}", string);

        } else {
          info!("{}", string);

        }
    }

}

pub const IVORY_PILL: &[u8] = include_bytes!("./start.germ");

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum CoreAxis {
    Load = 4,
    Peek = 22,
    Poke = 23,
    Wish = 10,
}

impl Runtime {
    /// Initialize a new runtime with the given loom size (in bytes).
    ///
    /// This is a "lite" initialization without checkpointing/persistence.
    ///
    /// # Arguments
    ///
    /// * `loom_size` - Size of the loom in bytes. Must be a power of 2.
    ///
    /// # Errors
    ///
    /// Returns an error if the runtime is already initialized.
    pub fn new(loom_size: usize) -> Result<Self, RuntimeError> {
        // Try to atomically set initialized flag
        if INITIALIZED
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(RuntimeError::AlreadyInitialized);
        }

        unsafe {
            vere_sys::u3o_Config.wag_w |= vere_sys::u3o_flag_u3o_hashless;
            vere_sys::u3o_Config.wag_w |= vere_sys::u3o_flag_u3o_trace;
            vere_sys::u3v_step_trace();

            vere_sys::u3o_Config.slog_f = Some(ivory_slog);
            vere_sys::u3o_Config.stderr_log_f = Some(stderr_log);
            vere_sys::u3m_boot_lite(loom_size);
        }
        println!("shrine: booted");
        let ivory = noun::cue_xeno(IVORY_PILL).ok_or(RuntimeError::InvalidPill)?;


        // SAFETY: We've ensured single initialization via atomic flag
        unsafe {
            let res = vere_sys::u3v_boot_germ(ivory.into_raw());
            if res != 0 {
                println!("failed to boot");
            }
        }

        Ok(Runtime {})
    }

    pub fn get() -> Result<Self, RuntimeError> {
        let init = INITIALIZED.load(Ordering::SeqCst);
        if !init {
            return Err(RuntimeError::NotInitialized);
        };
        Ok(Self {})
    }

    /// Initialize a new runtime with persistence at the given pier directory.
    ///
    /// # Arguments
    ///
    /// * `pier_path` - Path to the pier directory.
    /// * `loom_size` - Size of the loom in bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the runtime is already initialized or the path is invalid.
    pub fn with_pier(pier_path: &Path, loom_size: usize) -> Result<Self, RuntimeError> {
        if INITIALIZED
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(RuntimeError::AlreadyInitialized);
        }

        let path_str = pier_path
            .to_str()
            .ok_or_else(|| RuntimeError::InvalidPath(pier_path.display().to_string()))?;

        let c_path = CString::new(path_str)
            .map_err(|_| RuntimeError::InvalidPath(path_str.to_string()))?;

        // SAFETY: We've ensured single initialization and path is valid C string
        unsafe {
            vere_sys::u3m_boot(c_path.as_ptr() as *mut _, loom_size);
        }

        Ok(Self {})
    }

    pub fn set_roof(&mut self, roof: unsafe extern "C" fn(u32, u32, u32) -> u32) {
        unsafe {
            vere_sys::u3a_set_roof(Some(roof));
        }
    }

    /// Check if the runtime is initialized.
    pub fn is_initialized() -> bool {
        INITIALIZED.load(Ordering::SeqCst)
    }

    /// Execute nock: compute `*[subject formula]`.
    ///
    /// # Arguments
    ///
    /// * `subject` - The subject noun.
    /// * `formula` - The formula to evaluate.
    ///
    /// # Returns
    ///
    /// The result of the nock computation.
    pub fn nock(&self, subject: Noun, formula: Noun) -> Noun {
        let bus = subject.into_raw();
        let fol = formula.into_raw();
        // SAFETY: Runtime is initialized, inputs are valid nouns
        let result = unsafe { vere_sys::u3n_nock_on(bus, fol) };
        // SAFETY: u3n_nock_on returns a valid noun with appropriate refcount
        unsafe { Noun::from_raw(result) }
    }

    /// Slam a gate (call a function): compute `(gate sample)`.
    ///
    /// # Arguments
    ///
    /// * `gate` - The gate (function) to call.
    /// * `sample` - The sample (argument) to pass.
    ///
    /// # Returns
    ///
    /// The result of the gate invocation.
    pub fn slam(&self, gate: Noun, sample: Noun) -> Noun {
        let gat = gate.into_raw();
        let sam = sample.into_raw();
        // SAFETY: Runtime is initialized, inputs are valid nouns
        let result = unsafe { vere_sys::u3n_slam_on(gat, sam) };
        // SAFETY: u3n_slam_on returns a valid noun
        unsafe { Noun::from_raw(result) }
    }

    /// Kick a gate without changing the sample.
    ///
    /// # Arguments
    ///
    /// * `gate` - The gate to kick.
    ///
    /// # Returns
    ///
    /// The result of kicking the gate.
    pub fn kick(&self, gate: Noun) -> Noun {
        let gat = gate.into_raw();
        // SAFETY: Runtime is initialized, input is valid noun
        let result = unsafe { vere_sys::u3n_kick_on(gat) };
        // SAFETY: u3n_kick_on returns a valid noun
        unsafe { Noun::from_raw(result) }
    }

    /// Execute nock with soft (catching) semantics.
    ///
    /// Returns a cell `[tag result]` where:
    /// - `[%$ result]` on success
    /// - `[%error (list tank)]` on failure
    ///
    /// # Arguments
    ///
    /// * `timeout_ms` - Timeout in milliseconds (0 for no timeout).
    /// * `subject` - The subject noun.
    /// * `formula` - The formula to evaluate.
    pub fn soft_nock(&self, timeout_ms: u32, subject: Noun, formula: Noun) -> Noun {
        let bus = subject.into_raw();
        let fol = formula.into_raw();
        // SAFETY: Runtime is initialized, inputs are valid nouns
        let result = unsafe { vere_sys::u3m_soft_nock(bus, fol) };
        let _ = timeout_ms; // TODO: implement timeout via u3m_soft
        // SAFETY: u3m_soft_nock returns a valid noun
        unsafe { Noun::from_raw(result) }
    }

    pub fn soft_slam_opt(&self, _timeout_ms: u32, gat: Noun, sam: Noun) -> Option<Noun> {
        let res = self.soft_slam(gat, sam);
        let cell = res.into_cell().ok()?;
        if cell.head().as_raw() == 0 {
            return Some(cell.tail().to_owned());
        };
        return None;
    }

    /// Execute a slam with soft (catching) semantics.
    pub fn soft_slam(&self, gate: Noun, sample: Noun) -> Noun {
        let gat = gate.into_raw();
        let sam = sample.into_raw();
        // SAFETY: Runtime is initialized, inputs are valid nouns
        let result = unsafe { vere_sys::u3m_soft_slam(gat, sam) };
        // SAFETY: u3m_soft_slam returns a valid noun
        unsafe { Noun::from_raw(result) }
    }

    /// Trigger a garbage collection pass.
    pub fn gc(&self, root: &Noun) {
        // SAFETY: Runtime is initialized, root is a valid noun
        // u3m_grab takes a variable number of roots terminated by u3_none
        unsafe {
            vere_sys::u3m_grab(root.as_raw(), crate::noun::NONE);
        }
    }

    /// Clear persistent caches to reclaim memory.
    pub fn reclaim(&self) {
        // SAFETY: Runtime is initialized
        unsafe {
            vere_sys::u3m_reclaim();
        }
    }

    /// Compact memory (defragment).
    ///
    /// Returns the change in free space (words).
    pub fn pack(&self) -> u32 {
        // SAFETY: Runtime is initialized
        unsafe { vere_sys::u3m_pack() }
    }

    /// Save a checkpoint.
    pub fn save(&self) {
        // SAFETY: Runtime is initialized
        unsafe {
            vere_sys::u3m_save();
        }
    }

    /// Pretty-print a noun to a string.
    pub fn pretty(&self, noun: &Noun) -> String {
        // SAFETY: Runtime is initialized, noun is valid
        let c_str = unsafe { vere_sys::u3m_pretty(noun.as_raw()) };
        if c_str.is_null() {
            return String::new();
        }
        // SAFETY: u3m_pretty returns a valid C string allocated by the runtime.
        // We need to use the runtime's free function (u3a_free) to deallocate.
        let result = unsafe {
            let s = std::ffi::CStr::from_ptr(c_str).to_string_lossy().into_owned();
            libc::free(c_str as *mut _);
            // vere_sys::c3_free(c_str as *mut _);
            s
        };
        result
    }

    ///  Execute hoon code
    ///
    pub fn wish(&self, str: &str) -> Noun {
        let txt = CString::new(str).unwrap();
        // SAFETY: Runtime is initialized, str is a valid C string
        let result = unsafe { vere_sys::u3v_wish(txt.as_ptr() as *mut _) };
        // SAFETY: u3v_wish returns a valid noun
        unsafe { Noun::from_raw(result) }
    }

    pub fn poke(&self, tim_w: Option<u32>, ovo: Noun) -> Option<Noun> {
        unsafe {
            // let dif_w = u3m_pack();
            // eprintln!("pack {dif_w:?}");

            let tim_w = tim_w.unwrap_or(0);
            let result = vere_sys::u3v_poke_ivory(tim_w, ovo.into_raw());
            Noun::from_weak(result)
        }
    }

    pub fn get_arvo_noun(&self) -> Noun {
        unsafe {
            let res = Noun::from_raw((*u3v_Home).arv_u.roc);
            res.gain();
            return res;
        }
    }



}

impl Runtime {
}

impl Drop for Runtime {
    fn drop(&mut self) {
        // SAFETY: We're the only holder of the runtime
        unsafe {
            vere_sys::u3m_stop();
        }
        INITIALIZED.store(false, Ordering::SeqCst);
    }
}



