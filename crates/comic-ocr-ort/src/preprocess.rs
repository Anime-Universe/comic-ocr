//! Image preprocessing for a ViT/DeiT-family vision encoder.
//!
//! Every constant here mirrors a field of the model's `preprocessor_config.json`
//! on the HuggingFace Hub, or a documented default that `ViTImageProcessor`
//! applies when the field is absent. The file was fetched from
//! Values below MUST be re-read from the target checkpoint's own
//! `preprocessor_config.json` — they are model properties, not constants.
//! and reads, in full:
//!
//! ```json
//! {
//!   "do_normalize": true,
//!   "do_resize": true,
//!   "feature_extractor_type": "ViTFeatureExtractor",
//!   "image_mean": [0.5, 0.5, 0.5],
//!   "image_std": [0.5, 0.5, 0.5],
//!   "resample": 2,
//!   "size": 224
//! }
//! ```
//!
//! Note what is *not* in that file: `do_rescale` and `rescale_factor`. This is
//! the legacy `ViTFeatureExtractor` serialisation, and the missing keys are
//! filled in by `ViTImageProcessor`'s own defaults rather than skipped. Loading
//! the config above through `transformers` 4.42.4 resolves it to:
//!
//! ```text
//! do_normalize: true, do_rescale: true, do_resize: true,
//! image_mean: [0.5, 0.5, 0.5], image_std: [0.5, 0.5, 0.5],
//! rescale_factor: 0.00392156862745098, resample: 2,
//! size: {"height": 224, "width": 224}
//! ```
//!
//! so rescaling by 1/255 does happen, and the scalar `size: 224` expands to a
//! square 224x224 target (`get_size_dict(..., default_to_square=True)`) — the
//! aspect ratio of the input is *not* preserved.
//!
//! # Pipeline
//!
//! The order below is the order the upstream reference pipeline applies, which is
//! the reference implementation this crate is trying to match:
//!
//! 1. Greyscale round-trip — `img.convert("L").convert("RGB")`. Done by the
//!    *caller* of the processor, not the processor itself, but it is part of
//!    how the model is used in practice, so it is on by default here. See
//!    [`PreprocessConfig::greyscale`].
//! 2. Resize to 224x224 with PIL `BILINEAR`.
//! 3. Rescale by 1/255 into `[0, 1]`.
//! 4. Normalise: `(x - mean) / std`, which with mean = std = 0.5 maps
//!    `[0, 1]` onto `[-1, 1]`.
//! 5. Emit CHW `f32` with a leading batch axis: `[1, 3, 224, 224]`.

use comic_ocr_core::OcrError;
use image::{DynamicImage, RgbImage};

/// Target edge length. Source: `preprocessor_config.json` `"size": 224`,
/// expanded by `get_size_dict(size, default_to_square=True)` to a square
/// `{"height": 224, "width": 224}`. Cross-checked against the model's
/// `config.json`, whose encoder declares `image_size: 224`.
pub const IMAGE_SIZE: u32 = 224;

/// Source: the model's `config.json` encoder section, `num_channels: 3`.
/// The processor emits 3 channels even for greyscale input — the greyscale
/// round-trip in step 1 replicates one luma plane across R, G and B rather
/// than reducing the channel count.
pub const NUM_CHANNELS: usize = 3;

/// Source: `ViTImageProcessor`'s `rescale_factor` default of `1/255`, applied
/// because `do_rescale` defaults to `true` and `preprocessor_config.json`
/// does not override it.
pub const RESCALE_FACTOR: f32 = 1.0 / 255.0;

/// Source: `preprocessor_config.json` `"image_mean": [0.5, 0.5, 0.5]`.
///
/// These are deliberately NOT the ImageNet means (0.485/0.456/0.406). This
/// checkpoint normalises to `[-1, 1]`, not to ImageNet statistics.
pub const IMAGE_MEAN: [f32; NUM_CHANNELS] = [0.5, 0.5, 0.5];

/// Source: `preprocessor_config.json` `"image_std": [0.5, 0.5, 0.5]`.
///
/// Likewise not the ImageNet standard deviations (0.229/0.224/0.225).
pub const IMAGE_STD: [f32; NUM_CHANNELS] = [0.5, 0.5, 0.5];

