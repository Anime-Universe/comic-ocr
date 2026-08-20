//! Pure-Rust WordPiece decoding for a BERT character vocabulary.
//!
//! A BERT WordPiece checkpoint ships `vocab.txt`, `tokenizer_config.json` and
//! `special_tokens_map.json` — and no `tokenizer.json`. That makes it a classic
//! BERT WordPiece vocabulary: one token per line, line number is the token id.
//! Decoding it needs no model file, no `tokenizer.json` graph, and no Python.
//!
//! OCR only ever decodes. The generation loop produces ids and needs them back
//! as text, so this module implements that direction only; encoding is not here
//! because nothing in this workspace needs it.
//!
//! # Why every failure is an `Err`
//!
//! This crate has shipped three defects where a function returned a plausible
//! constant instead of a computed value — see
//! `crates/comic-ocr-ort/tests/test_no_fabricated_output.rs`. So: a vocabulary
//! that cannot be loaded is an error, never a default. And an id that is not in
//! the vocabulary never becomes `[UNK]` in the output — `[UNK]` is a real token
//! the model can genuinely emit, so substituting it would put characters into a
//! transcription that the model never produced. See [`WordPieceVocab::decode`].

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::str::FromStr;

use thiserror::Error;

use crate::types::OcrError;

/// Classification token. Generation starts here; the id is read from the vocabulary, not assumed.
pub const CLS_TOKEN: &str = "[CLS]";
/// Separator token. Used as EOS by BERT-decoder checkpoints — the generation loop stops on it.
pub const SEP_TOKEN: &str = "[SEP]";
/// Padding token.
pub const PAD_TOKEN: &str = "[PAD]";
/// Unknown token. The model can emit this legitimately; it is not an error marker.
pub const UNK_TOKEN: &str = "[UNK]";
/// Mask token. Unused during OCR generation, required by the vocabulary contract.
pub const MASK_TOKEN: &str = "[MASK]";

/// The tokens `skip_special_tokens` removes, and the ones a vocabulary must contain.
const SPECIAL_TOKENS: [&str; 5] = [CLS_TOKEN, SEP_TOKEN, PAD_TOKEN, UNK_TOKEN, MASK_TOKEN];

/// The WordPiece continuation marker. A token carrying it joins the previous
/// token with no separator.
const CONTINUATION_PREFIX: &str = "##";

/// Everything that can go wrong loading or strictly decoding a vocabulary.
#[derive(Debug, Error)]
pub enum TokenizerError {
    /// The file or string held no tokens. A zero-token vocabulary would decode
    /// every id to nothing and report success, which is the exact failure shape
    /// this crate has already shipped three times.
    #[error("vocabulary is empty: no tokens found")]
    EmptyVocab,

