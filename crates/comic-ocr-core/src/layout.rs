use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BoundingBox {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl BoundingBox {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// Sorts a slice of manga speech bubble bounding boxes in Japanese Reading Order:
/// Primary Axis: Top-to-Bottom (Row Bucket Clustering)
/// Secondary Axis: Right-to-Left (x descending within row)
pub fn sort_bubble_reading_order(bubbles: &mut [BoundingBox]) {
    if bubbles.len() <= 1 {
        return;
    }

    // Sort by y ascending first to establish candidate vertical sequence
    bubbles.sort_by_key(|b| b.y);

    // Group into transitive row buckets where adjacent items overlap within average height / 40px
    let mut rows: Vec<Vec<BoundingBox>> = Vec::new();

    for box_item in bubbles.iter() {
        if let Some(current_row) = rows.last_mut() {
            let row_y_avg = current_row.iter().map(|b| b.y as f64).sum::<f64>() / current_row.len() as f64;
            let threshold = current_row.iter().map(|b| b.height as f64).sum::<f64>() / current_row.len() as f64;
            let max_delta = (threshold * 0.75).max(40.0);

            if (box_item.y as f64 - row_y_avg).abs() <= max_delta {
                current_row.push(box_item.clone());
                continue;
            }
        }
        rows.push(vec![box_item.clone()]);
    }

    // Sort within each row bucket: Right-to-Left (x descending)
    let mut idx = 0;
    for mut row in rows {
        row.sort_by(|a, b| b.x.cmp(&a.x));
        for item in row {
            bubbles[idx] = item;
            idx += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_bubble_reading_order() {
        let mut bubbles = vec![
            BoundingBox::new(100, 20, 50, 50),
            BoundingBox::new(300, 10, 50, 50),
            BoundingBox::new(200, 200, 50, 50),
        ];

        sort_bubble_reading_order(&mut bubbles);

        // Expect right-most top bubble first (x=300, y=10)
        assert_eq!(bubbles[0].x, 300);
        assert_eq!(bubbles[1].x, 100);
        assert_eq!(bubbles[2].x, 200);
    }
}
