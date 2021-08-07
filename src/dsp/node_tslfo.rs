// Copyright (c) 2021 Weird Constructor <weirdconstructor@gmail.com>
// This file is a part of HexoDSP. Released under GPL-3.0-or-later.
// See README.md and COPYING for details.

use crate::nodes::{NodeAudioContext, NodeExecContext};
use crate::dsp::{
    NodeId, SAtom, ProcBuf, DspNode, LedPhaseVals,
    GraphAtomData, GraphFun, NodeContext,
};
use super::helpers::{TriSawLFO, Trigger};

#[derive(Debug, Clone)]
pub struct TsLfo {
    lfo:    Box<TriSawLFO>,
    trig:   Trigger,
}

impl TsLfo {
    pub fn new(_nid: &NodeId) -> Self {
        Self {
            lfo:  Box::new(TriSawLFO::new()),
            trig: Trigger::new(),
        }
    }

    pub const time : &'static str =
        "TsLfo time\nThe frequency or period time of the LFO, goes all the \
        way from 0.1ms up to 30s. Please note, that the text entry is always \
        in milliseconds.\nRange: (0..1)\n";
    pub const trig : &'static str =
        "TsLfo trig\nTriggers a phase reset of the LFO.\nRange: (0..1)\n";
    pub const rev : &'static str =
        "TsLfo rev\nThe reverse point of the LFO waveform. At 0.5 the LFO \
        will follow a triangle waveform. At 0.0 or 1.0 the LFO waveform will \
        be (almost) a (reversed) saw tooth. Node: A perfect sawtooth can not be \
        achieved with this oscillator, as there will always be a minimal \
        rise/fall time.\nRange: (0..1)\n";
    pub const sig : &'static str =
        "TsLfo sig\nThe LFO output.\nRange: (0..1)";
    pub const DESC : &'static str =
r#"TriSaw LFO

This simple LFO has a configurable waveform. You can blend between triangular to sawtooth waveforms using the 'rev' parameter.
"#;
    pub const HELP : &'static str =
r#"TsLfo - TriSaw LFO

This simple LFO has a configurable waveform. You can blend between
triangular to sawtooth waveforms using the 'rev' parameter.

Using the 'trig' input you can reset the LFO phase, which allows to use it
kind of like an envelope.
"#;

}

impl DspNode for TsLfo {
    fn outputs() -> usize { 1 }

    fn set_sample_rate(&mut self, srate: f32) {
        self.lfo.set_sample_rate(srate);
    }

    fn reset(&mut self) {
        self.lfo.reset();
        self.trig.reset();
    }

    #[inline]
    fn process<T: NodeAudioContext>(
        &mut self, ctx: &mut T, _ectx: &mut NodeExecContext,
        _nctx: &NodeContext,
        atoms: &[SAtom], inputs: &[ProcBuf],
        outputs: &mut [ProcBuf], ctx_vals: LedPhaseVals)
    {
        use crate::dsp::{out, inp, denorm, at};

        let time = inp::TsLfo::time(inputs);
        let trig = inp::TsLfo::trig(inputs);
        let rev  = inp::TsLfo::rev(inputs);
        let out  = out::TsLfo::sig(outputs);

        let mut lfo = &mut *self.lfo;

        for frame in 0..ctx.nframes() {
            if self.trig.check_trigger(denorm::TsLfo::trig(trig, frame)) {
                lfo.reset();
            }

            let time_ms = denorm::TsLfo::time(time, frame).clamp(0.1, 300000.0);

            lfo.set(
                1000.0 / time_ms,
                denorm::TsLfo::rev(rev, frame));

            out.write(frame, lfo.next_unipolar() as f32);
        }

        ctx_vals[0].set(out.read(ctx.nframes() - 1));
    }

    fn graph_fun() -> Option<GraphFun> {
        let mut lfo = TriSawLFO::new();
        lfo.set_sample_rate(160.0);

        Some(Box::new(move |gd: &dyn GraphAtomData, init: bool, _x: f32, xn: f32| -> f32 {
            if init {
                lfo.reset();
                let time_idx = NodeId::TsLfo(0).inp_param("time").unwrap().inp();
                let rev_idx  = NodeId::TsLfo(0).inp_param("rev").unwrap().inp();

                let time = gd.get_norm(time_idx as u32).sqrt();
                let rev  = gd.get_norm(rev_idx as u32);
                lfo.set(5.0 * (1.0 - time) + time * 1.0, rev);
            }

            lfo.next_unipolar() as f32
        }))
    }
}