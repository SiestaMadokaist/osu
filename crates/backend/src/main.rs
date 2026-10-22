mod audio;
mod onsets;

use std::{env, fs, path::PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;

#[derive(Serialize)]
struct Map {
    audio: &'static str,
    duration: f32,
    lanes: Vec<onsets::Lane>,
    notes: Vec<onsets::Note>,
}

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let input = PathBuf::from(args.next().context("usage: backend <input.mp3> [out_dir]")?);
    let out_dir = PathBuf::from(args.next().unwrap_or_else(|| "crates/frontend/public".into()));

    let decoded = audio::decode_mono(&input)?;
    let duration = decoded.samples.len() as f32 / decoded.sample_rate as f32;
    let lanes = onsets::lanes();
    let notes = onsets::detect(&decoded.samples, decoded.sample_rate, &lanes);
    eprintln!("{:.1}s of audio, {} notes", duration, notes.len());

    fs::create_dir_all(&out_dir)?;
    fs::copy(&input, out_dir.join("audio.mp3"))?;
    let map = Map { audio: "audio.mp3", duration, lanes, notes };
    fs::write(out_dir.join("map.json"), serde_json::to_vec(&map)?)?;
    Ok(())
}
