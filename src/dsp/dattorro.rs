// Copyright (c) 2021 Weird Constructor <weirdconstructor@gmail.com>
// This file is a part of HexoDSP. Released under GPL-3.0-or-later.
// See README.md and COPYING for details.

// This file contains a reverb implementation that is based
// on Jon Dattorro's 1997 reverb algorithm. It's also largely
// based on the C++ implementation from ValleyAudio / ValleyRackFree
//
// ValleyRackFree Copyright (C) 2020, Valley Audio Soft, Dale Johnson
// Adapted under the GPL-3.0-or-later License.
//
// See also: https://github.com/ValleyAudio/ValleyRackFree/blob/v1.0/src/Plateau/Dattorro.cpp
//      and: https://github.com/ValleyAudio/ValleyRackFree/blob/v1.0/src/Plateau/Dattorro.hpp
//
// And: https://ccrma.stanford.edu/~dattorro/music.html
// And: https://ccrma.stanford.edu/~dattorro/EffectDesignPart1.pdf

const DAT_SAMPLE_RATE    : f32 = 29761.0;
const DAT_SAMPLES_PER_MS : f32 = DAT_SAMPLE_RATE / 1000.0;

const DAT_INPUT_APF_TIMES_MS : [f32; 4] = [
    141.0 / DAT_SAMPLES_PER_MS,
    107.0 / DAT_SAMPLES_PER_MS,
    379.0 / DAT_SAMPLES_PER_MS,
    277.0 / DAT_SAMPLES_PER_MS,
];

const DAT_LEFT_APF1_TIME_MS  : f32 = 672.0  / DAT_SAMPLES_PER_MS;
const DAT_LEFT_APF2_TIME_MS  : f32 = 1800.0 / DAT_SAMPLES_PER_MS;

const DAT_RIGHT_APF1_TIME_MS : f32 = 908.0  / DAT_SAMPLES_PER_MS;
const DAT_RIGHT_APF2_TIME_MS : f32 = 2656.0 / DAT_SAMPLES_PER_MS;

const DAT_LEFT_DELAY1_TIME_MS : f32 = 4453.0  / DAT_SAMPLES_PER_MS;
const DAT_LEFT_DELAY2_TIME_MS : f32 = 3720.0  / DAT_SAMPLES_PER_MS;

const DAT_RIGHT_DELAY1_TIME_MS : f32 = 4217.0 / DAT_SAMPLES_PER_MS;
const DAT_RIGHT_DELAY2_TIME_MS : f32 = 3163.0 / DAT_SAMPLES_PER_MS;

const DAT_TAPS_TIME_MS : [f32; 7] = [
    266.0  / DAT_SAMPLES_PER_MS,
    2974.0 / DAT_SAMPLES_PER_MS,
    1913.0 / DAT_SAMPLES_PER_MS,
    1996.0 / DAT_SAMPLES_PER_MS,
    1990.0 / DAT_SAMPLES_PER_MS,
    187.0  / DAT_SAMPLES_PER_MS,
    1066.0 / DAT_SAMPLES_PER_MS,
];

const DAT_LFO_FREQS_HZ : [f32; 4] = [ 0.1, 0.15, 0.12, 0.18 ];

const DAT_INPUT_DIFFUSION1 : f32 = 0.75;
const DAT_INPUT_DIFFUSION2 : f32 = 0.625;
const DAT_PLATE_DIFFUSION1 : f32 = 0.7;
const DAT_PLATE_DIFFUSION2 : f32 = 0.5;

const DAT_LFO_EXCURSION_MS : f32 = 16.0 / DAT_SAMPLES_PER_MS;

use crate::dsp::helpers::{
    AllPass,
    TriSawLFO,
    OnePoleLPF,
    OnePoleHPF,
    DelayBuffer,
    DCBlockFilter
};

pub struct DattorroReverb {
    last_scale: f32,

    inp_dc_block:   [DCBlockFilter; 2],
    out_dc_block:   [DCBlockFilter; 2],

    lfos: [TriSawLFO; 4],

    input_hpf: OnePoleHPF,
    input_lpf: OnePoleLPF,

    pre_delay:  DelayBuffer,
    input_apfs: [(AllPass, f32, f32); 4],

    apf1:   [(AllPass, f32, f32); 2],
    hpf:    [OnePoleHPF; 2],
    lpf:    [OnePoleLPF; 2],
    apf2:   [(AllPass, f32, f32); 2],
    delay1: [(DelayBuffer, f32); 2],
    delay2: [(DelayBuffer, f32); 2],
}

pub trait DattorroReverbParams {
    /// Time for the pre-delay of the reverb.
    fn pre_delay_time_s(&self) -> f32;
    fn time_scale(&self) -> f32;
}

