// SPDX-License-Identifier: AGPL-3.0-only
// Translated from Soar `Core/SoarKernel/src/shared/soar_rand.h` and
// `soar_rand.cpp` (https://github.com/SoarGroup/Soar, commit 3fc337a), which
// is Richard J. Wagner's MersenneTwister.h v1.0. Modified: see
// docs/DEVIATIONS.md. Original notice:
//
// Copyright (C) 1997 - 2002, Makoto Matsumoto and Takuji Nishimura,
// Copyright (C) 2000 - 2003, Richard J. Wagner
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions
// are met:
//
//   1. Redistributions of source code must retain the above copyright
//      notice, this list of conditions and the following disclaimer.
//
//   2. Redistributions in binary form must reproduce the above copyright
//      notice, this list of conditions and the following disclaimer in the
//      documentation and/or other materials provided with the distribution.
//
//   3. The names of its contributors may not be used to endorse or promote
//      products derived from this software without specific prior written
//      permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
// "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
// LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
// A PARTICULAR PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL THE COPYRIGHT OWNER OR
// CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
// EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
// PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
// PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
// LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
// NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
// SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

//! Soar's random number generator: the MT19937 Mersenne Twister with
//! Wagner's output transforms (`soar_rand.h`).
//!
//! The stream is identical to Soar's for the same seed, which is what lets
//! the differential test compare random operator selections against the
//! real `soar` binary.
//!
//! Searched before porting: `petir::rng` (OpenMC LCG, ChaCha8) and
//! `raffles` in outram-park-backend, and this workspace. Neither holds a
//! Mersenne Twister, and matching Soar's choices bit for bit needs Soar's
//! own generator.
//!
//! Unlike Soar, which keeps one process-wide generator, each [`SoarRng`] is
//! an owned value (one per agent or per simulation), and there is no
//! time-based or `/dev/urandom` seeding: `no_std` code has no entropy
//! source, so the caller always supplies the seed.

const N: usize = 624;
const M: usize = 397;

/// Soar's Mersenne Twister (Go-style naming kept from upstream: `MTRand`).
#[derive(Clone)]
pub struct SoarRng {
    state: [u32; N],
    next: usize,
    left: usize,
}

impl core::fmt::Debug for SoarRng {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SoarRng")
            .field("left", &self.left)
            .finish_non_exhaustive()
    }
}

impl SoarRng {
    /// `MTRand(oneSeed)` / `SoarSeedRNG(seed)`, as set by Soar's
    /// `srand <seed>` command.
    pub fn new(seed: u32) -> Self {
        let mut r = Self {
            state: [0; N],
            next: 0,
            left: 0,
        };
        r.seed(seed);
        r
    }

    /// Re-seed (`MTRand::seed(oneSeed)`).
    pub fn seed(&mut self, seed: u32) {
        self.initialize(seed);
        self.reload();
    }

    fn initialize(&mut self, seed: u32) {
        self.state[0] = seed;
        for i in 1..N {
            let r = self.state[i - 1];
            self.state[i] = 1812433253u32
                .wrapping_mul(r ^ (r >> 30))
                .wrapping_add(i as u32);
        }
    }

    fn twist(m: u32, s0: u32, s1: u32) -> u32 {
        let mix = (s0 & 0x8000_0000) | (s1 & 0x7fff_ffff);
        m ^ (mix >> 1) ^ (0u32.wrapping_sub(s1 & 1) & 0x9908_b0df)
    }

    fn reload(&mut self) {
        let s = &mut self.state;
        for p in 0..N - M {
            s[p] = Self::twist(s[p + M], s[p], s[p + 1]);
        }
        for p in N - M..N - 1 {
            s[p] = Self::twist(s[p + M - N], s[p], s[p + 1]);
        }
        s[N - 1] = Self::twist(s[M - 1], s[N - 1], s[0]);
        self.left = N;
        self.next = 0;
    }

    /// Integer in `[0, 2^32 - 1]` (`randInt()` / `SoarRandInt()`).
    pub fn rand_int(&mut self) -> u32 {
        if self.left == 0 {
            self.reload();
        }
        self.left -= 1;
        let mut s1 = self.state[self.next];
        self.next += 1;
        s1 ^= s1 >> 11;
        s1 ^= (s1 << 7) & 0x9d2c_5680;
        s1 ^= (s1 << 15) & 0xefc6_0000;
        s1 ^ (s1 >> 18)
    }

    /// Integer in `[0, n]` by masked rejection (`randInt(n)` /
    /// `SoarRandInt(n)`).
    pub fn rand_int_max(&mut self, n: u32) -> u32 {
        let mut used = n;
        used |= used >> 1;
        used |= used >> 2;
        used |= used >> 4;
        used |= used >> 8;
        used |= used >> 16;
        loop {
            let i = self.rand_int() & used;
            if i <= n {
                return i;
            }
        }
    }

    /// Real number in `[0, 1]`, both ends included (`rand()` /
    /// `SoarRand()`).
    pub fn rand(&mut self) -> f64 {
        f64::from(self.rand_int()) * (1.0 / 4294967295.0)
    }

    /// Real number in `[0, n]` (`rand(n)` / `SoarRand(max)`).
    pub fn rand_max(&mut self, n: f64) -> f64 {
        self.rand() * n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MT19937 reference outputs: the first draws for seed 5489 (the
    /// reference implementation's default) and the 10000th draw, which the
    /// C++ standard specifies as 4123659995 for `std::mt19937`.
    #[test]
    fn matches_mt19937_reference() {
        let mut r = SoarRng::new(5489);
        assert_eq!(r.rand_int(), 3499211612);
        assert_eq!(r.rand_int(), 581869302);
        let mut r = SoarRng::new(5489);
        let mut last = 0;
        for _ in 0..10000 {
            last = r.rand_int();
        }
        assert_eq!(last, 4123659995);
    }

    /// Pins Wagner's `rand()`: multiply by the reciprocal of 2^32 - 1, so both
    /// ends are reachable.
    /// The differential test cannot see this constant: changing it to 2^32
    /// moves a draw by about 2^-32 and never changed a decision in 3000
    /// scenarios.
    #[test]
    fn rand_uses_closed_interval_divisor() {
        let mut a = SoarRng::new(77);
        let mut b = SoarRng::new(77);
        for _ in 0..100 {
            let i = a.rand_int();
            assert_eq!(
                b.rand().to_bits(),
                (f64::from(i) * (1.0 / 4294967295.0)).to_bits()
            );
        }
    }

    #[test]
    fn rand_int_max_stays_in_range() {
        let mut r = SoarRng::new(1);
        for n in [0u32, 1, 2, 5, 7, 100] {
            for _ in 0..200 {
                assert!(r.rand_int_max(n) <= n);
            }
        }
    }

    #[test]
    fn rand_is_closed_unit_interval() {
        let mut r = SoarRng::new(3);
        for _ in 0..1000 {
            let x = r.rand();
            assert!((0.0..=1.0).contains(&x));
        }
    }

    #[test]
    fn reseeding_restarts_the_stream() {
        let mut a = SoarRng::new(42);
        let first: [u32; 3] = core::array::from_fn(|_| a.rand_int());
        a.seed(42);
        let again: [u32; 3] = core::array::from_fn(|_| a.rand_int());
        assert_eq!(first, again);
    }
}