    /// A token the decoder depends on is absent, so `skip_special_tokens` and
    /// the generation loop's EOS check would both be silently wrong.
    #[error("vocabulary is missing the required special token `{0}`")]
    MissingSpecialToken(&'static str),

    /// The vocabulary file could not be read.
    #[error("could not read vocabulary from `{path}`: {source}")]
    Io {
        /// The path that was attempted.
        path: String,
        /// The underlying I/O failure.
        source: std::io::Error,
    },

    /// [`WordPieceVocab::try_decode`] met an id with no token behind it.
    #[error("token id {id} is outside the vocabulary (size {size})")]
    IdOutOfRange {
        /// The offending id.
        id: u32,
        /// Number of tokens in the vocabulary.
        size: usize,
    },

    /// More tokens than a `u32` id can address. Truncating to fit would make
    /// every id past the cut decode to the wrong token.
    #[error("vocabulary has {0} tokens, more than a u32 token id can address")]
    VocabTooLarge(usize),
}

impl From<TokenizerError> for OcrError {
    fn from(error: TokenizerError) -> Self {
        OcrError::TokenizerError(error.to_string())
    }
}

/// A BERT WordPiece vocabulary, indexed by token id.
///
/// Construct with [`WordPieceVocab::from_vocab_txt`] (from an embedded or
/// otherwise in-memory `vocab.txt`) or [`WordPieceVocab::from_file`].
#[derive(Debug, Clone)]
pub struct WordPieceVocab {
    /// Token text by id. Position in this vector *is* the id.
    tokens: Vec<String>,
    /// Reverse lookup. First occurrence wins, matching HuggingFace.
    ids: HashMap<String, u32>,
    cls_id: u32,
    sep_id: u32,
    pad_id: u32,
    unk_id: u32,
    mask_id: u32,
}

impl WordPieceVocab {
    /// Parse the contents of a `vocab.txt`.
    ///
    /// One token per line; the line number is the token id. Both `\n` and
    /// `\r\n` line endings are accepted, and a single trailing newline is not
    /// treated as an extra token. Blank lines *inside* the file are kept as
    /// empty tokens rather than dropped, because dropping one would shift every
    /// id after it and silently corrupt every subsequent decode.
    ///
    /// # Errors
    ///
    /// [`TokenizerError::EmptyVocab`] if no tokens are present, or
    /// [`TokenizerError::MissingSpecialToken`] if any of `[CLS]`, `[SEP]`,
    /// `[PAD]`, `[UNK]`, `[MASK]` is absent.
    pub fn from_vocab_txt(contents: &str) -> Result<Self, TokenizerError> {
        let mut lines: Vec<&str> = contents.split('\n').collect();
        // A file ending in a newline yields one trailing empty element that is
        // an artefact of the terminator, not a token.
        if lines.last() == Some(&"") {
            lines.pop();
        }

        let tokens: Vec<String> = lines
            .into_iter()
            .map(|line| line.strip_suffix('\r').unwrap_or(line).to_string())
            .collect();

        if tokens.is_empty() {
            return Err(TokenizerError::EmptyVocab);
        }

        if u32::try_from(tokens.len()).is_err() {
            return Err(TokenizerError::VocabTooLarge(tokens.len()));
        }

        let mut ids: HashMap<String, u32> = HashMap::with_capacity(tokens.len());
        for (index, token) in tokens.iter().enumerate() {
            // Safe: the length was just proven to fit, so every index does.
            let id = index as u32;
            ids.entry(token.clone()).or_insert(id);
        }

        let required = |name: &'static str| -> Result<u32, TokenizerError> {
            ids.get(name)
                .copied()
                .ok_or(TokenizerError::MissingSpecialToken(name))
        };

        Ok(Self {
            cls_id: required(CLS_TOKEN)?,
            sep_id: required(SEP_TOKEN)?,
            pad_id: required(PAD_TOKEN)?,
            unk_id: required(UNK_TOKEN)?,
            mask_id: required(MASK_TOKEN)?,
            tokens,
            ids,
        })
    }

