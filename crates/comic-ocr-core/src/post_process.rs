/// Japanese text normalization post-processing with optional Furigana bracket parsing.
/// Replaces ellipsis variants and converts ASCII digits/letters to Japanese full-width (jaconv h2z equivalent).
pub fn post_process(input: &str) -> String {
    post_process_with_furigana(input, false)
}

/// Post-processing function with 4-state Furigana bracket syntax parser.
/// When `extract_furigana` is true, formats readings into `漢[かん]字[じ]` format.
pub fn post_process_with_furigana(input: &str, extract_furigana: bool) -> String {
    let normalized = input.replace('…', "...");
    let mut result = String::with_capacity(normalized.len());

    if !extract_furigana {
        for ch in normalized.chars() {
            match ch {
                '!'..='~' => {
                    let fullwidth_code = (ch as u32) + 0xFEE0;
                    if let Some(fw_char) = char::from_u32(fullwidth_code) {
                        result.push(fw_char);
                    } else {
                        result.push(ch);
                    }
                }
                ' ' => result.push('　'),
                _ => result.push(ch),
            }
        }
        return result.trim().to_string();
    }

    // 4-State Furigana Bracket Parser FSM
    // States: 0 = BaseText, 1 = KanjiDetected, 2 = FuriganaReading, 3 = EmitFormatted
    enum ParserState {
        BaseText,
        KanjiDetected(char),
        FuriganaReading(char, String),
    }

    let mut state = ParserState::BaseText;

    for ch in normalized.chars() {
        match state {
            ParserState::BaseText => {
                if is_kanji(ch) {
                    state = ParserState::KanjiDetected(ch);
                } else {
                    push_normalized_char(&mut result, ch);
                }
            }
            ParserState::KanjiDetected(kanji) => {
                if ch == '(' || ch == '（' || ch == '[' || ch == '【' {
                    state = ParserState::FuriganaReading(kanji, String::new());
                } else if is_kanji(ch) {
                    push_normalized_char(&mut result, kanji);
                    state = ParserState::KanjiDetected(ch);
                } else {
                    push_normalized_char(&mut result, kanji);
                    push_normalized_char(&mut result, ch);
                    state = ParserState::BaseText;
                }
            }
            ParserState::FuriganaReading(kanji, mut reading) => {
                if ch == ')' || ch == '）' || ch == ']' || ch == '】' {
                    result.push(kanji);
                    result.push('[');
                    result.push_str(&reading);
                    result.push(']');
                    state = ParserState::BaseText;
                } else {
                    reading.push(ch);
                    state = ParserState::FuriganaReading(kanji, reading);
                }
            }
        }
    }

    // Flush remaining state
    match state {
        ParserState::KanjiDetected(kanji) => push_normalized_char(&mut result, kanji),
        ParserState::FuriganaReading(kanji, reading) => {
            push_normalized_char(&mut result, kanji);
            result.push('(');
            result.push_str(&reading);
        }
        ParserState::BaseText => {}
    }

    result.trim().to_string()
}

fn is_kanji(ch: char) -> bool {
    // CJK Unified Ideographs range (0x4E00..=0x9FFF)
    matches!(ch, '\u{4E00}'..='\u{9FFF}')
}

fn push_normalized_char(result: &mut String, ch: char) {
    match ch {
        '!'..='~' => {
            let fullwidth_code = (ch as u32) + 0xFEE0;
            if let Some(fw_char) = char::from_u32(fullwidth_code) {
                result.push(fw_char);
            } else {
                result.push(ch);
            }
        }
        ' ' => result.push('　'),
        _ => result.push(ch),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_post_process_ellipsis_and_fullwidth() {
        assert_eq!(post_process("…"), "．．．");
        assert_eq!(post_process("テスト 123"), "テスト　１２３");
        assert_eq!(post_process("Hello!"), "Ｈｅｌｌｏ！");
    }

    #[test]
    fn test_post_process_furigana_bracket_fsm() {
        assert_eq!(
            post_process_with_furigana("漢字(かんじ)", true),
            "漢字[かんじ]"
        );
        assert_eq!(
            post_process_with_furigana("漢（かん）字（じ）", true),
            "漢[かん]字[じ]"
        );
    }
}
