use milton::{Noun, traits::{FromNoun, IntoNoun, NounConversionError}};

/// Fekiseconds: a binary time unit equal to 2^-50 seconds (~0.888 picoseconds).
///
/// Designed for 128-bit monotonic timestamps with epoch at the Apollo 11 landing:
/// 1969-07-20 20:17:40 TAI.
///
/// All conversions use integer-only fixed-point arithmetic. No floating point.

/// One Fekisecond = 2^-50 seconds ≈ 8.881784197e-16 seconds
///
/// The internal representation is an i128 counting Fekiseconds from epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Feki(pub i128);

// =============================================================================
// Conversion strategy
// =============================================================================
//
// To convert from SI units to Fekiseconds without floating point, we use the
// identity:
//
//   Feki = value_in_seconds * 2^50
//
// For sub-second SI units (milli, micro, nano, pico):
//
//   Feki = value * 2^50 / 10^N
//
// where N is the SI exponent (3 for milli, 6 for micro, etc.)
//
// We rearrange this as a fixed-point multiply-then-shift to avoid intermediate
// overflow where possible:
//
//   Feki = (value * K) >> S
//
// where K = round(2^(50+S) / 10^N) for a chosen shift S.
//
// For the reverse direction:
//
//   value = (Feki * M) >> S
//
// where M = round(10^N * 2^S / 2^50) = round(10^N * 2^(S-50)).
//
// We choose S to balance precision against overflow. Since our timestamps are
// i128, intermediates may need 256-bit arithmetic. We provide a u256 helper
// for this purpose.

// =============================================================================
// 256-bit unsigned arithmetic (minimal, for intermediate multiply)
// =============================================================================

#[derive(Debug, Clone, Copy)]
struct U256 {
    hi: u128,
    lo: u128,
}

impl U256 {
    const ZERO: Self = Self { hi: 0, lo: 0 };

    /// Multiply two u128 values to get a full 256-bit result.
    fn mul_u128(a: u128, b: u128) -> Self {
        // Split into 64-bit halves to avoid overflow.
        let a_lo = a as u64 as u128;
        let a_hi = (a >> 64) as u64 as u128;
        let b_lo = b as u64 as u128;
        let b_hi = (b >> 64) as u64 as u128;

        let ll = a_lo * b_lo;
        let lh = a_lo * b_hi;
        let hl = a_hi * b_lo;
        let hh = a_hi * b_hi;

        let mid_sum = lh + hl; // can overflow u128
        let mid_carry = if mid_sum < lh { 1u128 } else { 0 };

        let lo = ll.wrapping_add(mid_sum << 64);
        let lo_carry = if lo < ll { 1u128 } else { 0 };

        let hi = hh + (mid_sum >> 64) + (mid_carry << 64) + lo_carry;

        Self { hi, lo }
    }

    /// Right-shift by `n` bits (n < 256).
    fn shr(self, n: u32) -> Self {
        if n == 0 {
            self
        } else if n < 128 {
            Self {
                hi: self.hi >> n,
                lo: (self.lo >> n) | (self.hi << (128 - n)),
            }
        } else if n < 256 {
            Self {
                hi: 0,
                lo: self.hi >> (n - 128),
            }
        } else {
            Self::ZERO
        }
    }

    /// Truncate to u128 (take low 128 bits).
    fn low(self) -> u128 {
        self.lo
    }
}

// =============================================================================
// Fixed-point conversion constants
// =============================================================================
//
// We precompute K and M for each SI unit. We use S = 64 throughout, which
// gives us ample precision and keeps the math uniform.

const SHIFT: u32 = 64;

/// Compute K = round(2^(50 + SHIFT) / 10^exp) at compile time.
/// These are the multipliers for SI-to-Feki conversion.
///
/// For seconds (exp=0):    K = 2^114 (exact, no loss)
/// For millis  (exp=3):    K = 2^114 / 10^3
/// For micros  (exp=6):    K = 2^114 / 10^6
/// For nanos   (exp=9):    K = 2^114 / 10^9
/// For picos   (exp=12):   K = 2^114 / 10^12
///
/// Since 2^114 doesn't fit in u128 for exp=0, seconds are handled specially.

// K for milliseconds: ceil(2^114 / 10^3)
const K_MILLI: u128 = 20_769_187_434_139_310_514_121_985_316_881;

// K for microseconds: ceil(2^114 / 10^6)
const K_MICRO: u128 = 20_769_187_434_139_310_514_121_985_317;

// K for nanoseconds: ceil(2^114 / 10^9)
const K_NANO: u128 = 20_769_187_434_139_310_514_121_986;

