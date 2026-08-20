use crate::post_process::post_process_with_furigana;

/// Japanese language post-processing profile.
/// Normalizes Japanese text, fullwidth character conversion, and Furigana extraction.
pub fn post_process_jp(input: &str, extract_furigana: bool) -> String {
    post_process_with_furigana(input, extract_furigana)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_post_process_jp_delegation() {
        let text = "漢字(かんじ)";
        let result = post_process_jp(text, true);
        assert_eq!(result, "漢字[かんじ]");
    }
}