    /// Load a `vocab.txt` from disk.
    ///
    /// # Errors
    ///
    /// [`TokenizerError::Io`] if the file cannot be read, plus every error
    /// [`WordPieceVocab::from_vocab_txt`] can return.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, TokenizerError> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path).map_err(|source| TokenizerError::Io {
            path: path.display().to_string(),
            source,
        })?;
        Self::from_vocab_txt(&contents)
    }

    /// Number of tokens, which is also one past the highest valid id.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    /// Always `false` — a constructed vocabulary is never empty, because an
    /// empty one is rejected at load time. Present for the `len`/`is_empty` pair.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// The token text behind an id, or `None` if the id is out of range.
    #[must_use]
    pub fn token(&self, id: u32) -> Option<&str> {
        self.tokens.get(id as usize).map(String::as_str)
    }

    /// The id of an exact token string, or `None` if the vocabulary lacks it.
    #[must_use]
    pub fn id_of(&self, token: &str) -> Option<u32> {
        self.ids.get(token).copied()
    }

    /// Id of `[CLS]`. The generation loop seeds the decoder with this.
    #[must_use]
    pub fn cls_id(&self) -> u32 {
        self.cls_id
    }

    /// Id of `[SEP]`. This is the EOS a generation loop stops on.
    #[must_use]
    pub fn sep_id(&self) -> u32 {
        self.sep_id
    }

    /// Id of `[PAD]`.
    #[must_use]
    pub fn pad_id(&self) -> u32 {
        self.pad_id
    }

    /// Id of `[UNK]`.
    #[must_use]
    pub fn unk_id(&self) -> u32 {
        self.unk_id
    }

    /// Id of `[MASK]`.
    #[must_use]
    pub fn mask_id(&self) -> u32 {
        self.mask_id
    }

    /// Whether a token string is one of the five special tokens.
    #[must_use]
    pub fn is_special_token(token: &str) -> bool {
        SPECIAL_TOKENS.contains(&token)
    }

    /// Whether an id maps to one of the five special tokens. Out-of-range ids
    /// are not special — they are not anything.
    #[must_use]
    pub fn is_special_id(&self, id: u32) -> bool {
        self.token(id).is_some_and(Self::is_special_token)
    }

    /// Decode ids to text with **no spaces between tokens**.
    ///
    /// This matches what the Python path it replaces produced, which was
    /// `tokenizer.decode(ids, skip_special_tokens=True).replace(' ', '')`.
    /// HuggingFace's BERT decoder joins tokens with spaces and then strips the
    /// `" ##"` sequences; the remaining spaces are removed because
    /// Japanese does not use them. The result is a plain concatenation of the
    /// WordPiece-merged words, which is what this returns.
    ///
    /// **Out-of-range ids contribute nothing.** They are skipped, not rendered
    /// as `[UNK]` and not rendered as any marker. `[UNK]` is a token the model
    /// can genuinely emit, so emitting it for an id the model did *not* emit
    /// would put fabricated characters into a transcription. Use
    /// [`WordPieceVocab::try_decode`] when the caller needs to know that an id
    /// was unmappable rather than silently lose it.
    #[must_use]
    pub fn decode(&self, ids: &[u32], skip_special_tokens: bool) -> String {
        self.decode_words(ids, skip_special_tokens).concat()
    }

    /// Like [`WordPieceVocab::decode`], but fails instead of dropping an id
    /// that is not in the vocabulary.
    ///
    /// # Errors
    ///
    /// [`TokenizerError::IdOutOfRange`] on the first unmappable id.
    pub fn try_decode(
        &self,
        ids: &[u32],
        skip_special_tokens: bool,
    ) -> Result<String, TokenizerError> {
        let mut words: Vec<String> = Vec::new();
        for &id in ids {
            let token = self.token(id).ok_or(TokenizerError::IdOutOfRange {
                id,
                size: self.tokens.len(),
            })?;
            if skip_special_tokens && Self::is_special_token(token) {
                continue;
            }
            push_wordpiece(&mut words, token);
        }
        Ok(words.concat())
    }

    /// Decode ids into WordPiece-merged *words*, before they are concatenated.
    ///
    /// This is where the `##` rule is observable: a token beginning `##`
    /// continues the previous word with no separator, any other token starts a
    /// new word. [`WordPieceVocab::decode`] joins these with nothing between
    /// them, so the distinction disappears in its output — callers that need
    /// word boundaries (a space-separated language, or a diagnostic) read them
    /// here.
    ///
    /// Out-of-range ids are skipped, exactly as in
    /// [`WordPieceVocab::decode`].
    #[must_use]
    pub fn decode_words(&self, ids: &[u32], skip_special_tokens: bool) -> Vec<String> {
        let mut words: Vec<String> = Vec::new();
        for &id in ids {
            let Some(token) = self.token(id) else {
                continue;
            };
            if skip_special_tokens && Self::is_special_token(token) {
                continue;
            }
            push_wordpiece(&mut words, token);
        }
        words
    }
}

/// Apply the WordPiece continuation rule to a token.
///
/// A `##`-prefixed token appends its remainder to the word in progress. When
/// there is no word in progress — the sequence opened with a continuation, or
/// everything before it was a skipped special token — the remainder starts one,
/// which is what HuggingFace's `" ".join(tokens).replace(" ##", "")` also does.
fn push_wordpiece(words: &mut Vec<String>, token: &str) {
    match token.strip_prefix(CONTINUATION_PREFIX) {
        Some(continuation) => match words.last_mut() {
            Some(word) => word.push_str(continuation),
            None => words.push(continuation.to_string()),
        },
        None => words.push(token.to_string()),
    }
}

impl FromStr for WordPieceVocab {
    type Err = TokenizerError;

