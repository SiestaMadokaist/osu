use std::f32::consts::PI;

use rustfft::{FftPlanner, num_complex::Complex};
use serde::Serialize;

const FFT_SIZE: usize = 2048;
const HOP: usize = 512;
/// Minimum gap between two notes in the same lane.
const MIN_GAP_SECS: f32 = 0.30;
/// Half-width of the window used for the adaptive threshold.
const THRESHOLD_SECS: f32 = 0.8;
/// A peak must also be the maximum within this many frames on each side.
const PEAK_RADIUS: usize = 3;

#[derive(Serialize, Clone)]
pub struct Lane {
    pub name: &'static str,
    pub lo: f32,
    pub hi: f32,
}

#[derive(Serialize)]
pub struct Note {
    /// Seconds from the start of the audio.
    pub t: f32,
    pub lane: usize,
    /// Normalised onset strength, roughly 0..1.
    pub s: f32,
}

pub fn lanes() -> Vec<Lane> {
    vec![
        Lane { name: "kick", lo: 40.0, hi: 100.0 },
        Lane { name: "bass", lo: 100.0, hi: 200.0 },
        Lane { name: "snare", lo: 200.0, hi: 500.0 },
        Lane { name: "low-mid", lo: 500.0, hi: 1200.0 },
        Lane { name: "mid", lo: 1200.0, hi: 3000.0 },
        Lane { name: "presence", lo: 3000.0, hi: 6000.0 },
        Lane { name: "high", lo: 6000.0, hi: 12000.0 },
    ]
}

pub fn detect(samples: &[f32], sample_rate: u32, lanes: &[Lane]) -> Vec<Note> {
    let flux = spectral_flux(samples, sample_rate, lanes);
    let frame_secs = HOP as f32 / sample_rate as f32;
    let mut notes = Vec::new();
    for (lane, series) in flux.iter().enumerate() {
        for (frame, strength) in pick_peaks(series, frame_secs) {
            // Frame centre, in seconds.
            let t = (frame * HOP + FFT_SIZE / 2) as f32 / sample_rate as f32;
            notes.push(Note { t, lane, s: strength.min(1.5) });
        }
    }
    notes.sort_by(|a, b| a.t.total_cmp(&b.t));
    notes
}

/// Per-lane onset strength curves: the sum of positive log-magnitude changes
/// between consecutive STFT frames, restricted to each lane's frequency band.
fn spectral_flux(samples: &[f32], sample_rate: u32, lanes: &[Lane]) -> Vec<Vec<f32>> {
    let fft = FftPlanner::<f32>::new().plan_fft_forward(FFT_SIZE);
    let window: Vec<f32> = (0..FFT_SIZE)
        .map(|i| 0.5 - 0.5 * (2.0 * PI * i as f32 / FFT_SIZE as f32).cos())
        .collect();
    let bin_hz = sample_rate as f32 / FFT_SIZE as f32;
    let ranges: Vec<_> = lanes
        .iter()
        .map(|l| (l.lo / bin_hz).round() as usize..(l.hi / bin_hz).round() as usize)
        .collect();

    let frames = samples.len().saturating_sub(FFT_SIZE) / HOP + 1;
    let mut flux = vec![vec![0.0f32; frames]; lanes.len()];
    let mut prev = vec![0.0f32; FFT_SIZE / 2];
    let mut buf = vec![Complex::new(0.0, 0.0); FFT_SIZE];

    for frame in 0..frames {
        let start = frame * HOP;
        for (i, slot) in buf.iter_mut().enumerate() {
            let s = samples.get(start + i).copied().unwrap_or(0.0);
            *slot = Complex::new(s * window[i], 0.0);
        }
        fft.process(&mut buf);
        let mag: Vec<f32> = buf[..FFT_SIZE / 2].iter().map(|c| (1.0 + 100.0 * c.norm()).ln()).collect();
        if frame > 0 {
            for (lane, range) in ranges.iter().enumerate() {
                flux[lane][frame] = range
                    .clone()
                    .filter(|&b| b < mag.len())
                    .map(|b| (mag[b] - prev[b]).max(0.0))
                    .sum();
            }
        }
        prev = mag;
    }

    // Scale each lane by its 99th percentile so lanes are comparable.
    for series in &mut flux {
        let mut sorted = series.clone();
        sorted.sort_by(f32::total_cmp);
        let p99 = sorted[(sorted.len() as f32 * 0.99) as usize % sorted.len().max(1)];
        if p99 > 0.0 {
            series.iter_mut().for_each(|v| *v /= p99);
        }
    }
    flux
}

/// Adaptive-threshold peak picking. Returns `(frame, strength)` pairs.
fn pick_peaks(series: &[f32], frame_secs: f32) -> Vec<(usize, f32)> {
    let half = (THRESHOLD_SECS / frame_secs) as usize;
    let min_gap = (MIN_GAP_SECS / frame_secs) as usize;
    let mut peaks: Vec<(usize, f32)> = Vec::new();

    for i in PEAK_RADIUS..series.len().saturating_sub(PEAK_RADIUS) {
        let v = series[i];
        let neighbours = &series[i - PEAK_RADIUS..=i + PEAK_RADIUS];
        if v < neighbours.iter().copied().fold(0.0, f32::max) {
            continue;
        }
        let window = &series[i.saturating_sub(half)..(i + half).min(series.len())];
        let mean = window.iter().sum::<f32>() / window.len() as f32;
        if v <= mean * 1.5 + 0.15 {
            continue;
        }
        match peaks.last_mut() {
            Some(last) if i - last.0 < min_gap => {
                if v > last.1 {
                    *last = (i, v);
                }
            }
            _ => peaks.push((i, v)),
        }
    }
    peaks
}
