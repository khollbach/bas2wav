use anyhow::Result;
use std::{f64::consts::PI, time::Duration};

use crate::Wav;

pub fn tone(wav: &mut Wav, freq: f64, dur: Duration) -> Result<()> {
    sine_wave(wav, freq, dur, false)
}

/// A single cycle of a sine wave, of the given frequency.
pub fn cycle(wav: &mut Wav, freq: f64) -> Result<()> {
    let dur = Duration::from_secs_f64(1. / freq);
    sine_wave(wav, freq, dur, false)
}

pub enum HalfCycle {
    High,
    Low,
}

/// Half of one cycle of a sine wave of the given frequency.
///
/// The sample values are either positive or negative, depending on `half`.
pub fn half_cycle(wav: &mut Wav, freq: f64, half: HalfCycle) -> Result<()> {
    let dur = Duration::from_secs_f64(1. / freq / 2.);
    let invert = matches!(half, HalfCycle::Low);
    sine_wave(wav, freq, dur, invert)
}

pub fn silence(wav: &mut Wav, dur: Duration) -> Result<()> {
    let num_samples = dur.as_secs_f64() * wav.spec().sample_rate as f64;
    for _ in 0..num_samples as u32 {
        wav.write_sample(0)?;
    }
    Ok(())
}

/// Three-quarters of the maximum voltage value, assuming each sample is i8.
const AMPLITUDE: f64 = i8::MAX as f64 * 3. / 4.;

fn sine_wave(wav: &mut Wav, freq: f64, dur: Duration, invert: bool) -> Result<()> {
    let num_samples = dur.as_secs_f64() * wav.spec().sample_rate as f64;
    for i in 0..num_samples as u32 {
        let time = i as f64 / wav.spec().sample_rate as f64; // (in seconds)
        let sign = if invert { -1. } else { 1. };
        let sample = (time * freq * 2. * PI).sin() * sign; // [-1, 1]
        wav.write_sample((sample * AMPLITUDE) as i8)?; // [i8::MIN, i8::MAX]
    }
    Ok(())
}
