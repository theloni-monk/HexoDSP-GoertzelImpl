// Copyright (c) 2021 Weird Constructor <weirdconstructor@gmail.com>
// This is a part of HexoDSP. Released under (A)GPLv3 or any later.
// See README.md and COPYING for details.

static FAST_COS_TAB_LOG2_SIZE : usize = 9;
static FAST_COS_TAB_SIZE : usize      = 1 << FAST_COS_TAB_LOG2_SIZE; // =512
static mut FAST_COS_TAB : [f32; 513] = [0.0; 513];

pub fn init_cos_tab() {
    for i in 0..(FAST_COS_TAB_SIZE+1) {
        let phase : f32 =
            (i as f32)
            * ((std::f32::consts::PI * 2.0)
               / (FAST_COS_TAB_SIZE as f32));
        unsafe {
            // XXX: note: mutable statics can be mutated by multiple
            //      threads: aliasing violations or data races
            //      will cause undefined behavior
            FAST_COS_TAB[i] = phase.cos();
        }
    }
}

const PHASE_SCALE : f32 = 1.0_f32 / (std::f32::consts::PI * 2.0_f32);

pub fn fast_cos(mut x: f32) -> f32 {
    x = x.abs(); // cosine is symmetrical around 0, let's get rid of negative values

    // normalize range from 0..2PI to 1..2
    let phase = x * PHASE_SCALE;

    let index = FAST_COS_TAB_SIZE as f32 * phase;

    let fract = index.fract();
    let index = index.floor() as usize;

    unsafe {
        // XXX: note: mutable statics can be mutated by multiple
        //      threads: aliasing violations or data races
        //      will cause undefined behavior
        let left         = FAST_COS_TAB[index as usize];
        let right        = FAST_COS_TAB[index as usize + 1];

        return left + (right - left) * fract;
    }
}

pub fn fast_sin(x: f32) -> f32 {
    fast_cos(x - (std::f32::consts::PI / 2.0))
}

static mut WHITE_NOISE_TAB: [f64; 1024] = [0.0; 1024];

