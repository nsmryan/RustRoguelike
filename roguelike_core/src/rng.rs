//! NOTE this code comes from the oorandom crate, modified
//! to support serde.

// The oorandom crate copyright is copied below:
// The MIT License (MIT)
// 
// Copyright (c) 2019 Simon Heath
// 
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
// 
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
// 
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! A tiny, robust PRNG implementation.
//!
//! More specifically, it implements a single GOOD PRNG algorithm,
//! which is currently a permuted congruential generator.  It has two
//! implementations, one that returns `u32` and one that returns
//! `u64`.  It also has functions that return floats or integer
//! ranges.  And that's it.  What more do you need?
//!
//! For more info on PCG generators, see http://www.pcg-random.org/
//!
//! This was designed as a minimalist utility for video games.  No
//! promises are made about its quality, and if you use it for
//! cryptography you will get what you deserve.
//!
//! Works with `#![no_std]`, has no global state, no dependencies
//! apart from some in the unit tests, and is generally neato.

#![forbid(unsafe_code)]
#![forbid(missing_docs)]
#![forbid(missing_debug_implementations)]
#![forbid(unused_results)]
#![no_std]
use core::ops::Range;

use serde::{Serialize, Deserialize};

/// A PRNG producing a 32-bit output.
///
/// The current implementation is `PCG-XSH-RR`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rand32 {
    state: u64,
    inc: u64,
}

impl Rand32 {
    /// The default value for `increment`.
    /// This is basically arbitrary, it comes from the
    /// PCG reference C implementation:
    /// https://github.com/imneme/pcg-c/blob/master/include/pcg_variants.h#L284
    pub const DEFAULT_INC: u64 = 1442695040888963407;

    /// This is the number that you have to Really Get Right.
    ///
    /// The value used here is from the PCG C implementation:
    /// https://github.com/imneme/pcg-c/blob/master/include/pcg_variants.h#L278
    pub(crate) const MULTIPLIER: u64 = 6364136223846793005;

    /// Creates a new PRNG with the given seed and a default increment.
    pub fn new(seed: u64) -> Self {
        Self::new_inc(seed, Self::DEFAULT_INC)
    }

    /// Creates a new PRNG.  The two inputs, `seed` and `increment`,
    /// determine what you get; `increment` basically selects which
    /// sequence of all those possible the PRNG will produce, and the
    /// `seed` selects where in that sequence you start.
    ///
    /// Both are arbitrary; increment must be an odd number but this
    /// handles that for you
    pub fn new_inc(seed: u64, increment: u64) -> Self {
        let mut rng = Self {
            state: 0,
            inc: increment.wrapping_shl(1) | 1,
        };
        // This initialization song-and-dance is a little odd,
        // but seems to be just how things go.
        let _ = rng.rand_u32();
        rng.state = rng.state.wrapping_add(seed);
        let _ = rng.rand_u32();
        rng
    }

    /// Returns the internal state of the PRNG.  This allows
    /// you to save a PRNG and create a new one that will resume
    /// from the same spot in the sequence.
    pub fn state(&self) -> (u64, u64) {
        (self.state, self.inc)
    }

    /// Creates a new PRNG from a saved state from `Rand32::state()`.
    /// This is NOT quite the same as `new_inc()` because `new_inc()` does
    /// a little extra setup work to initialize the state.
    pub fn from_state(state: (u64, u64)) -> Self {
        let (state, inc) = state;
        Self { state, inc }
    }

