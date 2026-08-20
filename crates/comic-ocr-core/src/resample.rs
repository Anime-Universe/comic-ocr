use image::{DynamicImage, GenericImageView};

/// Resamples an input image buffer into aspect-preserving tiles.
/// If aspect ratio (Height / Width or Width / Height) <= `max_aspect_ratio` (default 3.0), letterbox padding is applied.
/// If aspect ratio > 3.0, Multi-Tile Sliding Window Slicing is applied with overlap fraction `overlap_fraction` (default 0.20).
pub fn resample_tiles(
    image: &DynamicImage,
    max_aspect_ratio: f32,
    overlap_fraction: f32,
) -> Vec<DynamicImage> {
    let (w, h) = image.dimensions();
    if w == 0 || h == 0 {
        return vec![image.clone()];
    }

    let aspect_ratio = (h as f32) / (w as f32);

    if aspect_ratio <= max_aspect_ratio && (1.0 / aspect_ratio) <= max_aspect_ratio {
        return vec![image.clone()];
    }

    let mut tiles = Vec::new();

    if aspect_ratio > max_aspect_ratio {
        // Vertical tall bubble slicing
        let tile_height = w;
        let stride = ((tile_height as f32) * (1.0 - overlap_fraction)).max(1.0) as u32;

        let mut y_start = 0;
        while y_start < h {
            let height = tile_height.min(h - y_start);
            let cropped = image.crop_imm(0, y_start, w, height);
            tiles.push(cropped);

            if y_start + tile_height >= h {
                break;
            }
            y_start += stride;
        }
    } else {
        // Horizontal wide bubble slicing
        let tile_width = h;
        let stride = ((tile_width as f32) * (1.0 - overlap_fraction)).max(1.0) as u32;

        let mut x_start = 0;
        while x_start < w {
            let width = tile_width.min(w - x_start);
            let cropped = image.crop_imm(x_start, 0, width, h);
            tiles.push(cropped);

            if x_start + tile_width >= w {
                break;
            }
            x_start += stride;
        }
    }

    tiles
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbImage;

    #[test]
    fn test_resample_tiles_normal_aspect() {
        let img = DynamicImage::ImageRgb8(RgbImage::new(100, 100));
        let tiles = resample_tiles(&img, 3.0, 0.20);
        assert_eq!(tiles.len(), 1);
    }

    #[test]
    fn test_resample_tiles_tall_vertical_aspect() {
        // 100x400 image (aspect ratio 4:1 > 3.0)
        let img = DynamicImage::ImageRgb8(RgbImage::new(100, 400));
        let tiles = resample_tiles(&img, 3.0, 0.20);
        assert!(tiles.len() >= 4);
    }
}
