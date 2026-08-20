/// English language post-processing profile for English comic / manga text.
/// Normalizes ASCII punctuation, standardizes contractions, removes spaces before punctuation, and cleans OCR artifacts.
pub fn post_process_en(input: &str) -> String {
    let mut normalized = input.trim().to_string();

    // Standardize smart quotes and apostrophes
    normalized = normalized
        .replace(['’', '‘'], "'")
        .replace(['“', '”'], "\"")
        .replace('…', "...");

    // Remove space before punctuation marks: , . ! ? ; :
    let mut cleaned = String::with_capacity(normalized.len());
    let mut chars = normalized.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == ' ' {
            // If space is followed by punctuation or another space, skip it
            if let Some(&next_ch) = chars.peek() {
                if matches!(next_ch, ',' | '.' | '!' | '?' | ';' | ':' | ' ') {
                    continue;
                }
            }
        }
        cleaned.push(ch);
    }

    // Standardize common un-punctuated contractions
    let words: Vec<&str> = cleaned.split_whitespace().collect();
    let mut processed_words = Vec::with_capacity(words.len());

    for word in words {
        let replacement = match word.to_lowercase().as_str() {
            "im" => "I'm",
            "dont" => "don't",
            "cant" => "can't",
            "wont" => "won't",
            "youre" => "you're",
            "theyre" => "they're",
            "hes" => "he's",
            "shes" => "she's",
            "its" if !word.ends_with('.') => "it's",
            _ => word,
        };
        processed_words.push(replacement);
    }

    processed_words.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_post_process_en_normalization() {
        assert_eq!(post_process_en("  Hello ,  world !  "), "Hello, world!");
        assert_eq!(post_process_en("It’s  a   test…"), "It's a test...");
        assert_eq!(post_process_en("im sure you dont know"), "I'm sure you don't know");
    }
}