    /// Produces a random `u32` in the range `[0, u32::MAX]`.
    pub fn rand_u32(&mut self) -> u32 {
        let oldstate: u64 = self.state;
        self.state = oldstate
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(self.inc);
        let xorshifted: u32 = (((oldstate >> 18) ^ oldstate) >> 27) as u32;
        let rot: u32 = (oldstate >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// Produces a random `i32` in the range `[i32::MIN, i32::MAX]`.
    pub fn rand_i32(&mut self) -> i32 {
        self.rand_u32() as i32
    }

    /// Produces a random `f32` in the range `[0.0, 1.0)`.
    pub fn rand_float(&mut self) -> f32 {
        // This impl was taken more or less from `rand`, see
        // <https://docs.rs/rand/0.7.0/src/rand/distributions/float.rs.html#104-117>
        // There MAY be better ways to do this, see:
        // https://mumble.net/~campbell/2014/04/28/uniform-random-float
        // https://mumble.net/~campbell/2014/04/28/random_real.c
        // https://github.com/Lokathor/randomize/issues/34
        const TOTAL_BITS: u32 = 32;
        const PRECISION: u32 = core::f32::MANTISSA_DIGITS + 1;
        const MANTISSA_SCALE: f32 = 1.0 / ((1u32 << PRECISION) as f32);
        let mut u = self.rand_u32();
        u >>= TOTAL_BITS - PRECISION;
        u as f32 * MANTISSA_SCALE
    }

    /// Produces a random within the given bounds.  Like any `Range`,
    /// it includes the lower bound and excludes the upper one.
    ///
    /// This should be faster than `Self::rand() % end + start`, but the
    /// real advantage is it's more convenient.  Requires that
    /// `range.end <= range.start`.
    pub fn rand_range(&mut self, range: Range<u32>) -> u32 {
        // This is harder to do well than it looks, it seems.  I don't
        // trust Lokathor's implementation 'cause I don't understand
        // it, so I went to numpy's, which points me to "Lemire's
        // rejection algorithm": http://arxiv.org/abs/1805.10941
        //
        // Algorithms 3, 4 and 5 in that paper all seem fine modulo
        // minor performance differences, so this is algorithm 5.
        // It uses numpy's implementation, `buffered_bounded_lemire_uint32()`

        debug_assert!(range.start < range.end);
        let range_starting_from_zero = 0..(range.end - range.start);

        let s: u32 = range_starting_from_zero.end;
        let mut m: u64 = u64::from(self.rand_u32()) * u64::from(s);
        let mut leftover: u32 = (m & 0xFFFF_FFFF) as u32;

        if leftover < s {
            // TODO: verify the wrapping_neg() here
            let threshold: u32 = s.wrapping_neg() % s;
            while leftover < threshold {
                m = u64::from(self.rand_u32()).wrapping_mul(u64::from(s));
                leftover = (m & 0xFFFF_FFFF) as u32;
            }
        }
        (m >> 32) as u32 + range.start
    }
}

/// A PRNG producing a 64-bit output.
///
/// The current implementation is `PCG-XSH-RR`.
// BUGGO: The recommended algorithm is PCG-XSL-RR?
// See https://github.com/imneme/pcg-c/blob/master/include/pcg_variants.h#L2405
// Not sure if it matters?
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Rand64 {
    state: u128,
    inc: u128,
}

impl Rand64 {
    /// The default value for `increment`.
    ///
    /// The value used here is from the PCG default C implementation: http://www.pcg-random.org/download.html
    pub const DEFAULT_INC: u128 = 0x2FE0E169_FFBD06E3_5BC307BD_4D2F814F;

    /// This is the number that you have to Really Get Right.
    ///
    /// The value used here is from the PCG C implementation:
    /// https://github.com/imneme/pcg-c/blob/master/include/pcg_variants.h#L288
    pub(crate) const MULTIPLIER: u128 = 47026247687942121848144207491837523525;

    /// Creates a new PRNG with the given seed and a default increment.
    pub fn new(seed: u128) -> Self {
        Self::new_inc(seed, Self::DEFAULT_INC)
    }

    /// Same as `Rand32::new_inc()`
    pub fn new_inc(seed: u128, increment: u128) -> Self {
        let mut rng = Self {
            state: 0,
            inc: increment.wrapping_shl(1) | 1,
        };
        let _ = rng.rand_u64();
        rng.state = rng.state.wrapping_add(seed);
        let _ = rng.rand_u64();
        rng
    }

    /// Returns the internal state of the PRNG.  This allows
    /// you to save a PRNG and create a new one that will resume
    /// from the same spot in the sequence.
    pub fn state(&self) -> (u128, u128) {
        (self.state, self.inc)
    }

    /// Creates a new PRNG from a saved state from `Rand32::state()`.
    /// This is NOT quite the same as `new_inc()` because `new_inc()` does
    /// a little extra setup work to initialize the state.
    pub fn from_state(state: (u128, u128)) -> Self {
        let (state, inc) = state;
        Self { state, inc }
    }

    /// Produces a random `u64` in the range`[0, u64::MAX]`.
    pub fn rand_u64(&mut self) -> u64 {
        let oldstate: u128 = self.state;
        self.state = oldstate
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(self.inc);
        let xorshifted: u64 = (((oldstate >> 29) ^ oldstate) >> 58) as u64;
        let rot: u32 = (oldstate >> 122) as u32;
        xorshifted.rotate_right(rot)
    }

    /// Produces a random `i64` in the range `[i64::MIN, i64::MAX]`.
    pub fn rand_i64(&mut self) -> i64 {
        self.rand_u64() as i64
    }

    /// Produces a random `f64` in the range `[0.0, 1.0)`.
    pub fn rand_float(&mut self) -> f64 {
        const TOTAL_BITS: u32 = 64;
        const PRECISION: u32 = core::f64::MANTISSA_DIGITS + 1;
        const MANTISSA_SCALE: f64 = 1.0 / ((1u64 << PRECISION) as f64);
        let mut u = self.rand_u64();
        u >>= TOTAL_BITS - PRECISION;
        u as f64 * MANTISSA_SCALE
    }

    /// Produces a random within the given bounds.  Like any `Range`,
    /// it includes the lower bound and excludes the upper one.
    ///
    /// This should be faster than `Self::rand() % end + start`, but the
    /// real advantage is it's more convenient.  Requires that
    /// `range.end <= range.start`.
    pub fn rand_range(&mut self, range: Range<u64>) -> u64 {
        // Same as `Rand32::rand_range()`
        debug_assert!(range.start < range.end);
        let range_starting_from_zero = 0..(range.end - range.start);

        let s: u64 = range_starting_from_zero.end;
        let mut m: u128 = u128::from(self.rand_u64()) * u128::from(s);
        let mut leftover: u64 = (m & 0xFFFFFFFF_FFFFFFFF) as u64;

        if leftover < s {
            // TODO: Verify the wrapping_negate() here
            let threshold: u64 = s.wrapping_neg() % s;
            while leftover < threshold {
                m = u128::from(self.rand_u64()) * u128::from(s);
                leftover = (m & 0xFFFFFFFF_FFFFFFFF) as u64;
            }
        }
        (m.wrapping_shr(64)) as u64 + range.start
    }
}