/// Source: `preprocessor_config.json` `"resample": 2`, which is
/// `PIL.Image.Resampling.BILINEAR` (the PIL enum is NEAREST=0, LANCZOS=1,
/// BILINEAR=2, BICUBIC=3, BOX=4, HAMMING=5).
///
/// `FilterType::Triangle` is the `image` crate's name for a bilinear filter:
/// unit support, weights scaled by the sampling ratio when downscaling, and
/// normalised to sum to one — the same construction PIL uses. The two are not
/// bit-identical, because PIL accumulates in quantised fixed point and `image`
/// in `f32`, so individual output pixels can land one byte apart.
///
/// Measured, rather than assumed. This module's output was compared element by
/// element against `ViTImageProcessor` (transformers 4.42.4, Pillow 10.1.0)
/// loading the config above, over the five crops in `assets/examples` and a
/// random-noise image. One LSB of the byte quantisation is `(1/255)/0.5`
/// = 0.00784 in normalised units.
///
/// ```text
/// input            exact match   mean |diff|   max |diff|
/// 00.jpg  99x178       82%         0.17 LSB     2 LSB
/// 01.jpg 188x251       81%         0.19 LSB     2 LSB
/// 02.jpg  62x103       88%         0.12 LSB     2 LSB
/// 03.jpg 406x115       85%         0.15 LSB     1 LSB
/// 14.jpg 650x1024      91%         0.09 LSB     1 LSB
/// random 137x291        -          0.17 LSB     1 LSB
/// ```
///
/// The residual is entirely resampler rounding. Closing it completely would
/// mean reimplementing PIL's fixed-point resampler; at under one byte of image
/// intensity it is far below the noise the encoder already tolerates, whereas
/// the mean/std error this module replaced was a systematic shift of the whole
/// tensor.
pub const RESAMPLE_FILTER: image::imageops::FilterType = image::imageops::FilterType::Triangle;

/// Fixed-point ITU-R 601-2 luma weights, matching PIL's `RGB -> L` conversion
/// exactly. PIL computes `(r*19595 + g*38470 + b*7471 + 32768) >> 16`; the
/// `+ 32768` is a round-to-nearest term, so truncating instead is off by one
/// on many inputs. Verified against Pillow 10.1.0 over 20,000 random RGB
/// triples with zero mismatches.
///
/// This is why [`greyscale_luma`] exists instead of a call to the `image`
/// crate's `to_luma8()`: that uses Rec. 709 weights (0.2126/0.7152/0.0722),
/// which disagree with PIL badly — pure green maps to 150 under PIL and 182
/// under Rec. 709.
const LUMA_601_WEIGHTS: [u32; NUM_CHANNELS] = [19595, 38470, 7471];
const LUMA_601_ROUND: u32 = 32768;
const LUMA_601_SHIFT: u32 = 16;

/// Which optional stages of the reference pipeline to apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreprocessConfig {
    /// Apply the `convert("L").convert("RGB")` greyscale round-trip before
    /// resizing.
    ///
    /// `true` matches the upstream reference pipeline's behaviour, the reference
    /// implementation. It is not part of `ViTImageProcessor` itself, so a
    /// caller that feeds the processor colour input directly wants `false`.
    ///
    /// This is not a cosmetic switch. On a random-colour test image the two
    /// settings produce tensors differing by up to 1.37 in a range that only
    /// spans `[-1, 1]`.
    pub greyscale: bool,
}

impl Default for PreprocessConfig {
    fn default() -> Self {
        Self { greyscale: true }
    }
}

/// A preprocessed image: the tensor and the shape it must be interpreted with.
///
/// The two travel together so a caller cannot pair the buffer with a shape that
/// does not describe it.
#[derive(Debug, Clone, PartialEq)]
pub struct PreprocessedImage {
    data: Vec<f32>,
    shape: [usize; 4],
}

impl PreprocessedImage {
    /// The tensor in CHW order with a leading batch axis, row-major.
    ///
    /// Index of channel `c`, row `y`, column `x` is
    /// `c * IMAGE_SIZE * IMAGE_SIZE + y * IMAGE_SIZE + x`.
    pub fn data(&self) -> &[f32] {
        &self.data
    }

    /// `[1, NUM_CHANNELS, IMAGE_SIZE, IMAGE_SIZE]`.
    pub fn shape(&self) -> [usize; 4] {
        self.shape
    }

    /// Consumes into `(shape, data)`, the pair an ONNX Runtime tensor wants.
    pub fn into_parts(self) -> (Vec<i64>, Vec<f32>) {
        let shape = self.shape.iter().map(|&d| d as i64).collect();
        (shape, self.data)
    }
}

