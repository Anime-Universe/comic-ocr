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
/// Primary Axis: Right-to-Left (x descending)
/// Secondary Axis: Top-to-Bottom (y ascending)
pub fn sort_bubble_reading_order(bubbles: &mut [BoundingBox]) {
    bubbles.sort_by(|a, b| {
        // Right-to-Left priority if vertical columns differ significantly
        let y_diff = (a.y as i32 - b.y as i32).abs();
        if y_diff > 40 {
            a.y.cmp(&b.y)
        } else {
            b.x.cmp(&a.x)
        }
    });
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