pub fn init_white_noise_tab() {
    let mut rng = RandGen::new();
    unsafe {
        for i in 0..WHITE_NOISE_TAB.len() {
            WHITE_NOISE_TAB[i as usize] = rng.next_open01();
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct RandGen {
    r: [u64; 2],
}

// Taken from xoroshiro128 crate under MIT License
// Implemented by Matthew Scharley (Copyright 2016)
// https://github.com/mscharley/rust-xoroshiro128
pub fn next_xoroshiro128(state: &mut [u64; 2]) -> u64 {
    let s0: u64     = state[0];
    let mut s1: u64 = state[1];
    let result: u64 = s0.wrapping_add(s1);

    s1 ^= s0;
    state[0] = s0.rotate_left(55) ^ s1 ^ (s1 << 14); // a, b
    state[1] = s1.rotate_left(36); // c

    result
}

// Taken from rand::distributions
// Licensed under the Apache License, Version 2.0
// Copyright 2018 Developers of the Rand project.
pub fn u64_to_open01(u: u64) -> f64 {
    use core::f64::EPSILON;
    let float_size         = std::mem::size_of::<f64>() as u32 * 8;
    let fraction           = u >> (float_size - 52);
    let exponent_bits: u64 = (1023 as u64) << 52;
    f64::from_bits(fraction | exponent_bits) - (1.0 - EPSILON / 2.0)
}

impl RandGen {
    pub fn new() -> Self {
        RandGen {
            r: [0x193a6754a8a7d469, 0x97830e05113ba7bb],
        }
    }

    pub fn next(&mut self) -> u64 {
        next_xoroshiro128(&mut self.r)
    }

    pub fn next_open01(&mut self) -> f64 {
        u64_to_open01(self.next())
    }
}


//- splitmix64 (http://xoroshiro.di.unimi.it/splitmix64.c) 
//"""
//  Written in 2015 by Sebastiano Vigna (vigna@acm.org)
//
//  To the extent possible under law, the author has dedicated all copyright
//  and related and neighboring rights to this software to the public domain
//  worldwide. This software is distributed without any warranty.
//
//  See <http://creativecommons.org/publicdomain/zero/1.0/>. 
//"""
//
// Written by Alexander Stocko <as@coder.gg>
//
// To the extent possible under law, the author has dedicated all copyright
// and related and neighboring rights to this software to the public domain
// worldwide. This software is distributed without any warranty.
//
// See <LICENSE or http://creativecommons.org/publicdomain/zero/1.0/>

/// The `SplitMix64` random number generator.
#[derive(Copy, Clone)]
pub struct SplitMix64(pub u64);

impl SplitMix64 {
    pub fn new(seed: u64) -> Self { Self(seed) }
    pub fn new_from_i64(seed: i64) -> Self {
        Self::new(u64::from_be_bytes(seed.to_be_bytes()))
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        use std::num::Wrapping as w;

        let mut z = w(self.0) + w(0x9E37_79B9_7F4A_7C15_u64);
        self.0 = z.0;
        z = (z ^ (z >> 30)) * w(0xBF58_476D_1CE4_E5B9_u64);
        z = (z ^ (z >> 27)) * w(0x94D0_49BB_1331_11EB_u64);
        (z ^ (z >> 31)).0
    }

    #[inline]
    pub fn next_i64(&mut self) -> i64 {
        i64::from_be_bytes(
            self.next_u64().to_be_bytes())
    }

    #[inline]
    pub fn next_open01(&mut self) -> f64 {
        u64_to_open01(self.next_u64())
    }
}

#[inline]
pub fn crossfade(v1: f32, v2: f32, mix: f32) -> f32 {
    v1 * (1.0 - mix) + v2 * mix
}

#[inline]
pub fn clamp(f: f32, min: f32, max: f32) -> f32 {
         if f < min { min }
    else if f > max { max }
    else            {   f }
}

pub fn square_135(phase: f32) -> f32 {
      fast_sin(phase)
    + fast_sin(phase * 3.0) / 3.0
    + fast_sin(phase * 5.0) / 5.0
}

pub fn square_35(phase: f32) -> f32 {
      fast_sin(phase * 3.0) / 3.0
    + fast_sin(phase * 5.0) / 5.0
}

// note: MIDI note value?
pub fn note_to_freq(note: f32) -> f32 {
    440.0 * (2.0_f32).powf((note - 69.0) / 12.0)
}

// Ported from LMMS under GPLv2
// * DspEffectLibrary.h - library with template-based inline-effects
// * Copyright (c) 2006-2014 Tobias Doerffel <tobydox/at/users.sourceforge.net>
//
/// Signal distortion
/// ```text
/// gain:        0.1 - 5.0       default = 1.0
/// threshold:   0.0 - 100.0     default = 0.8
/// i:           signal
/// ```
pub fn f_distort(gain: f32, threshold: f32, i: f32) -> f32 {
    gain * (
        i * ( i.abs() + threshold )
        / ( i * i + (threshold - 1.0) * i.abs() + 1.0 ))
}

// Ported from LMMS under GPLv2
// * DspEffectLibrary.h - library with template-based inline-effects
// * Copyright (c) 2006-2014 Tobias Doerffel <tobydox/at/users.sourceforge.net>
//
/// Foldback Signal distortion
/// ```text
/// gain:        0.1 - 5.0       default = 1.0
/// threshold:   0.0 - 100.0     default = 0.8
/// i:           signal
/// ```
pub fn f_fold_distort(gain: f32, threshold: f32, i: f32) -> f32 {
    if i >= threshold || i < -threshold {
        gain
        * ((
            ((i - threshold) % threshold * 4.0).abs()
            - threshold * 2.0).abs()
           - threshold)
    } else {
        gain * i
    }
}

pub fn lerp(x: f32, a: f32, b: f32) -> f32 {
    (a * (1.0 - x)) + (b * x)
}

pub fn lerp64(x: f64, a: f64, b: f64) -> f64 {
    (a * (1.0 - x)) + (b * x)
}

pub fn p2range(x: f32, a: f32, b: f32) -> f32 {
    lerp(x, a, b)
}

pub fn p2range_exp(x: f32, a: f32, b: f32) -> f32 {
    let x = x * x;
    (a * (1.0 - x)) + (b * x)
}

pub fn p2range_exp4(x: f32, a: f32, b: f32) -> f32 {
    let x = x * x * x * x;
    (a * (1.0 - x)) + (b * x)
}


pub fn range2p(v: f32, a: f32, b: f32) -> f32 {
    ((v - a) / (b - a)).abs()
}

pub fn range2p_exp(v: f32, a: f32, b: f32) -> f32 {
    (((v - a) / (b - a)).abs()).sqrt()
}

pub fn range2p_exp4(v: f32, a: f32, b: f32) -> f32 {
    (((v - a) / (b - a)).abs()).sqrt().sqrt()
}

/// ```text
/// gain: 24.0 - -90.0   default = 0.0
/// ```
pub fn gain2coef(gain: f32) -> f32 {
    if gain > -90.0 {
        10.0_f32.powf(gain * 0.05)
    } else {
        0.0
    }
}

// quickerTanh / quickerTanh64 credits to mopo synthesis library:
// Under GPLv3 or any later.
// Little IO <littleioaudio@gmail.com>
// Matt Tytel <matthewtytel@gmail.com>
pub fn quicker_tanh64(v: f64) -> f64 {
    let square = v * v;
    v / (1.0 + square / (3.0 + square / 5.0))
}

pub fn quicker_tanh(v: f32) -> f32 {
    let square = v * v;
    v / (1.0 + square / (3.0 + square / 5.0))
}

// quickTanh / quickTanh64 credits to mopo synthesis library:
// Under GPLv3 or any later.
// Little IO <littleioaudio@gmail.com>
// Matt Tytel <matthewtytel@gmail.com>
pub fn quick_tanh64(v: f64) -> f64 {
    let abs_v = v.abs();
    let square = v * v;
    let num =
        v * (2.45550750702956
             + 2.45550750702956 * abs_v
             + square * (0.893229853513558
                         + 0.821226666969744 * abs_v));
    let den =
        2.44506634652299
        + (2.44506634652299 + square)
          * (v + 0.814642734961073 * v * abs_v).abs();

    num / den
}

pub fn quick_tanh(v: f32) -> f32 {
    let abs_v = v.abs();
    let square = v * v;
    let num =
        v * (2.45550750702956
             + 2.45550750702956 * abs_v
             + square * (0.893229853513558
                         + 0.821226666969744 * abs_v));
    let den =
        2.44506634652299
        + (2.44506634652299 + square)
          * (v + 0.814642734961073 * v * abs_v).abs();

    num / den
}

/// A helper function for exponential envelopes:
#[inline]
pub fn sqrt4_to_pow4(x: f32, v: f32) -> f32 {
    if v > 0.75 {
        let xsq1 = x.sqrt();
        let xsq = xsq1.sqrt();
        let v = (v - 0.75) * 4.0;
        xsq1 * (1.0 - v) + xsq * v

    } else if v > 0.5 {
        let xsq = x.sqrt();
        let v = (v - 0.5) * 4.0;
        x * (1.0 - v) + xsq * v

    } else if v > 0.25 {
        let xx = x * x;
        let v = (v - 0.25) * 4.0;
        x * v + xx * (1.0 - v)

    } else {
        let xx = x * x;
        let xxxx = xx * xx;
        let v = v * 4.0;
        xx * v + xxxx * (1.0 - v)
    }
}

/// A-100 Eurorack states, that a trigger is usually 2-10 milliseconds.
const TRIG_SIGNAL_LENGTH_MS : f32 = 2.0;

#[derive(Debug, Clone, Copy)]
pub struct TrigSignal {
    length:     u32,
    scount:     u32,
}

impl TrigSignal {
    pub fn new() -> Self {
        Self {
            length: ((44100.0 * TRIG_SIGNAL_LENGTH_MS) / 1000.0).ceil() as u32,
            scount: 0,
        }
    }

    pub fn reset(&mut self) {
        self.scount = 0;
    }

    pub fn set_sample_rate(&mut self, srate: f32) {
        self.length = ((srate * TRIG_SIGNAL_LENGTH_MS) / 1000.0).ceil() as u32;
        self.scount = 0;
    }

    #[inline]
    pub fn trigger(&mut self) { self.scount = self.length; }

    #[inline]
    pub fn next(&mut self) -> f32 {
        if self.scount > 0 {
            self.scount -= 1;
            1.0
        } else {
            0.0
        }
    }
}

impl Default for TrigSignal {
    fn default() -> Self { Self::new() }
}

#[derive(Debug, Clone, Copy)]
pub struct Trigger {
    triggered:  bool,
}

impl Trigger {
    pub fn new() -> Self {
        Self {
            triggered: false,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.triggered = false;
    }

    #[inline]
    pub fn check_trigger(&mut self, input: f32) -> bool {
        if self.triggered {
            if input <= 0.25 {
                self.triggered = false;
            }

            false

        } else if input > 0.75 {
            self.triggered = true;
            true

        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TriggerPhaseClock {
    clock_phase:    f64,
    clock_inc:      f64,
    prev_trigger:   bool,
    clock_samples:  u32,
}

impl TriggerPhaseClock {
    pub fn new() -> Self {
        Self {
            clock_phase:    0.0,
            clock_inc:      0.0,
            prev_trigger:   true,
            clock_samples:  0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.clock_samples = 0;
        self.clock_inc     = 0.0;
        self.prev_trigger  = true;
        self.clock_samples = 0;
    }

    #[inline]
    pub fn next_phase(&mut self, clock_limit: f64, trigger_in: f32) -> f64 {
        if self.prev_trigger {
            if trigger_in <= 0.25 {
                self.prev_trigger = false;
            }

        } else if trigger_in > 0.75 {
            self.prev_trigger = true;

            if self.clock_samples > 0 {
                self.clock_inc =
                    1.0 / (self.clock_samples as f64);
            }

            self.clock_samples = 0;
        }

        self.clock_samples += 1;

        self.clock_phase += self.clock_inc;
        self.clock_phase = self.clock_phase % clock_limit;

        self.clock_phase
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TriggerSampleClock {
    prev_trigger:   bool,
    clock_samples:  u32,
    counter:        u32,
}

impl TriggerSampleClock {
    pub fn new() -> Self {
        Self {
            prev_trigger:   true,
            clock_samples:  0,
            counter:        0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.clock_samples = 0;
        self.counter       = 0;
    }

    #[inline]
    pub fn next(&mut self, trigger_in: f32) -> u32 {
        if self.prev_trigger {
            if trigger_in <= 0.25 {
                self.prev_trigger = false;
            }

        } else if trigger_in > 0.75 {
            self.prev_trigger  = true;
            self.clock_samples = self.counter;
            self.counter       = 0;
        }

        self.counter += 1;

        self.clock_samples
    }
}

/// Default size of the delay buffer: 5 seconds at 8 times 48kHz
const DEFAULT_DELAY_BUFFER_SAMPLES : usize = 8 * 48000 * 5;

#[derive(Debug, Clone)]
pub struct DelayBuffer {
    data:   Vec<f32>,
    wr:     usize,
    srate:  f32,
}

impl DelayBuffer {
    pub fn new() -> Self {
        Self {
            data:   vec![0.0; DEFAULT_DELAY_BUFFER_SAMPLES],
            wr:     0,
            srate:  44100.0,
        }
    }

    pub fn new_with_size(size: usize) -> Self {
        Self {
            data:   vec![0.0; size],
            wr:     0,
            srate:  44100.0,
        }
    }

    pub fn set_sample_rate(&mut self, srate: f32) {
        self.srate = srate;
    }

    pub fn reset(&mut self) {
        self.data.fill(0.0);
        self.wr = 0;
    }

    #[inline]
    pub fn feed(&mut self, input: f32) {
        self.data[self.wr] = input;
        self.wr = (self.wr + 1) % self.data.len();
    }

    #[inline]
    pub fn cubic_interpolate_at(&self, delay_time: f32) -> f32 {
        let data   = &self.data[..];
        let len    = data.len();
        let s_offs = (delay_time * self.srate) / 1000.0;
        let offs   = s_offs.floor() as usize % len;
        let fract  = s_offs.fract();

        let i = (self.wr + len) - offs;

        // Hermite interpolation, take from 
        // https://github.com/eric-wood/delay/blob/main/src/delay.rs#L52
        //
        // Thanks go to Eric Wood!
        //
        // For the interpolation code:
        // MIT License, Copyright (c) 2021 Eric Wood
        let xm1 = data[(i - 1) % len];
        let x0  = data[i       % len];
        let x1  = data[(i + 1) % len];
        let x2  = data[(i + 2) % len];

        let c     = (x1 - xm1) * 0.5;
        let v     = x0 - x1;
        let w     = c + v;
        let a     = w + v + (x2 - x0) * 0.5;
        let b_neg = w + a;

        let fract = fract as f32;
        (((a * fract) - b_neg) * fract + c) * fract + x0
    }

    #[inline]
    pub fn nearest_at(&self, delay_time: f32) -> f32 {
        let len  = self.data.len();
        let offs = (delay_time * self.srate).floor() as usize % len;
        let idx  = ((self.wr + len) - offs) % len;
        self.data[idx]
    }

    #[inline]
    pub fn at(&self, delay_sample_count: usize) -> f32 {
        let len  = self.data.len();
        let idx  = ((self.wr + len) - delay_sample_count) % len;
        self.data[idx]
    }
}

// translated from Odin 2 Synthesizer Plugin
// Copyright (C) 2020 TheWaveWarden
// under GPLv3 or any later
#[derive(Debug, Clone)]
pub struct DCBlockFilter {
    xm1:    f64,
    ym1:    f64,
    r:      f64,
}

impl DCBlockFilter {
    pub fn new() -> Self {
        Self {
            xm1: 0.0,
            ym1: 0.0,
            r:   0.995,
        }
    }

    pub fn reset(&mut self) {
        self.xm1 = 0.0;
        self.ym1 = 0.0;
    }

    pub fn set_sample_rate(&mut self, srate: f32) {
        self.r = 0.995;
        if srate > 90000.0 {
            self.r = 0.9965;
        } else if srate > 120000.0 {
            self.r = 0.997;
        }
    }

    pub fn next(&mut self, input: f32) -> f32 {
        let y = input as f64 - self.xm1 + self.r * self.ym1;
        self.xm1 = input as f64;
        self.ym1 = y;
        y as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_range2p_exp() {
        let a = p2range_exp(0.5, 1.0, 100.0);
        let x = range2p_exp(a, 1.0, 100.0);

        assert!((x - 0.5).abs() < std::f32::EPSILON);
    }

    #[test]
    fn check_range2p() {
        let a = p2range(0.5, 1.0, 100.0);
        let x = range2p(a, 1.0, 100.0);

        assert!((x - 0.5).abs() < std::f32::EPSILON);
    }
}
