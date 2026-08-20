//! Build a character vocabulary from a corpus we own.
//!
//! The exported ONNX graphs carry no `vocab.txt`, so the generator can run and
//! its output cannot be read — a decoder emits token ids and nothing maps them
//! back to characters. This module closes that gap from the other direction:
//! rather than shipping someone else's 6,144-token vocabulary, derive one from
//! the transcriptions in our own corpus.
//!
//! That matters beyond convenience. A vocabulary is a structural dependency —
//! ids are positions, so two models with different vocabularies cannot share a
//! checkpoint. Deriving ours removes the last thing tying this repository's
//! output to a reference model, and sizes the vocabulary to what the corpus
//! actually contains instead of what someone else's corpus did.
//!
//! Character-level, not WordPiece subwords. Japanese comic text is
//! predominantly kana and kanji where a character IS the unit; English comic
//! lettering is near-universally uppercase and short. Sub-word merges buy little
//! here and cost a training-time tokenizer we would then have to reimplement in
//! Rust exactly, which is the class of bug the differential tokenizer tests
//! exist to catch.

use std::collections::BTreeMap;

/// Special tokens, in the order BERT-family checkpoints place them.
///
/// Order is not cosmetic: `[PAD]` at id 0 means a padded position decodes to
/// padding rather than to a real character, which is the difference between a
/// short reading and a reading with garbage appended.
pub const SPECIAL_TOKENS: [&str; 5] = ["[PAD]", "[UNK]", "[CLS]", "[SEP]", "[MASK]"];

/// How a vocabulary was derived, so a checkpoint can say what it was trained
/// against rather than leaving the reader to guess.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VocabReport {
    /// Distinct characters observed across the corpus.
    pub distinct_characters: usize,
    /// Characters admitted after the frequency floor.
    pub admitted: usize,
    /// Characters seen but dropped for being rarer than the floor.
    ///
    /// Counted, never silently discarded: a vocabulary that shrank because the
    /// floor was wrong looks identical to one built from a small corpus.
    pub dropped_rare: usize,
    /// Total characters scanned, including repeats.
    pub characters_scanned: usize,
    /// Lines that contributed nothing — empty or whitespace only.
    pub empty_lines: usize,
}

/// Build a `vocab.txt` body from transcriptions.
///
/// `min_frequency` is a floor, not a cap: a character seen fewer times than this
/// is dropped, because a glyph observed once is more likely a transcription
/// error than a character the model must learn. Dropped characters are counted
/// and reported — they decode as `[UNK]`, which is a real token and an honest
/// answer, not a silent substitution.
///
/// Returns the file body and a report of what was admitted and refused.
pub fn build_character_vocab(
    transcriptions: impl IntoIterator<Item = impl AsRef<str>>,
    min_frequency: usize,
) -> (String, VocabReport) {
    // BTreeMap, not HashMap: the vocabulary must be byte-identical across runs
    // on the same corpus. Ids are positions, so a vocabulary that reorders
    // between builds silently invalidates every checkpoint trained against the
    // previous one — and nothing would report it, because both files look fine.
    let mut counts: BTreeMap<char, usize> = BTreeMap::new();
    let mut scanned = 0usize;
    let mut empty_lines = 0usize;

    for line in transcriptions {
        let line = line.as_ref();
        if line.trim().is_empty() {
            empty_lines += 1;
            continue;
        }
        for ch in line.chars() {
            // A newline inside a transcription is layout, not content: the model
            // emits a reading, and the caller decides how it is laid out.
            if ch == '\n' || ch == '\r' {
                continue;
            }
            scanned += 1;
            *counts.entry(ch).or_insert(0) += 1;
        }
    }

    let distinct = counts.len();
    let mut admitted: Vec<char> = counts
        .iter()
        .filter(|(_, n)| **n >= min_frequency)
        .map(|(ch, _)| *ch)
        .collect();
    admitted.sort_unstable();

    let mut body = String::new();
    for token in SPECIAL_TOKENS {
        body.push_str(token);
        body.push('\n');
    }
    for ch in &admitted {
        body.push(*ch);
        body.push('\n');
    }

    let report = VocabReport {
        distinct_characters: distinct,
        admitted: admitted.len(),
        dropped_rare: distinct - admitted.len(),
        characters_scanned: scanned,
        empty_lines,
    };
    (body, report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::WordPieceVocab;

    /// The whole point: what this builds must load.
    #[test]
    fn the_built_vocabulary_loads_in_the_tokenizer_that_will_use_it() {
        let (body, _) = build_character_vocab(["こんにちは", "HELLO"], 1);
        let vocab = WordPieceVocab::from_vocab_txt(&body)
            .expect("a vocabulary this module builds must load in the tokenizer");
        // Round-trip through the real decoder, not just a parse.
        let ids: Vec<u32> = "こんにちは"
            .chars()
            .map(|c| {
                vocab
                    .id_of(&c.to_string())
                    .expect("every admitted char has an id")
            })
            .collect();
        assert_eq!(vocab.decode(&ids, true), "こんにちは");
    }

    /// Ids are positions, so a vocabulary that reorders between builds silently
    /// invalidates every checkpoint trained against the previous one — and both
    /// files look perfectly fine.
    #[test]
    fn the_same_corpus_always_builds_the_same_bytes() {
        let corpus = ["だれ", "HELLO", "だれ", "世界"];
        let (a, _) = build_character_vocab(corpus, 1);
        let (b, _) = build_character_vocab(corpus, 1);
        assert_eq!(a, b);
        // And order does not depend on iteration order of the input.
        let (c, _) = build_character_vocab(["世界", "HELLO", "だれ", "だれ"], 1);
        assert_eq!(a, c, "the vocabulary must not depend on corpus ordering");
    }

    /// Special tokens must come first and in this order. `[PAD]` at id 0 means a
    /// padded position decodes to padding rather than to a real character.
    #[test]
    fn special_tokens_lead_and_pad_is_zero() {
        let (body, _) = build_character_vocab(["あ"], 1);
        let lines: Vec<&str> = body.lines().collect();
        assert_eq!(&lines[..5], &SPECIAL_TOKENS[..]);
        assert_eq!(lines[0], "[PAD]");
    }

    /// A dropped character is REPORTED, not silently absent. A vocabulary that
    /// shrank because the floor was wrong looks exactly like one built from a
    /// small corpus.
    #[test]
    fn rare_characters_are_counted_when_they_are_dropped() {
        let (body, report) = build_character_vocab(["ああああ", "き"], 2);
        assert_eq!(report.distinct_characters, 2);
        assert_eq!(report.admitted, 1, "only あ clears a floor of 2");
        assert_eq!(report.dropped_rare, 1);
        assert!(!body.contains('き'), "き is below the floor");
        assert!(body.contains('あ'));
    }

    /// An empty transcription is a fact about the corpus, not a character.
    #[test]
    fn blank_transcriptions_are_counted_rather_than_scanned() {
        let (_, report) = build_character_vocab(["", "   ", "あ"], 1);
        assert_eq!(report.empty_lines, 2);
        assert_eq!(report.characters_scanned, 1);
    }
}
