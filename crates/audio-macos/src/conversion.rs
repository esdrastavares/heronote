//! Audio sample conversion utilities
//!
//! This module provides functions for converting between different audio sample formats
//! and channel configurations. All conversions normalize to f32 samples in the range [-1.0, 1.0].

/// Convert I16 samples to F32 normalized range [-1.0, 1.0]
///
/// Handles the edge case where i16::MIN would overflow when negated,
/// mapping it directly to -1.0.
#[inline]
pub fn i16_to_f32(sample: i16) -> f32 {
    if sample == i16::MIN {
        -1.0
    } else {
        sample as f32 / i16::MAX as f32
    }
}

/// Convert I32 samples to F32 normalized range [-1.0, 1.0]
///
/// Handles the edge case where i32::MIN would overflow when negated,
/// mapping it directly to -1.0.
#[inline]
pub fn i32_to_f32(sample: i32) -> f32 {
    if sample == i32::MIN {
        -1.0
    } else {
        sample as f32 / i32::MAX as f32
    }
}

/// Convert F64 samples to F32
#[inline]
pub fn f64_to_f32(sample: f64) -> f32 {
    sample as f32
}

/// Convert a slice of I16 samples to F32
pub fn convert_i16_slice_to_f32(data: &[i16]) -> Vec<f32> {
    data.iter().map(|&s| i16_to_f32(s)).collect()
}

/// Convert a slice of I32 samples to F32
pub fn convert_i32_slice_to_f32(data: &[i32]) -> Vec<f32> {
    data.iter().map(|&s| i32_to_f32(s)).collect()
}

/// Convert multi-channel audio to mono by averaging all channels
///
/// If the input is already mono (channels == 1), returns a clone of the input.
pub fn convert_to_mono(data: &[f32], channels: usize) -> Vec<f32> {
    if channels == 1 {
        return data.to_vec();
    }

    data.chunks(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_i16_to_f32_normal() {
        assert!((i16_to_f32(0) - 0.0).abs() < f32::EPSILON);
        assert!((i16_to_f32(i16::MAX) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_i16_to_f32_min_value() {
        assert!((i16_to_f32(i16::MIN) - (-1.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_i32_to_f32_min_value() {
        assert!((i32_to_f32(i32::MIN) - (-1.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_convert_to_mono_stereo() {
        let stereo = vec![0.5, -0.5, 1.0, -1.0];
        let mono = convert_to_mono(&stereo, 2);
        assert_eq!(mono.len(), 2);
        assert!((mono[0] - 0.0).abs() < f32::EPSILON);
        assert!((mono[1] - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_convert_to_mono_already_mono() {
        let mono_input = vec![0.5, -0.5, 1.0];
        let mono_output = convert_to_mono(&mono_input, 1);
        assert_eq!(mono_input, mono_output);
    }
}