impl DattorroReverb {
    pub fn new() -> Self {
        let mut this = Self {
            last_scale: 1.0,

            inp_dc_block:   [DCBlockFilter::new(); 2],
            out_dc_block:   [DCBlockFilter::new(); 2],

            lfos: [TriSawLFO::new(); 4],

            input_hpf: OnePoleHPF::new(),
            input_lpf: OnePoleLPF::new(),

            pre_delay:  DelayBuffer::new(),
            input_apfs: Default::default(),

            apf1:   Default::default(),
            hpf:    [OnePoleHPF::new(); 2],
            lpf:    [OnePoleLPF::new(); 2],
            apf2:   Default::default(),
            delay1: Default::default(),
            delay2: Default::default(),
        };

        this.reset();

        this
    }

    pub fn reset(&mut self) {
        self.input_lpf.reset();
        self.input_hpf.reset();

        self.input_lpf.set_freq(22000.0);
        self.input_hpf.set_freq(0.0);

        self.input_apfs[0] =
            (AllPass::new(), DAT_INPUT_APF_TIMES_MS[0], DAT_INPUT_DIFFUSION1);
        self.input_apfs[1] =
            (AllPass::new(), DAT_INPUT_APF_TIMES_MS[1], DAT_INPUT_DIFFUSION1);
        self.input_apfs[2] =
            (AllPass::new(), DAT_INPUT_APF_TIMES_MS[2], DAT_INPUT_DIFFUSION2);
        self.input_apfs[3] =
            (AllPass::new(), DAT_INPUT_APF_TIMES_MS[3], DAT_INPUT_DIFFUSION2);

        self.apf1[0] =
            (AllPass::new(), DAT_LEFT_APF1_TIME_MS, -DAT_PLATE_DIFFUSION1);
        self.apf1[1] =
            (AllPass::new(), DAT_RIGHT_APF1_TIME_MS, -DAT_PLATE_DIFFUSION1);
        self.apf2[0] =
            (AllPass::new(), DAT_LEFT_APF2_TIME_MS, -DAT_PLATE_DIFFUSION2);
        self.apf2[1] =
            (AllPass::new(), DAT_RIGHT_APF2_TIME_MS, -DAT_PLATE_DIFFUSION2);

        self.delay1[0] = (DelayBuffer::new(), DAT_LEFT_DELAY1_TIME_MS);
        self.delay1[1] = (DelayBuffer::new(), DAT_RIGHT_DELAY1_TIME_MS);
        self.delay2[0] = (DelayBuffer::new(), DAT_LEFT_DELAY2_TIME_MS);
        self.delay2[1] = (DelayBuffer::new(), DAT_RIGHT_DELAY2_TIME_MS);

        self.lpf[0].reset();
        self.lpf[1].reset();
        self.lpf[0].set_freq(10000.0);
        self.lpf[1].set_freq(10000.0);

        self.hpf[0].reset();
        self.hpf[1].reset();
        self.hpf[0].set_freq(0.0);
        self.hpf[1].set_freq(0.0);

        self.lfos[0].set(DAT_LFO_FREQS_HZ[0], 0.5);
        self.lfos[0].set_phase_offs(0.0);
        self.lfos[0].reset();
        self.lfos[1].set(DAT_LFO_FREQS_HZ[1], 0.5);
        self.lfos[1].set_phase_offs(0.25);
        self.lfos[1].reset();
        self.lfos[2].set(DAT_LFO_FREQS_HZ[2], 0.5);
        self.lfos[2].set_phase_offs(0.5);
        self.lfos[2].reset();
        self.lfos[3].set(DAT_LFO_FREQS_HZ[3], 0.5);
        self.lfos[3].set_phase_offs(0.75);
        self.lfos[3].reset();

        self.set_time_scale(1.0);
    }

    #[inline]
    pub fn set_time_scale(&mut self, scale: f32) {
        if (self.last_scale - scale).abs() > std::f32::EPSILON {
            let scale = scale.max(0.0001);
            self.last_scale = scale;

            self.apf1[0].1 = DAT_LEFT_APF1_TIME_MS  * scale;
            self.apf1[1].1 = DAT_RIGHT_APF1_TIME_MS * scale;
            self.apf2[0].1 = DAT_LEFT_APF2_TIME_MS  * scale;
            self.apf2[1].1 = DAT_RIGHT_APF2_TIME_MS * scale;
        }
    }

    pub fn set_sample_rate(&mut self, srate: f32) {
    }

    pub fn process(
        &mut self,
        params: &mut dyn DattorroReverbParams,
        input_l: f32, input_r: f32
    ) -> (f32, f32)
    {
        self.set_time_scale(params.time_scale());

        (0.0, 0.0)
    }
}
