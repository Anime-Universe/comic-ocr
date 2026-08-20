//! Sequence-decoding logic, with no session and no model.
//!
//! Split out deliberately. The previous generation loop was deleted in the
//! reference-checkpoint purge because every constant in it — layer count, start
//! token, encoder sequence length — had been *measured from one model* and was
//! therefore worthless the moment the model changed. Nothing here is measured
//! from a checkpoint: the graph shape is derived from the graph's own tensor
//! names, the special tokens come from the vocabulary, and the decode rules are
//! arithmetic.
//!
//! The consequence is that all of this is testable with no weights present,
//! which is the whole point — the parts that need a live session are confined to
//! `comic-ocr-ort::generate`, and are the only parts that stay unverified until
//! a checkpoint exists.

use crate::OcrError;

/// The shape of a VisionEncoderDecoder ONNX export, read off the graphs.
///
/// Derived rather than declared. A decoder trimmed to two layers and one trimmed
/// to four differ only in how many cache tensors they name, so counting the
/// names is both simpler and more durable than a constant somebody has to
/// remember to update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphContract {
    /// Decoder layers, each contributing four cache tensors (self key/value,
    /// cross key/value).
    pub num_layers: usize,
}

impl GraphContract {
    /// Count decoder layers from the with-past graph's input names.
    ///
    /// Inputs are named `past_key_values.{layer}.{decoder|encoder}.{key|value}`,
    /// so the layer count is one more than the highest index seen. Counting the
    /// names and dividing by four would give the same answer on a well-formed
    /// graph and a wrong one on a malformed export; taking the maximum index and
    /// then checking every expected name is present catches a truncated export
    /// instead of silently believing it.
    pub fn from_input_names(names: &[impl AsRef<str>]) -> Result<Self, OcrError> {
        let mut highest: Option<usize> = None;
        for name in names {
            let name = name.as_ref();
            let Some(rest) = name.strip_prefix("past_key_values.") else {
                continue;
            };
            let Some((index, _)) = rest.split_once('.') else {
                continue;
            };
            let Ok(index) = index.parse::<usize>() else {
                return Err(OcrError::EngineError(format!(
                    "cache tensor {name} has a non-numeric layer index"
                )));
            };
            highest = Some(highest.map_or(index, |current: usize| current.max(index)));
        }

        let Some(highest) = highest else {
            return Err(OcrError::EngineError(
                "no past_key_values.* inputs on the decoder graph — this export has no KV cache, \
                 so single-token decoding is impossible"
                    .to_string(),
            ));
        };
        let num_layers = highest + 1;

        // Every layer must contribute all four tensors. A gap means a partial
        // export, which would otherwise surface as a shape error deep in the
        // first decode step.
        for layer in 0..num_layers {
            for side in ["decoder", "encoder"] {
                for part in ["key", "value"] {
                    let expected = format!("past_key_values.{layer}.{side}.{part}");
                    if !names.iter().any(|n| n.as_ref() == expected) {
                        return Err(OcrError::EngineError(format!(
                            "decoder graph declares {num_layers} layers but is missing {expected}"
                        )));
                    }
                }
            }
        }

        Ok(Self { num_layers })
    }

    /// The `present.*` output name whose value feeds `past_key_values.*` on the
    /// next step. Pairing is exact prefix substitution.
    pub fn present_for(layer: usize, side: &str, part: &str) -> String {
        format!("present.{layer}.{side}.{part}")
    }

    pub fn past_for(layer: usize, side: &str, part: &str) -> String {
        format!("past_key_values.{layer}.{side}.{part}")
    }
}

/// Token ids a no-repeat-ngram rule forbids at the next step.
///
/// `generation_config` for this architecture sets `no_repeat_ngram_size: 3`, and
/// greedy argmax without it does not reproduce the reference implementation's
/// output — so this is part of correctness, not a refinement.
///
/// The rule: if the last `n-1` tokens have occurred before, whatever followed
/// them then may not be emitted now.
pub fn banned_by_no_repeat_ngram(generated: &[u32], ngram_size: usize) -> Vec<u32> {
    if ngram_size == 0 || generated.len() < ngram_size {
        return Vec::new();
    }
    let prefix_len = ngram_size - 1;
    let suffix = &generated[generated.len() - prefix_len..];

    let mut banned = Vec::new();
    // Every position where the same prefix occurred and something followed it.
    for start in 0..generated.len().saturating_sub(prefix_len) {
        if &generated[start..start + prefix_len] == suffix {
            let next = start + prefix_len;
            if next < generated.len() && !banned.contains(&generated[next]) {
                banned.push(generated[next]);
            }
        }
    }
    banned
}

/// A beam under consideration.
#[derive(Debug, Clone, PartialEq)]
pub struct Beam {
    pub tokens: Vec<u32>,
    /// Sum of log probabilities. Kept unnormalised; length penalty is applied
    /// only when ranking finished beams, matching the reference behaviour.
    pub logprob: f32,
    pub finished: bool,
}

impl Beam {
    pub fn start(start_token: u32) -> Self {
        Self {
            tokens: vec![start_token],
            logprob: 0.0,
            finished: false,
        }
    }

    /// Length-penalised score used to rank finished beams.
    ///
    /// `score = logprob / len^penalty`. With `length_penalty = 2.0` this favours
    /// longer sequences, which is what the reference config asks for; using the
    /// raw logprob instead systematically prefers short truncated readings.
    pub fn score(&self, length_penalty: f32) -> f32 {
        let len = self.tokens.len().max(1) as f32;
        self.logprob / len.powf(length_penalty)
    }
}

