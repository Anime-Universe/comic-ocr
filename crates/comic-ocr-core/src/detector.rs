use crate::scene_graph::Rect;
use image::{DynamicImage, GenericImageView};

/// Automatic Text & Speech Bubble Region Bounding Box Detector.
/// Segments raw full-page images or color covers into individual text crops
/// sorted in Japanese Reading Order (Right-to-Left, Top-to-Bottom).
pub struct TextDetector;

impl TextDetector {
    /// Detect text region bounding boxes in a dynamic image.
    /// Returns bounding boxes in pixel coordinates sorted by reading order.
    pub fn detect_regions(image: &DynamicImage) -> Vec<Rect> {
        let (width, height) = image.dimensions();
        if width == 0 || height == 0 {
            return vec![];
        }

        let total_area = (width * height) as f64;
        let luma_img = image.to_luma8();

        // Compute Otsu / Adaptive threshold binarization
        let mut text_mask = vec![false; (width * height) as usize];
        let mut sum_luma = 0u64;

        for pixel in luma_img.pixels() {
            sum_luma += pixel.0[0] as u64;
        }

        let avg_luma = (sum_luma / (width * height) as u64) as u8;

        // Dark text on bright background OR bright text on dark background
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;
                let val = luma_img.get_pixel(x, y).0[0];
                if val < avg_luma.saturating_sub(25) || val > avg_luma.saturating_add(60) {
                    text_mask[idx] = true;
                }
            }
        }

        // Horizontal & Vertical Connected Component Dilation Grid
        let grid_w = 32;
        let grid_h = 32;
        let cell_w = (width as usize).div_ceil(grid_w);
        let cell_h = (height as usize).div_ceil(grid_h);

        let mut active_cells = vec![false; grid_w * grid_h];

        for gy in 0..grid_h {
            for gx in 0..grid_w {
                let x_start = gx * cell_w;
                let x_end = ((gx + 1) * cell_w).min(width as usize);
                let y_start = gy * cell_h;
                let y_end = ((gy + 1) * cell_h).min(height as usize);

                if x_start < x_end && y_start < y_end {
                    let mut active_count = 0;
                    for y in y_start..y_end {
                        for x in x_start..x_end {
                            let idx = y * width as usize + x;
                            if text_mask[idx] {
                                active_count += 1;
                            }
                        }
                    }

                    let cell_area = (x_end - x_start) * (y_end - y_start);
                    if cell_area > 0 && (active_count as f64 / cell_area as f64) > 0.08 {
                        active_cells[gy * grid_w + gx] = true;
                    }
                }
            }
        }

        // Cluster adjacent active cells into region bounding boxes
        let mut visited = vec![false; grid_w * grid_h];
        let mut rects = Vec::new();

        for gy in 0..grid_h {
            for gx in 0..grid_w {
                let cell_idx = gy * grid_w + gx;
                if active_cells[cell_idx] && !visited[cell_idx] {
                    // Flood fill to collect cluster
                    let mut min_gx = gx;
                    let mut max_gx = gx;
                    let mut min_gy = gy;
                    let mut max_gy = gy;

                    let mut queue = vec![(gx, gy)];
                    visited[cell_idx] = true;

                    while let Some((cx, cy)) = queue.pop() {
                        min_gx = min_gx.min(cx);
                        max_gx = max_gx.max(cx);
                        min_gy = min_gy.min(cy);
                        max_gy = max_gy.max(cy);

                        // Check 8-neighbors safely
                        for dy in -1i32..=1i32 {
                            for dx in -1i32..=1i32 {
                                let nx = cx as i32 + dx;
                                let ny = cy as i32 + dy;
                                if nx >= 0 && nx < grid_w as i32 && ny >= 0 && ny < grid_h as i32 {
                                    let n_idx = (ny as usize) * grid_w + (nx as usize);
                                    if active_cells[n_idx] && !visited[n_idx] {
                                        visited[n_idx] = true;
                                        queue.push((nx as usize, ny as usize));
                                    }
                                }
                            }
                        }
                    }

                    let px_x = (min_gx * cell_w) as f64;
                    let px_y = (min_gy * cell_h) as f64;
                    let px_w =
                        (((max_gx + 1) * cell_w).min(width as usize) - min_gx * cell_w) as f64;
                    let px_h =
                        (((max_gy + 1) * cell_h).min(height as usize) - min_gy * cell_h) as f64;
                    let area = px_w * px_h;

                    // Filter out full-page container or tiny noise
                    if area > 400.0 && area < 0.85 * total_area {
                        rects.push(Rect {
                            x: px_x,
                            y: px_y,
                            width: px_w,
                            height: px_h,
                        });
                    }
                }
            }
        }

        // Sort in Japanese Reading Order: Right-to-Left primary, Top-to-Bottom secondary
        rects.sort_by(|a, b| {
            let row_a = (a.y / 200.0) as i32;
            let row_b = (b.y / 200.0) as i32;
            if row_a != row_b {
                row_a.cmp(&row_b)
            } else {
                // Right to left (higher X first)
                b.x.partial_cmp(&a.x).unwrap_or(std::cmp::Ordering::Equal)
            }
        });

        // Fallback: If no sub-regions detected, return single full-image bounding box
        if rects.is_empty() {
            vec![Rect {
                x: 0.0,
                y: 0.0,
                width: width as f64,
                height: height as f64,
            }]
        } else {
            rects
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbImage;

    #[test]
    fn test_text_detector_fallback() {
        let img = DynamicImage::ImageRgb8(RgbImage::new(100, 100));
        let regions = TextDetector::detect_regions(&img);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].width, 100.0);
    }
}