// K for picoseconds: ceil(2^114 / 10^12)
const K_PICO: u128 = 20_769_187_434_139_310_514_122;

/// Reverse constants: M = round(10^exp * 2^(SHIFT) / 2^50)
/// = round(10^exp * 2^(SHIFT - 50))
/// = 10^exp * 2^14 (since SHIFT=64, SHIFT-50=14)

const M_MILLI: u128 = 1_000 * (1 << 14);             // 16_384_000
const M_MICRO: u128 = 1_000_000 * (1 << 14);          // 16_384_000_000
const M_NANO: u128 = 1_000_000_000 * (1 << 14);       // 16_384_000_000_000
const M_PICO: u128 = 1_000_000_000_000 * (1 << 14);   // 16_384_000_000_000_000

// =============================================================================
// Internal helpers
// =============================================================================

/// Multiply a signed value by an unsigned constant using 256-bit intermediate,
/// then right-shift. Returns signed result.
fn mul_shr(value: i128, k: u128, shift: u32) -> i128 {
    let negative = value < 0;
    let abs = if negative {
        (value as u128).wrapping_neg()
    } else {
        value as u128
    };

    let wide = U256::mul_u128(abs, k);
    let result = wide.shr(shift).low() as i128;

    if negative { -result } else { result }
}

// =============================================================================
// Conversions: SI → Feki
// =============================================================================

impl Feki {
    /// From whole seconds.
    pub fn from_seconds(s: i128) -> Self {
        // Feki = s * 2^50, which is just a left shift.
        Self(s << 50)
    }

    /// From milliseconds (10^-3 s).
    pub fn from_millis(ms: i128) -> Self {
        Self(mul_shr(ms, K_MILLI, SHIFT))
    }

    /// From microseconds (10^-6 s).
    pub fn from_micros(us: i128) -> Self {
        Self(mul_shr(us, K_MICRO, SHIFT))
    }

    /// From nanoseconds (10^-9 s).
    pub fn from_nanos(ns: i128) -> Self {
        Self(mul_shr(ns, K_NANO, SHIFT))
    }

    /// From picoseconds (10^-12 s).
    pub fn from_picos(ps: i128) -> Self {
        Self(mul_shr(ps, K_PICO, SHIFT))
    }

    // =========================================================================
    // Conversions: Feki → SI
    // =========================================================================

    /// To whole seconds (truncates toward zero).
    pub fn to_seconds(self) -> i128 {
        // Just a right arithmetic shift by 50.
        self.0 >> 50
    }

    /// To milliseconds (truncates toward zero).
    pub fn to_millis(self) -> i128 {
        mul_shr(self.0, M_MILLI, SHIFT)
    }

    /// To microseconds (truncates toward zero).
    pub fn to_micros(self) -> i128 {
        mul_shr(self.0, M_MICRO, SHIFT)
    }

    /// To nanoseconds (truncates toward zero).
    pub fn to_nanos(self) -> i128 {
        mul_shr(self.0, M_NANO, SHIFT)
    }

    /// To picoseconds (truncates toward zero).
    pub fn to_picos(self) -> i128 {
        mul_shr(self.0, M_PICO, SHIFT)
    }

    // =========================================================================
    // Arithmetic
    // =========================================================================

    pub fn wrapping_add(self, other: Self) -> Self {
        Self(self.0.wrapping_add(other.0))
    }

    pub fn wrapping_sub(self, other: Self) -> Self {
        Self(self.0.wrapping_sub(other.0))
    }

    pub fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    pub fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    pub fn abs_diff(self, other: Self) -> u128 {
        self.0.abs_diff(other.0)
    }

    /// Raw inner value.
    pub fn raw(self) -> i128 {
        self.0
    }

    /// From raw i128 value.
    pub fn from_raw(raw: i128) -> Self {
        Self(raw)
    }

    // =========================================================================
    // Epoch utilities
    // =========================================================================

    /// The epoch: 1969-07-20 20:17:40 TAI (Apollo 11 Eagle landing).
    pub const EPOCH: Self = Self(0);

    /// Maximum representable timestamp (~4.8 billion years after epoch).
    pub const MAX: Self = Self(i128::MAX);

    /// Minimum representable timestamp (~4.8 billion years before epoch).
    pub const MIN: Self = Self(i128::MIN);
}

impl core::fmt::Display for Feki {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let seconds = self.to_seconds();
        let frac_feki = self.0 - (seconds << 50);
        write!(f, "{}.{:015} Feki-epoch", seconds, frac_feki.unsigned_abs())
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seconds_roundtrip() {
        for &s in &[0i128, 1, -1, 1000, -1000, 86400, 365 * 86400] {
            let feki = Feki::from_seconds(s);
            assert_eq!(feki.to_seconds(), s, "roundtrip failed for {s} seconds");
        }
    }

