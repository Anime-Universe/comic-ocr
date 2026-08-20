/// Japanese language post-processing profile.
pub fn post_process_jp(input: &str, extract_furigana: bool) -> String {
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