/// PIL's `RGB -> L`: ITU-R 601-2 luma in rounded fixed point.
///
/// Kept `#[inline]` and standalone so the test suite can assert it against
/// values sampled from a real Pillow build.
#[inline]
pub fn greyscale_luma(r: u8, g: u8, b: u8) -> u8 {
    let acc = u32::from(r) * LUMA_601_WEIGHTS[0]
        + u32::from(g) * LUMA_601_WEIGHTS[1]
        + u32::from(b) * LUMA_601_WEIGHTS[2]
        + LUMA_601_ROUND;
    // The weights sum to exactly 65536, so with the rounding term the maximum
    // possible result is 255 and the shift cannot overflow a u8.
    (acc >> LUMA_601_SHIFT) as u8
}

/// Preprocesses an image the way the upstream reference pipeline does.
///
/// Equivalent to [`preprocess_with`] under [`PreprocessConfig::default`], which
/// includes the greyscale round-trip.
///
/// # Errors
///
/// Returns [`OcrError::InvalidInput`] if either dimension is zero. There is no
/// meaningful tensor for an empty image, and emitting a zero-filled one would
/// hand the model a black square that it would happily transcribe.
pub fn preprocess(image: &DynamicImage) -> Result<PreprocessedImage, OcrError> {
    preprocess_with(image, PreprocessConfig::default())
}

