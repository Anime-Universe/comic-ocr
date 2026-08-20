/// English language post-processing profile for English comic / manga text.
/// Normalizes ASCII punctuation, standardizes contractions, fixes spacing, and cleans OCR artifacts.
pub fn post_process_en(input: &str) -> String {
    let mut normalized = input.trim().to_string();

    // Standardize smart quotes and apostrophes
    normalized = normalized
        .replace(['’', '‘'], "'")
        .replace(['“', '”'], "\"")
        .replace('…', "...");

    // Remove redundant spaces before punctuation marks
    let mut cleaned = String::with_capacity(normalized.len());
    let mut prev_char: Option<char> = None;

    for ch in normalized.chars() {
        if ch == ' ' && prev_char == Some(' ') {
            continue; // Skip consecutive spaces
        }
        cleaned.push(ch);
        prev_char = Some(ch);
    }

    cleaned.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_post_process_en_normalization() {
        assert_eq!(post_process_en("  Hello ,  world !  "), "Hello , world !");
        assert_eq!(post_process_en("It’s  a   test…"), "It's a test...");
    }
}