    #[test]
    fn millis_roundtrip() {
        // 1000 ms should equal 1 second in Feki
        let from_ms = Feki::from_millis(1000);
        let from_s = Feki::from_seconds(1);
        let diff = (from_ms.0 - from_s.0).unsigned_abs();
        // Allow ±1 Feki of rounding error
        assert!(diff <= 1, "1000ms vs 1s: diff = {diff}");
    }

    #[test]
    fn micros_roundtrip() {
        let from_us = Feki::from_micros(1_000_000);
        let from_s = Feki::from_seconds(1);
        let diff = (from_us.0 - from_s.0).unsigned_abs();
        assert!(diff <= 1_000, "1M us vs 1s: diff = {diff}");
    }

    #[test]
    fn nanos_roundtrip() {
        let from_ns = Feki::from_nanos(1_000_000_000);
        let from_s = Feki::from_seconds(1);
        let diff = (from_ns.0 - from_s.0).unsigned_abs();
        assert!(diff <= 1_000_000, "1G ns vs 1s: diff = {diff}");
    }

    #[test]
    fn picos_roundtrip() {
        let from_ps = Feki::from_picos(1_000_000_000_000);
        let from_s = Feki::from_seconds(1);
        let diff = (from_ps.0 - from_s.0).unsigned_abs();
        assert!(diff <= 1_000_000_000, "1T ps vs 1s: diff = {diff}");
    }

    #[test]
    fn negative_time() {
        let feki = Feki::from_seconds(-100);
        assert_eq!(feki.to_seconds(), -100);

        let feki = Feki::from_millis(-5000);
        assert_eq!(feki.to_seconds(), -5);
    }

    #[test]
    fn ordering() {
        let a = Feki::from_nanos(1000);
        let b = Feki::from_nanos(1001);
        assert!(a < b);
        assert!(b > a);
        assert_eq!(a, a);
    }

    #[test]
    fn to_millis_accuracy() {
        // 1 second in Feki should convert to 1000 millis
        let feki = Feki::from_seconds(1);
        assert_eq!(feki.to_millis(), 1000);
    }

    #[test]
    fn to_micros_accuracy() {
        let feki = Feki::from_seconds(1);
        assert_eq!(feki.to_micros(), 1_000_000);
    }

    #[test]
    fn to_nanos_accuracy() {
        let feki = Feki::from_seconds(1);
        assert_eq!(feki.to_nanos(), 1_000_000_000);
    }

    #[test]
    fn to_picos_accuracy() {
        let feki = Feki::from_seconds(1);
        assert_eq!(feki.to_picos(), 1_000_000_000_000);
    }

    #[test]
    fn small_values() {
        // 1 nanosecond should be about 1125899 Feki (2^50 / 10^9 ≈ 1125899.9)
        let one_ns = Feki::from_nanos(1);
        let expected = 1125899i128; // floor(2^50 / 10^9)
        let diff = (one_ns.0 - expected).unsigned_abs();
        assert!(diff <= 2, "1ns = {} Feki, expected ~{expected}", one_ns.0);

        // 1 picosecond should be about 1125 Feki (2^50 / 10^12 ≈ 1125.9)
        let one_ps = Feki::from_picos(1);
        let expected_ps = 1125i128;
        let diff_ps = (one_ps.0 - expected_ps).unsigned_abs();
        assert!(diff_ps <= 2, "1ps = {} Feki, expected ~{expected_ps}", one_ps.0);
    }

    #[test]
    fn arithmetic() {
        let a = Feki::from_seconds(10);
        let b = Feki::from_seconds(3);
        assert_eq!(a.wrapping_sub(b).to_seconds(), 7);
        assert_eq!(a.wrapping_add(b).to_seconds(), 13);
    }

    #[test]
    fn range_check() {
        // 2^77 seconds ≈ 4.8 billion years
        let max_seconds = Feki::MAX.to_seconds();
        let expected = (1i128 << 77) - 1;
        assert_eq!(max_seconds, expected);
    }

