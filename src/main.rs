mod sound;

use a2kit::lang::applesoft::tokenizer::Tokenizer;
use anyhow::{ensure, Context, Result};
use hound::{SampleFormat, WavSpec, WavWriter};
use itertools::Itertools;
use sound::{cycle, half_cycle, silence, tone, HalfCycle};
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{env, fs};

/// These parameters are chosen to be the same as `c2t`.
const SPEC: WavSpec = WavSpec {
    channels: 1,
    sample_rate: 11025,
    bits_per_sample: 8,
    sample_format: SampleFormat::Int,
};

fn main() {
    let (bas_file,) = env::args()
        .skip(1)
        .collect_tuple()
        .expect("expected 1 command line argument");
    let wav_file = wav_file_name(&bas_file).expect("expected .bas file extension");

    let bas_plaintext = fs::read_to_string(&bas_file).expect("failed to read plaintext");
    let bas_tokenized = Tokenizer::new()
        .tokenize(&bas_plaintext, 0x801)
        .expect("failed to tokenize basic program");

    let mut wav = WavWriter::create(wav_file, SPEC).expect("failed to write .wav header");
    first_segment(&mut wav, bas_tokenized.len()).expect("failed to write first segment");
    second_segment(&mut wav, &bas_tokenized).expect("failed to write second segment");
    silence(&mut wav, Duration::from_secs_f64(0.1)).expect("failed to write trailing silence");
    wav.finalize().expect("failed to finalize .wav file");
}

/// Convert `path/to/file-name.bas` to `path/to/file-name.wav`.
fn wav_file_name(bas_path: impl AsRef<Path>) -> Result<PathBuf> {
    let bas_path = bas_path.as_ref();

    let ext = bas_path
        .extension()
        .context("no extension")?
        .to_str()
        .context("non-unicode extension")?;
    ensure!(ext == "bas", "expected .bas got {ext:?}");

    let dir = bas_path.parent().context("no parent dir")?;
    let mut wav_name = bas_path.file_stem().context("no file name")?.to_owned();
    wav_name.push(".wav");
    Ok(dir.join(wav_name))
}

type Wav = WavWriter<BufWriter<File>>;

fn first_segment(wav: &mut Wav, len: usize) -> Result<()> {
    segment_header(wav)?;

    // Note that the length field is deliberately off-by-one. I haven't come
    // across any docs mentioning this (though I haven't explicitly looked), but
    // I did verify it experimentally.
    //
    // In particular, this means that it's possible for the payload segment to
    // contain exactly 64KiB of data, but it cannot be empty.
    ensure!(len != 0, "length must not be zero");
    let len_minus_one: u16 = (len - 1)
        .try_into()
        .with_context(|| format!("length must be at most 2^16: {len:?}"))?;

    let [len_low, len_high] = len_minus_one.to_le_bytes();
    let flag_byte = 0x55; // 0x55 to load; 0xd5 to load and run
    let checksum = 0xFF ^ len_low ^ len_high ^ flag_byte;

    byte(wav, len_low)?;
    byte(wav, len_high)?;
    byte(wav, flag_byte)?;
    byte(wav, checksum)?;

    segment_footer(wav)?;
    Ok(())
}

fn second_segment(wav: &mut Wav, bytes: &[u8]) -> Result<()> {
    segment_header(wav)?;

    let mut checksum = 0xFF;
    for &b in bytes {
        byte(wav, b)?;
        checksum ^= b;
    }
    byte(wav, checksum)?;

    segment_footer(wav)?;
    Ok(())
}

fn segment_header(wav: &mut Wav) -> Result<()> {
    tone(wav, 770., Duration::from_secs(4))?;
    sync_bit(wav)?;
    Ok(())
}

fn sync_bit(wav: &mut Wav) -> Result<()> {
    half_cycle(wav, 2500., HalfCycle::High)?;
    half_cycle(wav, 2000., HalfCycle::Low)?;
    Ok(())
}

/// c2t ends every segment with a trailing "1" bit, which gets ignored by the
/// Apple II. We do the same here.
///
/// This appears to be necessary for the last segment; otherwise the Apple II
/// doesn't read the last bit of the transmission, and hangs waiting for you to
/// finish. This fixes the issue. (We could also end the last segment with a
/// short "tone", but a single bit appears to suffice.)
fn segment_footer(wav: &mut Wav) -> Result<()> {
    bit(wav, true)
}

fn byte(wav: &mut Wav, byte: u8) -> Result<()> {
    // Most significant bit first.
    for i in (0..=7).rev() {
        let indicator = 1 << i;
        let is_set = byte & indicator != 0;
        bit(wav, is_set)?;
    }
    Ok(())
}

fn bit(wav: &mut Wav, bit: bool) -> Result<()> {
    if bit {
        cycle(wav, 1000.)
    } else {
        cycle(wav, 2000.)
    }
}
