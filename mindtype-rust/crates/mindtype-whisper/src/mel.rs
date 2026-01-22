//! Mel spectrogram computation for Whisper
//!
//! Whisper expects 80 mel bins at 16kHz, with:
//! - 25ms window (400 samples)
//! - 10ms hop (160 samples)
//! - Mel scale from 0 to 8000 Hz

use ndarray::Array2;
use rustfft::{num_complex::Complex, FftPlanner};
use std::f32::consts::PI;

/// Whisper audio parameters
pub const SAMPLE_RATE: usize = 16000;
pub const N_FFT: usize = 400;
pub const HOP_LENGTH: usize = 160;
pub const N_MELS: usize = 80;
pub const CHUNK_LENGTH: usize = 30; // seconds
pub const N_SAMPLES: usize = CHUNK_LENGTH * SAMPLE_RATE; // 480000
pub const N_FRAMES: usize = N_SAMPLES / HOP_LENGTH; // 3000

/// Compute mel spectrogram from audio samples
pub fn compute_mel_spectrogram(audio: &[f32]) -> Array2<f32> {
    // Pad or truncate to chunk length
    let mut padded = vec![0.0f32; N_SAMPLES];
    let copy_len = audio.len().min(N_SAMPLES);
    padded[..copy_len].copy_from_slice(&audio[..copy_len]);

    // Compute STFT
    let stft = compute_stft(&padded);

    // Convert to power spectrogram
    let power_spec = stft.mapv(|x| x * x);

    // Apply mel filterbank
    let mel_filters = create_mel_filterbank();
    let mel_spec = mel_filters.dot(&power_spec);

    // Convert to log scale (with small epsilon for numerical stability)
    let log_mel = mel_spec.mapv(|x| (x.max(1e-10)).ln());

    // Normalize to match Whisper's expected input range
    let max_val = log_mel.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let normalized = log_mel.mapv(|x| ((x - max_val).max(-8.0) + 4.0) / 4.0);

    normalized
}

/// Compute Short-Time Fourier Transform using FFT
fn compute_stft(audio: &[f32]) -> Array2<f32> {
    let n_frames = (audio.len() - N_FFT) / HOP_LENGTH + 1;
    let n_freqs = N_FFT / 2 + 1;

    let window = hann_window(N_FFT);
    let mut stft = Array2::zeros((n_freqs, n_frames));

    // Create FFT planner once for all frames
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(N_FFT);

    // Reusable buffer for FFT computation
    let mut fft_buffer: Vec<Complex<f32>> = vec![Complex::new(0.0, 0.0); N_FFT];

    for (frame_idx, frame_start) in (0..audio.len() - N_FFT + 1).step_by(HOP_LENGTH).enumerate() {
        if frame_idx >= n_frames {
            break;
        }

        // Apply window and copy to complex buffer
        for (i, (&sample, &w)) in audio[frame_start..frame_start + N_FFT]
            .iter()
            .zip(window.iter())
            .enumerate()
        {
            fft_buffer[i] = Complex::new(sample * w, 0.0);
        }

        // Compute FFT in-place
        fft.process(&mut fft_buffer);

        // Store magnitudes (only first half + 1 due to symmetry)
        for freq_idx in 0..n_freqs {
            let c = fft_buffer[freq_idx];
            stft[[freq_idx, frame_idx]] = (c.re * c.re + c.im * c.im).sqrt();
        }
    }

    stft
}

/// Create Hann window
fn hann_window(length: usize) -> Vec<f32> {
    (0..length)
        .map(|i| {
            0.5 * (1.0 - (2.0 * PI * i as f32 / (length - 1) as f32).cos())
        })
        .collect()
}

/// Create mel filterbank matrix
fn create_mel_filterbank() -> Array2<f32> {
    let n_freqs = N_FFT / 2 + 1;
    let fmin = 0.0f32;
    let fmax = (SAMPLE_RATE / 2) as f32;

    // Convert Hz to mel scale
    let mel_min = hz_to_mel(fmin);
    let mel_max = hz_to_mel(fmax);

    // Create mel points
    let mel_points: Vec<f32> = (0..=N_MELS + 1)
        .map(|i| mel_min + (mel_max - mel_min) * (i as f32) / (N_MELS + 1) as f32)
        .collect();

    // Convert back to Hz
    let hz_points: Vec<f32> = mel_points.iter().map(|&m| mel_to_hz(m)).collect();

    // Convert to FFT bins
    let bin_points: Vec<usize> = hz_points
        .iter()
        .map(|&hz| ((N_FFT + 1) as f32 * hz / SAMPLE_RATE as f32).floor() as usize)
        .collect();

    // Create filterbank
    let mut filterbank = Array2::zeros((N_MELS, n_freqs));

    for m in 0..N_MELS {
        let start = bin_points[m];
        let center = bin_points[m + 1];
        let end = bin_points[m + 2];

        // Rising slope
        for k in start..center {
            if center > start && k < n_freqs {
                filterbank[[m, k]] = (k - start) as f32 / (center - start) as f32;
            }
        }

        // Falling slope
        for k in center..end {
            if end > center && k < n_freqs {
                filterbank[[m, k]] = (end - k) as f32 / (end - center) as f32;
            }
        }
    }

    filterbank
}

/// Convert frequency in Hz to mel scale
fn hz_to_mel(hz: f32) -> f32 {
    2595.0 * (1.0 + hz / 700.0).log10()
}

/// Convert mel scale to Hz
fn mel_to_hz(mel: f32) -> f32 {
    700.0 * (10.0f32.powf(mel / 2595.0) - 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mel_spectrogram_shape() {
        let audio = vec![0.0f32; N_SAMPLES];
        let mel = compute_mel_spectrogram(&audio);
        // Verify 80 mel bins
        assert_eq!(mel.shape()[0], N_MELS);
        // STFT produces (N_SAMPLES - N_FFT) / HOP_LENGTH + 1 frames
        let expected_frames = (N_SAMPLES - N_FFT) / HOP_LENGTH + 1;
        assert_eq!(mel.shape()[1], expected_frames);
    }

    #[test]
    fn test_hann_window() {
        let window = hann_window(400);
        assert_eq!(window.len(), 400);
        // Window starts at 0
        assert!((window[0] - 0.0).abs() < 1e-6);
        // Window ends near 0
        assert!(window[399] < 1e-5);
        // Center values should be close to 1.0 (max is at (N-1)/2 = 199.5)
        // For even-length windows, indices 199 and 200 are both close to max
        let max_val = window.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        assert!((max_val - 1.0).abs() < 0.001);
        // Verify symmetry
        assert!((window[100] - window[299]).abs() < 1e-6);
    }

    #[test]
    fn test_hz_mel_conversion() {
        // Test roundtrip conversion
        let hz = 1000.0;
        let mel = hz_to_mel(hz);
        let hz_back = mel_to_hz(mel);
        assert!((hz - hz_back).abs() < 0.01);
    }

    #[test]
    fn test_mel_filterbank_shape() {
        let filterbank = create_mel_filterbank();
        let n_freqs = N_FFT / 2 + 1;
        assert_eq!(filterbank.shape(), &[N_MELS, n_freqs]);
    }

    #[test]
    fn test_stft_output() {
        // Test STFT on simple signal
        let audio = vec![0.0f32; N_SAMPLES];
        let stft = compute_stft(&audio);
        let n_freqs = N_FFT / 2 + 1;
        let n_frames = (N_SAMPLES - N_FFT) / HOP_LENGTH + 1;
        assert_eq!(stft.shape(), &[n_freqs, n_frames]);
    }
}