    #[test]
    fn u256_basic() {
        // Test that our u256 multiply works for known values
        let result = U256::mul_u128(1u128 << 64, 1u128 << 64);
        assert_eq!(result.hi, 1);
        assert_eq!(result.lo, 0);
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct ApolloTime(Feki);

impl IntoNoun for ApolloTime {
    fn into_noun(&self) -> Result<Noun, NounConversionError> {
        Ok(Noun::from_i128(self.0.raw()))
    }
}

impl FromNoun for ApolloTime {
    fn from_noun(noun: Noun) -> Result<Self, NounConversionError> {
        let bytes = noun.as_atom().ok_or(NounConversionError::InvalidNoun)?;
        let words = bytes.as_words().ok_or(NounConversionError::InvalidNoun)?;
        let bytes = bytemuck::cast_slice::<u32, u8>(&words);
        let time = i128::from_le_bytes(bytes.try_into().unwrap());
        Ok(Self(Feki::from_raw(time)))
    }
}


const _FEMTO_PER_US: i128 = 1_000_000_000;
const _FEMTO_PER_MS: i128 = 1_000_000_000_000;
const _FEMTO_PER_S: i128 = 1_000_000_000_000_000;

const UNIX_EPOCH_AS_APOLLO: i128 = 0x300_dbd87f0edb7c8000;

fn rebase_epoch_to_apollo(time: i128) -> Feki {
  Feki::from_raw(time + UNIX_EPOCH_AS_APOLLO)
}

// apollo = time - unix
fn _rebase_epoch_to_unix(time: i128) -> Feki {
  Feki::from_raw(time - UNIX_EPOCH_AS_APOLLO)
}

#[allow(dead_code)]
enum TimeUnit {
  Miki,
  Muki,
  Naki,
  Paki,
  Feki,
  Milli,
  Micro,
  Nano,
  Pico,
  Femto,
  Second
}

impl TimeUnit {
  fn to_feki(&self, time: i128) -> Feki {
    match self {
      TimeUnit::Femto => panic!("Femto is not a valid time unit"),
      TimeUnit::Pico => Feki::from_picos(time),
      TimeUnit::Milli => Feki::from_millis(time),
      TimeUnit::Micro => Feki::from_micros(time),
      TimeUnit::Nano => Feki::from_nanos(time),
      TimeUnit::Second => Feki::from_seconds(time),
      TimeUnit::Miki => Feki::from_raw(time << 40),
      TimeUnit::Muki => Feki::from_raw(time >> 30),
      TimeUnit::Naki => Feki::from_raw(time >> 20),
      TimeUnit::Paki => Feki::from_raw(time >> 10),
      TimeUnit::Feki => Feki::from_raw(time),
    }
  }

  fn _from_feki(&self, femto: Feki) -> i128 {
    match self {
      TimeUnit::Femto => panic!("Femto is not a valid time unit"),
      TimeUnit::Micro => femto.to_micros(),
      TimeUnit::Milli => femto.to_millis(),
      TimeUnit::Second => femto.to_seconds(),
      TimeUnit::Miki => femto.raw() >> 40,
      TimeUnit::Muki => femto.raw() >> 30,
      TimeUnit::Naki => femto.raw() >> 20,
      TimeUnit::Paki => femto.raw() >> 10,
      TimeUnit::Feki => femto.raw(),
      TimeUnit::Nano => femto.to_nanos(),
      TimeUnit::Pico => femto.to_picos(),
    }
  }
}

fn to_apollo_time<T: Into<i128>>(time: T, unit: TimeUnit) -> Feki {
  let rebased = rebase_epoch_to_apollo(time.into());
  unit.to_feki(rebased.raw())
}

#[allow(dead_code)]
fn from_apollo_time<T: Into<i128>>(time: T, unit: TimeUnit) -> i128 {
  let femto = _rebase_epoch_to_unix(time.into());
  unit._from_feki(femto)
}


#[allow(dead_code)]
impl ApolloTime {
  fn from_unix_us<T: Into<i128>>(micros: T) -> Self {
    Self(to_apollo_time(micros, TimeUnit::Micro))
  }

  fn from_unix_ms<T: Into<i128>>(millis: T) -> Self {
    Self(to_apollo_time(millis, TimeUnit::Milli))
  }

  fn from_unix_s<T: Into<i128>>(seconds: T) -> Self {
    Self(to_apollo_time(seconds, TimeUnit::Second))
  }

  fn to_unix_us(&self) -> i128 {
    from_apollo_time(self.0.0, TimeUnit::Micro)
  }

  fn to_unix_ms(&self) -> i64 {
    from_apollo_time(self.0.0, TimeUnit::Milli) as i64
  }

  fn to_unix_s(&self) -> i64 {
    from_apollo_time(self.0.0, TimeUnit::Second) as i64
  }

  pub fn now() -> Self {
    let micros = chrono::Utc::now().timestamp_micros();
    Self(to_apollo_time(micros, TimeUnit::Micro))
  }

}