    fn from_str(contents: &str) -> Result<Self, Self::Err> {
        Self::from_vocab_txt(contents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ids 0..=4 are the special tokens, matching BERT convention; 5.. are
    /// content pieces, including `##` continuations in both scripts.
    const VOCAB: &str =
        "[PAD]\n[UNK]\n[CLS]\n[SEP]\n[MASK]\nhello\n##world\nこん\n##にちは\n猫\n##が\n好\n##き\n";

    const PAD: u32 = 0;
    const UNK: u32 = 1;
    const CLS: u32 = 2;
    const SEP: u32 = 3;
    const MASK: u32 = 4;
    const HELLO: u32 = 5;
    const WORLD: u32 = 6;
    const KON: u32 = 7;
    const NICHIWA: u32 = 8;
    const NEKO: u32 = 9;
    const GA: u32 = 10;
    const SUKI: u32 = 11;
    const KI: u32 = 12;

    fn vocab() -> WordPieceVocab {
        WordPieceVocab::from_vocab_txt(VOCAB).expect("hand-built vocabulary must load")
    }

    #[test]
    fn line_number_is_the_token_id() {
        let vocab = vocab();
        assert_eq!(vocab.len(), 13);
        assert!(!vocab.is_empty());

        for (id, expected) in [
            (PAD, "[PAD]"),
            (UNK, "[UNK]"),
            (CLS, "[CLS]"),
            (SEP, "[SEP]"),
            (MASK, "[MASK]"),
            (HELLO, "hello"),
            (WORLD, "##world"),
            (KON, "こん"),
            (KI, "##き"),
        ] {
            assert_eq!(vocab.token(id), Some(expected), "id {id}");
            assert_eq!(vocab.id_of(expected), Some(id), "token {expected}");
        }
    }

    #[test]
    fn special_token_ids_are_exposed() {
        let vocab = vocab();
        assert_eq!(vocab.pad_id(), PAD);
        assert_eq!(vocab.unk_id(), UNK);
        assert_eq!(vocab.cls_id(), CLS);
        assert_eq!(vocab.sep_id(), SEP);
        assert_eq!(vocab.mask_id(), MASK);
    }

    /// A generation loop stops when it sees `[SEP]`; that id has to come from
    /// the vocabulary rather than a constant, or a re-ordered vocab breaks it.
    #[test]
    fn sep_id_is_read_from_the_vocabulary_not_assumed() {
        let reordered = "[SEP]\n[PAD]\n[UNK]\n[CLS]\n[MASK]\nhello\n";
        let vocab = WordPieceVocab::from_vocab_txt(reordered).expect("loads");
        assert_eq!(vocab.sep_id(), 0);
        assert_eq!(vocab.pad_id(), 1);
        assert_eq!(vocab.token(vocab.sep_id()), Some("[SEP]"));
    }

    #[test]
    fn round_trip_of_a_hand_built_vocabulary() {
        let vocab = vocab();
        let text = "hello";
        let id = vocab.id_of(text).expect("token is present");
        assert_eq!(vocab.decode(&[id], true), text);
    }

    #[test]
    fn continuation_tokens_merge_into_the_previous_word() {
        let vocab = vocab();
        assert_eq!(vocab.decode(&[HELLO, WORLD], true), "helloworld");
        assert_eq!(
            vocab.decode_words(&[HELLO, WORLD], true),
            vec!["helloworld".to_string()],
            "`##world` continues `hello`, so this is one word"
        );
    }

    #[test]
    fn a_non_continuation_token_starts_a_new_word() {
        let vocab = vocab();
        assert_eq!(
            vocab.decode_words(&[HELLO, HELLO], true),
            vec!["hello".to_string(), "hello".to_string()],
            "no `##`, so these are two words"
        );
        // The concatenated form cannot tell the two cases apart — which is why
        // `decode_words` exists and is asserted on above.
        assert_eq!(vocab.decode(&[HELLO, HELLO], true), "hellohello");
    }

    #[test]
    fn a_leading_continuation_opens_a_word() {
        let vocab = vocab();
        assert_eq!(
            vocab.decode_words(&[WORLD], true),
            vec!["world".to_string()]
        );
        assert_eq!(vocab.decode(&[CLS, WORLD], true), "world");
    }

    #[test]
    fn skipping_special_tokens_removes_them() {
        let vocab = vocab();
        let ids = [CLS, HELLO, WORLD, SEP, PAD, PAD];
        assert_eq!(vocab.decode(&ids, true), "helloworld");
    }

    #[test]
    fn keeping_special_tokens_renders_them_literally() {
        let vocab = vocab();
        let ids = [CLS, HELLO, WORLD, SEP];
        assert_eq!(vocab.decode(&ids, false), "[CLS]helloworld[SEP]");
        assert_eq!(
            vocab.decode_words(&ids, false),
            vec![
                "[CLS]".to_string(),
                "helloworld".to_string(),
                "[SEP]".to_string()
            ]
        );
    }

    #[test]
    fn every_special_token_is_skippable() {
        let vocab = vocab();
        let ids = [PAD, UNK, CLS, SEP, MASK];
        assert_eq!(vocab.decode(&ids, true), "");
        assert_eq!(vocab.decode(&ids, false), "[PAD][UNK][CLS][SEP][MASK]");
        for id in ids {
            assert!(vocab.is_special_id(id), "id {id} should be special");
        }
        assert!(!vocab.is_special_id(HELLO));
    }

    #[test]
    fn an_empty_id_slice_decodes_to_an_empty_string() {
        let vocab = vocab();
        assert_eq!(vocab.decode(&[], true), "");
        assert_eq!(vocab.decode(&[], false), "");
        assert!(vocab.decode_words(&[], true).is_empty());
        assert_eq!(vocab.try_decode(&[], true).expect("no ids, no error"), "");
    }

    /// The defined behaviour: an id with no token behind it contributes
    /// nothing. It must not panic, and it must not become `[UNK]` — `[UNK]` is
    /// a reading the model can legitimately produce, so emitting it here would
    /// invent a character the model never chose.
    #[test]
    fn an_out_of_range_id_is_dropped_rather_than_invented() {
        let vocab = vocab();
        let beyond = u32::try_from(vocab.len()).expect("small vocab") + 7;
        assert_eq!(vocab.token(beyond), None);
        assert!(!vocab.is_special_id(beyond));

        let decoded = vocab.decode(&[HELLO, beyond, WORLD], true);
        assert_eq!(decoded, "helloworld");
        assert!(
            !decoded.contains("[UNK]"),
            "an unmappable id must never surface as the `[UNK]` token text"
        );

        assert_eq!(vocab.decode(&[u32::MAX], true), "");
        assert_eq!(vocab.decode(&[beyond], false), "");
    }

    /// A continuation after a dropped id attaches to the word that is actually
    /// in progress, rather than silently starting a new one.
    #[test]
    fn a_dropped_id_does_not_break_the_word_in_progress() {
        let vocab = vocab();
        let beyond = u32::try_from(vocab.len()).expect("small vocab");
        assert_eq!(
            vocab.decode_words(&[HELLO, beyond, WORLD], true),
            vec!["helloworld".to_string()]
        );
    }

    #[test]
    fn try_decode_reports_the_id_it_could_not_map() {
        let vocab = vocab();
        let beyond = u32::try_from(vocab.len()).expect("small vocab") + 3;
        let error = vocab
            .try_decode(&[HELLO, beyond], true)
            .expect_err("an unmappable id must be an error, not a placeholder");
        match error {
            TokenizerError::IdOutOfRange { id, size } => {
                assert_eq!(id, beyond);
                assert_eq!(size, vocab.len());
            }
            other => panic!("expected IdOutOfRange, got {other:?}"),
        }
        assert_eq!(
            vocab
                .try_decode(&[HELLO, WORLD], true)
                .expect("all mappable"),
            "helloworld"
        );
    }

    /// Convention for CJK character vocabularies: the decoded string carries no spaces at all.
    /// The Python path this replaces ended in `.replace(' ', '')`.
    #[test]
    fn japanese_output_carries_no_spaces() {
        let vocab = vocab();
        let ids = [CLS, KON, NICHIWA, NEKO, GA, SUKI, KI, SEP];
        let decoded = vocab.decode(&ids, true);
        assert_eq!(decoded, "こんにちは猫が好き");
        assert!(
            !decoded.contains(' '),
            "pieces join with nothing; got {decoded:?}"
        );
        assert!(!decoded.contains('#'), "the `##` marker must be stripped");
        assert_eq!(
            vocab.decode_words(&ids, true),
            vec![
                "こんにちは".to_string(),
                "猫が".to_string(),
                "好き".to_string()
            ],
            "three words merged from seven pieces, joined with nothing by `decode`"
        );
    }

    #[test]
    fn multibyte_continuations_do_not_split_characters() {
        let vocab = vocab();
        // `##にちは` is 3 characters / 9 UTF-8 bytes; merging must be by string,
        // not by byte index.
        assert_eq!(vocab.decode(&[KON, NICHIWA], true), "こんにちは");
        assert_eq!(vocab.decode(&[KON, NICHIWA], true).chars().count(), 5);
    }

    #[test]
    fn an_empty_vocabulary_is_an_error() {
        assert!(matches!(
            WordPieceVocab::from_vocab_txt(""),
            Err(TokenizerError::EmptyVocab)
        ));
        assert!(matches!(
            WordPieceVocab::from_vocab_txt("\n"),
            Err(TokenizerError::MissingSpecialToken(_)),
        ));
    }

    #[test]
    fn a_missing_special_token_is_an_error() {
        for omitted in SPECIAL_TOKENS {
            let text: String = SPECIAL_TOKENS
                .iter()
                .filter(|token| **token != omitted)
                .map(|token| format!("{token}\n"))
                .collect::<String>()
                + "hello\n";
            match WordPieceVocab::from_vocab_txt(&text) {
                Err(TokenizerError::MissingSpecialToken(name)) => assert_eq!(name, omitted),
                Err(other) => panic!("expected MissingSpecialToken({omitted}), got {other:?}"),
                Ok(_) => panic!("a vocabulary without {omitted} must not load"),
            }
        }
    }

    #[test]
    fn a_blank_line_keeps_its_id_slot() {
        // Dropping the blank would shift `hello` from 6 to 5 and corrupt every
        // decode after it.
        let text = "[PAD]\n[UNK]\n[CLS]\n[SEP]\n[MASK]\n\nhello\n";
        let vocab = WordPieceVocab::from_vocab_txt(text).expect("loads");
        assert_eq!(vocab.len(), 7);
        assert_eq!(vocab.token(5), Some(""));
        assert_eq!(vocab.token(6), Some("hello"));
    }

    #[test]
    fn crlf_and_a_missing_trailing_newline_both_parse() {
        let crlf = "[PAD]\r\n[UNK]\r\n[CLS]\r\n[SEP]\r\n[MASK]\r\nhello";
        let vocab = WordPieceVocab::from_vocab_txt(crlf).expect("loads");
        assert_eq!(vocab.len(), 6);
        assert_eq!(vocab.token(5), Some("hello"));
        assert_eq!(vocab.decode(&[5], true), "hello");
    }

    #[test]
    fn parses_through_the_from_str_trait_too() {
        let vocab: WordPieceVocab = VOCAB.parse().expect("FromStr agrees with from_vocab_txt");
        assert_eq!(vocab.len(), 13);
        assert_eq!(vocab.decode(&[CLS, HELLO, WORLD, SEP], true), "helloworld");
    }

    #[test]
    fn loads_from_a_file_on_disk() {
        let path = std::env::temp_dir().join(format!(
            "comic-ocr-vocab-{}-{:?}.txt",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::write(&path, VOCAB).expect("write temp vocabulary");

        let vocab = WordPieceVocab::from_file(&path).expect("loads from disk");
        let _ = fs::remove_file(&path);

        assert_eq!(vocab.len(), 13);
        assert_eq!(vocab.decode(&[CLS, KON, NICHIWA, SEP], true), "こんにちは");
    }

    #[test]
    fn a_missing_file_is_an_error_not_an_empty_vocabulary() {
        let path = std::env::temp_dir().join("comic-ocr-vocab-does-not-exist-9d1f7.txt");
        match WordPieceVocab::from_file(&path) {
            Err(TokenizerError::Io { path: reported, .. }) => {
                assert!(reported.contains("comic-ocr-vocab-does-not-exist-9d1f7"));
            }
            Err(other) => panic!("expected Io, got {other:?}"),
            Ok(_) => panic!("a missing vocabulary file must not produce a vocabulary"),
        }
    }

    #[test]
    fn tokenizer_errors_convert_into_ocr_errors() {
        let error: OcrError = TokenizerError::EmptyVocab.into();
        match error {
            OcrError::TokenizerError(message) => assert!(message.contains("empty")),
            other => panic!("expected OcrError::TokenizerError, got {other:?}"),
        }
    }
}
