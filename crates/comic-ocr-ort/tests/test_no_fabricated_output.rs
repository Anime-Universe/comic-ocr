//! No engine path may return a value it did not compute.
//!
//! This file exists because the same defect appeared three times in three
//! different disguises, and each one passed the whole test suite:
//!
//!   1. `"…"` returned as text when the inference subprocess failed, with
//!      `confidence: 0.985` attached.
//!   2. `confidence: 0.985` and `token_probabilities: [0.99, 0.985, 0.988]`
//!      hardcoded on every success — later relocated into the generated Python
//!      string, where the Rust `val.get("confidence")` around it read like
//!      genuine extraction.
//!   3. `let raw_text = "ONNX_NATIVE_PREDICTION"` returned as the transcription
//!      from the native path, with a *real* softmax confidence beside it. That
//!      is the worst shape available: `Ok`, a plausible score, and a string that
//!      never came from the model.
//!
//! None of these are caught by testing behaviour, because the fabricated value
//! is indistinguishable from a real one at the call site — that is what makes it
//! dangerous. So this scans the source instead.
//!
//! It is deliberately crude. A cleverer check would be easier to work around.

use std::fs;
use std::path::{Path, PathBuf};

fn engine_sources() -> Vec<PathBuf> {
    let mut found = Vec::new();
    for dir in [
        Path::new("src"),
        Path::new("crates/comic-ocr-ort/src"),
        Path::new("../comic-ocr-ort/src"),
    ] {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                    found.push(path);
                }
            }
        }
        if !found.is_empty() {
            break;
        }
    }
    found
}

/// The scan must prove it found something before it can prove anything absent.
/// A source gate that silently scanned zero files would pass forever, which is
/// the same failure mode it exists to prevent.
#[test]
fn the_scan_actually_reads_the_engine() {
    let sources = engine_sources();
    assert!(
        !sources.is_empty(),
        "found no engine sources to scan — this gate would pass vacuously"
    );
    let total: usize = sources
        .iter()
        .filter_map(|p| fs::read_to_string(p).ok())
        .map(|s| s.len())
        .sum();
    assert!(total > 2000, "engine sources scanned only {total} bytes");
}

#[test]
fn no_engine_path_returns_a_placeholder_transcription() {
    for path in engine_sources() {
        let source = fs::read_to_string(&path).expect("read engine source");
        for (number, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with("///") {
                continue; // the header above names these on purpose
            }
            for forbidden in [
                "ONNX_NATIVE_PREDICTION",
                "PLACEHOLDER",
                "\"STUB\"",
                "\"DUMMY\"",
            ] {
                assert!(
                    !trimmed.contains(forbidden),
                    "{}:{} returns the placeholder {forbidden} as if it were model output. \
                     Return Err(OcrError::NotImplemented) instead — a caller cannot tell a \
                     placeholder from a reading, and it ends up in the corpus.",
                    path.display(),
                    number + 1
                );
            }
        }
    }
}

#[test]
fn no_engine_path_hardcodes_a_confidence() {
    // The exact constants that shipped. Not a general "no float literals" rule:
    // thresholds and normalisation constants are legitimate, and a rule broad
    // enough to catch those would be turned off within a week.
    for path in engine_sources() {
        let source = fs::read_to_string(&path).expect("read engine source");
        for (number, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with("///") {
                continue;
            }
            let fabricated = ["confidence: 0.985", "0.99, 0.985, 0.988", "0.985,"];
            for pattern in fabricated {
                assert!(
                    !trimmed.contains(pattern),
                    "{}:{} carries the hardcoded confidence `{pattern}`. Confidence must be \
                     computed from the model's own probabilities, or absent — never defaulted.",
                    path.display(),
                    number + 1
                );
            }
        }
    }
}

/// The sentinel that used to stand in for a failed read. `Err` is the answer.
#[test]
fn failure_is_never_reported_as_an_ellipsis_reading() {
    for path in engine_sources() {
        let source = fs::read_to_string(&path).expect("read engine source");
        for (number, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with("///") {
                continue;
            }
            assert!(
                !trimmed.contains("\"…\".to_string()"),
                "{}:{} returns the ellipsis sentinel as a transcription. Return Err instead.",
                path.display(),
                number + 1
            );
        }
    }
}