/// Pick the `beam_width` best continuations across all live beams.
///
/// Takes `(beam_index, token, logprob)` candidates already computed by the
/// caller — which keeps every session detail out of this function and makes the
/// selection rule testable on its own.
pub fn select_beams(
    beams: &[Beam],
    candidates: &[(usize, u32, f32)],
    beam_width: usize,
    eos_token: u32,
) -> Vec<Beam> {
    let mut next: Vec<Beam> = candidates
        .iter()
        .filter_map(|&(beam_index, token, logprob)| {
            let parent = beams.get(beam_index)?;
            if parent.finished {
                return None;
            }
            let mut tokens = parent.tokens.clone();
            tokens.push(token);
            Some(Beam {
                finished: token == eos_token,
                tokens,
                logprob: parent.logprob + logprob,
            })
        })
        .collect();

    // Finished beams stay in contention — they are carried forward so a short
    // high-probability reading is not discarded before ranking.
    next.extend(beams.iter().filter(|b| b.finished).cloned());

    next.sort_by(|a, b| {
        b.logprob
            .partial_cmp(&a.logprob)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    next.truncate(beam_width.max(1));
    next
}

/// Whether decoding should stop: every beam finished, or the cap is reached.
pub fn should_stop(beams: &[Beam], step: usize, max_length: usize) -> bool {
    step >= max_length || (!beams.is_empty() && beams.iter().all(|b| b.finished))
}

/// The best beam by length-penalised score.
pub fn best_beam(beams: &[Beam], length_penalty: f32) -> Option<&Beam> {
    beams.iter().max_by(|a, b| {
        a.score(length_penalty)
            .partial_cmp(&b.score(length_penalty))
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names_for(layers: usize) -> Vec<String> {
        let mut v = vec!["input_ids".to_string(), "encoder_hidden_states".to_string()];
        for layer in 0..layers {
            for side in ["decoder", "encoder"] {
                for part in ["key", "value"] {
                    v.push(GraphContract::past_for(layer, side, part));
                }
            }
        }
        v
    }

    #[test]
    fn layer_count_is_derived_from_the_graph_not_assumed() {
        for layers in [1usize, 2, 4, 12] {
            let contract = GraphContract::from_input_names(&names_for(layers)).unwrap();
            assert_eq!(contract.num_layers, layers);
        }
    }

    /// A truncated export must be refused, not believed. Deleting one tensor
    /// leaves a name set that a naive count-and-divide would accept.
    #[test]
    fn a_missing_cache_tensor_is_an_error() {
        let mut names = names_for(2);
        names.retain(|n| n != "past_key_values.1.encoder.value");
        let error = GraphContract::from_input_names(&names).unwrap_err();
        assert!(format!("{error}").contains("past_key_values.1.encoder.value"));
    }

    #[test]
    fn a_graph_with_no_cache_is_an_error_naming_why() {
        let error =
            GraphContract::from_input_names(&["input_ids", "encoder_hidden_states"]).unwrap_err();
        assert!(format!("{error}").contains("no KV cache"));
    }

    #[test]
    fn no_repeat_ngram_bans_the_token_that_followed_a_repeated_prefix() {
        // "a b c ... a b" -> c is banned
        let generated = [10u32, 11, 12, 20, 10, 11];
        assert_eq!(banned_by_no_repeat_ngram(&generated, 3), vec![12]);
    }

    #[test]
    fn no_repeat_ngram_is_inert_below_its_window() {
        assert!(banned_by_no_repeat_ngram(&[10, 11], 3).is_empty());
        assert!(banned_by_no_repeat_ngram(&[10, 11, 12], 0).is_empty());
    }

    #[test]
    fn no_repeat_ngram_collects_every_distinct_follower() {
        // prefix (10,11) occurred twice, followed by 12 and then 13
        let generated = [10u32, 11, 12, 99, 10, 11, 13, 99, 10, 11];
        let mut banned = banned_by_no_repeat_ngram(&generated, 3);
        banned.sort_unstable();
        assert_eq!(banned, vec![12, 13]);
    }

    /// Length penalty is why this is not just argmax: with penalty 2.0 a longer
    /// sequence at the same total logprob must win.
    #[test]
    fn length_penalty_favours_the_longer_reading() {
        let short = Beam {
            tokens: vec![2, 5, 3],
            logprob: -3.0,
            finished: true,
        };
        let long = Beam {
            tokens: vec![2, 5, 6, 7, 3],
            logprob: -3.0,
            finished: true,
        };
        assert!(long.score(2.0) > short.score(2.0));
        // and with no penalty they tie
        assert!((long.score(0.0) - short.score(0.0)).abs() < 1e-6);
    }

    #[test]
    fn select_keeps_the_best_and_respects_the_width() {
        let beams = vec![Beam::start(2)];
        let next = select_beams(&beams, &[(0, 10, -0.1), (0, 11, -2.0), (0, 12, -0.5)], 2, 3);
        assert_eq!(next.len(), 2);
        assert_eq!(next[0].tokens, vec![2, 10]);
        assert_eq!(next[1].tokens, vec![2, 12]);
    }

    #[test]
    fn a_beam_ending_in_eos_is_finished_and_carried_forward() {
        let beams = vec![Beam::start(2)];
        let next = select_beams(&beams, &[(0, 3, -0.2)], 4, 3);
        assert!(next[0].finished);
        // it survives the following step rather than being dropped
        let after = select_beams(&next, &[], 4, 3);
        assert_eq!(after.len(), 1);
        assert!(after[0].finished);
    }

    #[test]
    fn stopping_requires_every_beam_finished_or_the_cap() {
        let live = vec![Beam::start(2)];
        assert!(!should_stop(&live, 1, 300));
        assert!(should_stop(&live, 300, 300));
        let done = vec![Beam {
            tokens: vec![2, 3],
            logprob: -1.0,
            finished: true,
        }];
        assert!(should_stop(&done, 1, 300));
    }
}