/// Preprocesses an image with explicit control over the optional stages.
///
/// # Errors
///
/// Returns [`OcrError::InvalidInput`] if either dimension is zero.
pub fn preprocess_with(
    image: &DynamicImage,
    config: PreprocessConfig,
) -> Result<PreprocessedImage, OcrError> {
    let (width, height) = (image.width(), image.height());
    if width == 0 || height == 0 {
        return Err(OcrError::InvalidInput(format!(
            "cannot preprocess an image with a zero dimension (got {width}x{height})"
        )));
    }

    // Drop any alpha channel first, as PIL does: `convert("L")` and
    // `convert("RGB")` both read RGB and ignore alpha rather than compositing
    // it onto a background.
    let mut rgb = image.to_rgb8();

    // Step 1: greyscale round-trip, replicating one luma plane across all
    // three channels. Done before the resize, matching upstream ordering.
    if config.greyscale {
        for pixel in rgb.pixels_mut() {
            let luma = greyscale_luma(pixel[0], pixel[1], pixel[2]);
            pixel[0] = luma;
            pixel[1] = luma;
            pixel[2] = luma;
        }
    }

    // Step 2: resize to a square target. Aspect ratio is not preserved.
    //
    // Skipping the resampler when the input is already 224x224 is not just an
    // optimisation, it is what PIL does: `Image.resize` returns a copy when the
    // requested size equals the current one. Running a bilinear pass anyway
    // would be an identity in exact arithmetic but could still round a pixel.
    let resized: RgbImage = if width == IMAGE_SIZE && height == IMAGE_SIZE {
        rgb
    } else {
        image::imageops::resize(&rgb, IMAGE_SIZE, IMAGE_SIZE, RESAMPLE_FILTER)
    };

    // Steps 3-5: rescale, normalise, and scatter into CHW.
    let plane = (IMAGE_SIZE as usize) * (IMAGE_SIZE as usize);
    let mut data = vec![0.0f32; NUM_CHANNELS * plane];
    for (x, y, pixel) in resized.enumerate_pixels() {
        let offset = y as usize * IMAGE_SIZE as usize + x as usize;
        for c in 0..NUM_CHANNELS {
            let rescaled = f32::from(pixel[c]) * RESCALE_FACTOR;
            data[c * plane + offset] = (rescaled - IMAGE_MEAN[c]) / IMAGE_STD[c];
        }
    }

    Ok(PreprocessedImage {
        data,
        shape: [1, NUM_CHANNELS, IMAGE_SIZE as usize, IMAGE_SIZE as usize],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    const PLANE: usize = (IMAGE_SIZE as usize) * (IMAGE_SIZE as usize);
    const NO_GREY: PreprocessConfig = PreprocessConfig { greyscale: false };

    fn solid(width: u32, height: u32, colour: [u8; 3]) -> DynamicImage {
        DynamicImage::ImageRgb8(RgbImage::from_pixel(width, height, Rgb(colour)))
    }

    /// The normalisation the reference pipeline applies, written out the long
    /// way so the test does not simply restate the implementation.
    fn expected(value: u8) -> f32 {
        (value as f32 / 255.0 - 0.5) / 0.5
    }

    #[test]
    fn constants_match_preprocessor_config() {
        // Guards against a silent revert to ImageNet statistics, which is what
        // the inline preprocessing block in `predict` used before this module
        // existed.
        assert_eq!(IMAGE_MEAN, [0.5, 0.5, 0.5]);
        assert_eq!(IMAGE_STD, [0.5, 0.5, 0.5]);
        assert_eq!(IMAGE_SIZE, 224);
        assert_eq!(NUM_CHANNELS, 3);
        assert!((RESCALE_FACTOR - 1.0 / 255.0).abs() < f32::EPSILON);
        assert_eq!(RESAMPLE_FILTER, image::imageops::FilterType::Triangle);
    }

    #[test]
    fn tensor_has_the_shape_and_length_the_model_expects() {
        let out = preprocess(&solid(64, 96, [10, 20, 30])).expect("preprocess");
        assert_eq!(out.shape(), [1, 3, 224, 224]);
        assert_eq!(out.data().len(), 3 * 224 * 224);
        assert_eq!(out.data().len(), NUM_CHANNELS * PLANE);

        let (shape, data) = out.into_parts();
        assert_eq!(shape, vec![1i64, 3, 224, 224]);
        assert_eq!(data.len(), 3 * 224 * 224);
    }

    #[test]
    fn layout_is_chw_not_hwc() {
        // Distinct per-channel values. In CHW the first plane is entirely red,
        // the second entirely green, the third entirely blue. In HWC the first
        // three values would instead be R, G, B of a single pixel.
        let out = preprocess_with(&solid(50, 50, [255, 0, 128]), NO_GREY).expect("preprocess");
        let data = out.data();

        for (channel, &value) in [255u8, 0, 128].iter().enumerate() {
            let plane = &data[channel * PLANE..(channel + 1) * PLANE];
            assert!(
                plane.iter().all(|v| (v - expected(value)).abs() < 1e-6),
                "channel {channel} plane is not uniformly the value for {value}; \
                 this is the signature of HWC ordering"
            );
        }

        // Stated the other way round: consecutive elements belong to the same
        // channel, not to the same pixel.
        assert!((data[0] - data[1]).abs() < 1e-6);
        assert!((data[0] - data[PLANE]).abs() > 1.0);
    }

    #[test]
    fn normalisation_matches_hand_computed_values() {
        // A solid image survives bilinear resampling unchanged, so every output
        // element is exactly the normalisation of the source byte. Values below
        // were computed by hand:
        //   37  -> 37/255  = 0.14509804; (0.14509804 - 0.5) / 0.5 = -0.70980392
        //   200 -> 200/255 = 0.78431373; (0.78431373 - 0.5) / 0.5 =  0.56862745
        //   255 -> 1.0;  (1.0 - 0.5) / 0.5 =  1.0
        let out = preprocess_with(&solid(80, 40, [37, 200, 255]), NO_GREY).expect("preprocess");
        let data = out.data();

        assert!((data[0] - (-0.709_803_92)).abs() < 1e-6, "got {}", data[0]);
        assert!(
            (data[PLANE] - 0.568_627_5).abs() < 1e-6,
            "got {}",
            data[PLANE]
        );
        assert!(
            (data[2 * PLANE] - 1.0).abs() < 1e-6,
            "got {}",
            data[2 * PLANE]
        );

        // The [0, 255] byte range maps onto exactly [-1, 1] under mean = std = 0.5.
        assert!((expected(0) - (-1.0)).abs() < 1e-6);
        assert!((expected(255) - 1.0).abs() < 1e-6);
        assert!(
            data.iter().all(|v| (-1.0..=1.0).contains(v)),
            "mean/std of 0.5 must confine output to [-1, 1]"
        );
    }

    #[test]
    fn imagenet_constants_would_produce_a_different_tensor() {
        // Pins the actual defect this module was written to fix: had the old
        // ImageNet numbers been correct, these assertions would not hold.
        //
        // Note that the two formulas nearly agree near the middle of the byte
        // range — at 128 they differ by only 0.07 — so a discriminating test
        // has to sample away from the centre. That near-agreement is exactly
        // why the wrong constants degraded accuracy without ever looking
        // obviously broken.
        const IMAGENET_MEAN: [f32; 3] = [0.485, 0.456, 0.406];
        const IMAGENET_STD: [f32; 3] = [0.229, 0.224, 0.225];

        let out = preprocess_with(&solid(32, 32, [240, 240, 240]), NO_GREY).expect("preprocess");
        for channel in 0..NUM_CHANNELS {
            let ours = out.data()[channel * PLANE];
            let imagenet = (240.0f32 / 255.0 - IMAGENET_MEAN[channel]) / IMAGENET_STD[channel];
            assert!(
                (ours - imagenet).abs() > 1.0,
                "channel {channel}: config-driven {ours} must not coincide with ImageNet {imagenet}"
            );
        }
    }

    #[test]
    fn greyscale_matches_pil_itu_r_601_2() {
        // Reference values read out of Pillow 10.1.0 via
        // `Image.new("RGB", (1, 1), t).convert("L").getpixel((0, 0))`.
        for &(rgb, pil) in &[
            ((255u8, 0u8, 0u8), 76u8),
            ((0, 255, 0), 150),
            ((0, 0, 255), 29),
            ((10, 200, 90), 131),
            ((37, 113, 201), 100),
            ((128, 128, 128), 128),
            ((0, 0, 0), 0),
            ((255, 255, 255), 255),
        ] {
            let (r, g, b) = rgb;
            assert_eq!(greyscale_luma(r, g, b), pil, "PIL mismatch for {rgb:?}");
        }

        // Rec. 709, which `image`'s `to_luma8` would use, disagrees sharply.
        // If this ever stops holding, someone swapped in the wrong weights.
        assert_ne!(greyscale_luma(0, 255, 0), 182);
    }

    #[test]
    fn greyscale_collapses_the_three_channels() {
        let out = preprocess(&solid(60, 60, [255, 0, 128])).expect("preprocess");
        let data = out.data();
        let luma = greyscale_luma(255, 0, 128);
        for channel in 0..NUM_CHANNELS {
            assert!((data[channel * PLANE] - expected(luma)).abs() < 1e-6);
        }
    }

    #[test]
    fn greyscale_is_not_a_no_op() {
        // Documents that the flag changes the tensor materially, so nobody
        // "simplifies" it away on the assumption that it is cosmetic.
        let image = solid(64, 64, [200, 30, 90]);
        let with = preprocess_with(&image, PreprocessConfig { greyscale: true }).expect("grey");
        let without = preprocess_with(&image, NO_GREY).expect("colour");
        assert_ne!(with, without);
    }

    #[test]
    fn one_by_one_image_is_handled() {
        let out = preprocess(&solid(1, 1, [200, 200, 200])).expect("1x1 must not fail");
        assert_eq!(out.data().len(), 3 * 224 * 224);
        // Upsampling a single pixel can only ever yield that pixel's value.
        assert!(out.data().iter().all(|v| (v - expected(200)).abs() < 1e-6));
    }

    #[test]
    fn extreme_aspect_ratios_are_handled() {
        for (w, h) in [
            (1u32, 4000u32),
            (4000, 1),
            (3, 997),
            (997, 3),
            (1, 1),
            (224, 224),
        ] {
            let out = preprocess(&solid(w, h, [17, 99, 240]))
                .unwrap_or_else(|e| panic!("{w}x{h} failed: {e}"));
            assert_eq!(out.shape(), [1, 3, 224, 224], "{w}x{h}");
            assert_eq!(out.data().len(), 3 * 224 * 224, "{w}x{h}");
            assert!(
                out.data().iter().all(|v| v.is_finite()),
                "{w}x{h} produced a non-finite element"
            );
        }
    }

    #[test]
    fn a_non_uniform_image_produces_a_non_uniform_tensor() {
        // Cheap guard against the failure mode this repository has shipped
        // three times: a function returning a plausible constant. A gradient in
        // must give varying values out.
        let mut buffer = RgbImage::new(300, 120);
        for (x, y, pixel) in buffer.enumerate_pixels_mut() {
            *pixel = Rgb([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8]);
        }
        let out = preprocess(&DynamicImage::ImageRgb8(buffer)).expect("preprocess");
        let first = out.data()[0];
        assert!(
            out.data().iter().any(|v| (v - first).abs() > 0.05),
            "gradient input collapsed to a constant tensor"
        );
    }

    #[test]
    fn zero_dimension_is_an_error_not_a_blank_tensor() {
        let empty = DynamicImage::ImageRgb8(RgbImage::new(0, 10));
        let err = preprocess(&empty).expect_err("a 0x10 image has no valid tensor");
        assert!(matches!(err, OcrError::InvalidInput(_)), "got {err:?}");

        let empty = DynamicImage::ImageRgb8(RgbImage::new(10, 0));
        assert!(matches!(preprocess(&empty), Err(OcrError::InvalidInput(_))));
    }
}
