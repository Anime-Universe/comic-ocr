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

/// Which way a page reads horizontally.
///
/// Shared with `validation`, deliberately. The two used to disagree: the sorter
/// hardcoded right-to-left while the validator took the direction as a string
/// and only enforced its rule when that string said RTL. On a left-to-right page
/// the sorter therefore emitted reversed reading order AND the validator's only
/// branch never fired — wrong output with a clean bill of health. One type, used
/// by both, is what makes that state unrepresentable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadingDirection {
    /// Japanese manga: rightmost first within a row.
    RightToLeft,
    /// Western comics: leftmost first within a row.
    LeftToRight,
}

impl ReadingDirection {
    /// Parse a declared direction. Accepts the spellings that actually occur in
    /// the corpus and in the iPub schema, case-insensitively.
    ///
    /// Returns `None` for anything unrecognised rather than defaulting. A
    /// silent default is how `"RTL"` in the wrong case, or `"vertical-rl"`,
    /// ends up reading as left-to-right with nothing said about it.
    pub fn parse(declared: &str) -> Option<Self> {
        match declared
            .trim()
            .to_ascii_lowercase()
            .replace('_', "-")
            .as_str()
        {
            "rtl" | "right-to-left" | "vertical-rl" => Some(Self::RightToLeft),
            "ltr" | "left-to-right" | "horizontal-tb" => Some(Self::LeftToRight),
            _ => None,
        }
    }
}

/// Sorts speech bubble bounding boxes into reading order.
/// Primary axis: top-to-bottom (row bucket clustering).
/// Secondary axis: within a row, per `direction`.
pub fn sort_bubble_reading_order(bubbles: &mut [BoundingBox], direction: ReadingDirection) {
    if bubbles.len() <= 1 {
        return;
    }

    // Sort by y ascending first to establish candidate vertical sequence
    bubbles.sort_by_key(|b| b.y);

    // Group into transitive row buckets where adjacent items overlap within average height / 40px
    let mut rows: Vec<Vec<BoundingBox>> = Vec::new();

    for box_item in bubbles.iter() {
        if let Some(current_row) = rows.last_mut() {
            let row_y_avg =
                current_row.iter().map(|b| b.y as f64).sum::<f64>() / current_row.len() as f64;
            let threshold =
                current_row.iter().map(|b| b.height as f64).sum::<f64>() / current_row.len() as f64;
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
        match direction {
            ReadingDirection::RightToLeft => row.sort_by_key(|b| std::cmp::Reverse(b.x)),
            ReadingDirection::LeftToRight => row.sort_by_key(|b| b.x),
        }
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

        sort_bubble_reading_order(&mut bubbles, ReadingDirection::RightToLeft);

        // Expect right-most top bubble first (x=300, y=10)
        assert_eq!(bubbles[0].x, 300);
        assert_eq!(bubbles[1].x, 100);
        assert_eq!(bubbles[2].x, 200);
    }

    /// The case that used to be silently wrong: an LTR page sorted RTL, with the
    /// validator's only branch not firing, so nothing reported it.
    #[test]
    fn ltr_sorts_leftmost_first_within_a_row() {
        let mut bubbles = vec![
            BoundingBox::new(500, 10, 80, 40),
            BoundingBox::new(100, 12, 80, 40),
            BoundingBox::new(300, 11, 80, 40),
        ];
        sort_bubble_reading_order(&mut bubbles, ReadingDirection::LeftToRight);
        assert_eq!(
            bubbles.iter().map(|b| b.x).collect::<Vec<_>>(),
            vec![100, 300, 500]
        );
    }

    #[test]
    fn rtl_and_ltr_are_mirror_images_of_each_other() {
        let make = || {
            vec![
                BoundingBox::new(500, 10, 80, 40),
                BoundingBox::new(100, 12, 80, 40),
                BoundingBox::new(300, 11, 80, 40),
            ]
        };
        let (mut rtl, mut ltr) = (make(), make());
        sort_bubble_reading_order(&mut rtl, ReadingDirection::RightToLeft);
        sort_bubble_reading_order(&mut ltr, ReadingDirection::LeftToRight);
        let rtl_x: Vec<_> = rtl.iter().map(|b| b.x).collect();
        let mut ltr_x: Vec<_> = ltr.iter().map(|b| b.x).collect();
        ltr_x.reverse();
        assert_eq!(rtl_x, ltr_x);
    }

    /// An unrecognised direction must not silently become one of them.
    #[test]
    fn parse_refuses_what_it_does_not_recognise() {
        assert_eq!(
            ReadingDirection::parse("rtl"),
            Some(ReadingDirection::RightToLeft)
        );
        assert_eq!(
            ReadingDirection::parse("RTL"),
            Some(ReadingDirection::RightToLeft)
        );
        assert_eq!(
            ReadingDirection::parse("right_to_left"),
            Some(ReadingDirection::RightToLeft)
        );
        assert_eq!(
            ReadingDirection::parse("vertical-rl"),
            Some(ReadingDirection::RightToLeft)
        );
        assert_eq!(
            ReadingDirection::parse("ltr"),
            Some(ReadingDirection::LeftToRight)
        );
        assert_eq!(
            ReadingDirection::parse("left-to-right"),
            Some(ReadingDirection::LeftToRight)
        );
        assert_eq!(ReadingDirection::parse("sideways"), None);
        assert_eq!(ReadingDirection::parse(""), None);
    }
}
