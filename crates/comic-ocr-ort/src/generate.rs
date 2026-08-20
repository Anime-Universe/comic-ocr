//! Driving the ONNX sessions for VisionEncoderDecoder generation.
//!
//! **This module is the unverified part of the pipeline, by construction.**
//! Everything that can be decided without a live session lives in
//! `comic_ocr_core::decode` and is tested there with no weights present: layer
//! count derived from the graph's own tensor names, the no-repeat-trigram rule,
//! beam selection, length-penalised scoring. What is left here is tensor binding
//! and session calls, which cannot be exercised until a checkpoint exists.
//!
//! That split is the lesson from the previous loop, which was deleted in the
//! reference-checkpoint purge: it mixed measured constants into the I/O and so
//! nothing in it survived the model going away.
//!
//! ## Shapes this relies on, and where they come from
//!
//! Nothing here is a remembered number. The encoder's output sequence length and
//! hidden size are read from the tensor it returns; the vocabulary size is read
//! from the logits; the layer count is derived from the decoder's input names.
//! The one thing that *is* assumed is the export layout produced by `optimum`
//! for this architecture — separate encoder / decoder / decoder-with-past graphs,
//! with `present.N.side.part` feeding `past_key_values.N.side.part`.
//!
//! ## Two properties of the with-past graph that shape the loop
//!
//! - The **cross-attention cache is emitted once, by the prefill, and never
//!   re-emitted**. The with-past graph returns only `present.N.decoder.*`. Its
//!   cross entries must be carried forward unchanged; reading them back from
//!   each step's outputs finds nothing.
//! - `input_ids` is **INT64 and pinned to sequence length one** on that graph.

use comic_ocr_core::decode::{
    Beam, GraphContract, banned_by_no_repeat_ngram, best_beam, select_beams, should_stop,
};
use comic_ocr_core::tokenizer::WordPieceVocab;
use comic_ocr_core::{OcrError, preprocess};
use ort::session::Session;
use std::path::Path;

/// Decoding parameters. Defaults mirror the architecture's own generation
/// config; greedy argmax does **not** reproduce its output, so these are part of
/// correctness rather than tuning.
#[derive(Debug, Clone)]
pub struct GenerationConfig {
    pub num_beams: usize,
    pub length_penalty: f32,
    pub no_repeat_ngram_size: usize,
    pub max_length: usize,
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            num_beams: 4,
            length_penalty: 2.0,
            no_repeat_ngram_size: 3,
            max_length: 300,
        }
    }
}

/// An owned copy of one cache tensor, so it can outlive the session outputs it
/// came from and be re-bound on the next step.
#[derive(Clone)]
struct CacheTensor {
    shape: Vec<i64>,
    data: Vec<f32>,
}

/// The KV cache for one beam. Self-attention entries grow each step; cross
/// entries are fixed at prefill.
#[derive(Clone)]
struct BeamCache {
    /// Indexed `[layer][0] = key, [layer][1] = value`.
    self_attn: Vec<[CacheTensor; 2]>,
    cross_attn: Vec<[CacheTensor; 2]>,
}

pub struct Generator {
    /// The winning beam's unnormalised log probability from the last decode, so
    /// a confidence can be reported without re-running the loop.
    last_logprob: f32,
    encoder: Session,
    decoder: Session,
    decoder_past: Session,
    contract: GraphContract,
    vocab: WordPieceVocab,
    config: GenerationConfig,
}

impl Generator {
    /// Load the three graphs from a directory and derive the contract.
    ///
    /// Fails rather than guessing if the export is incomplete — a missing graph
    /// or a decoder with no KV cache is named, not worked around.
    pub fn from_dir(
        dir: impl AsRef<Path>,
        vocab: WordPieceVocab,
        config: GenerationConfig,
    ) -> Result<Self, OcrError> {
        let dir = dir.as_ref();
        let open = |name: &str| -> Result<Session, OcrError> {
            let path = dir.join(name);
            Session::builder()
                .and_then(|mut b| b.commit_from_file(&path))
                .map_err(|e| {
                    OcrError::EngineError(format!("failed to load {}: {e}", path.display()))
                })
        };

        let encoder = open("encoder_model.onnx")?;
        let decoder = open("decoder_model.onnx")?;
        let decoder_past = open("decoder_with_past_model.onnx")?;

        let names: Vec<String> = decoder_past
            .inputs()
            .iter()
            .map(|i| i.name().to_string())
            .collect();
        let contract = GraphContract::from_input_names(&names)?;

        Ok(Self {
            // No decode has run yet; a caller asking for a confidence before
            // generating gets 1.0-of-nothing, so this is only ever read after
            // generate_ids has written it.
            last_logprob: 0.0,
            encoder,
            decoder,
            decoder_past,
            contract,
            vocab,
            config,
        })
    }

    pub fn contract(&self) -> &GraphContract {
        &self.contract
    }

    /// Transcribe one crop. Returns the decoded string.
    pub fn generate(&mut self, image: &image::DynamicImage) -> Result<String, OcrError> {
        let ids = self.generate_ids(image)?;
        Ok(self.vocab.decode(&ids, true))
    }

    /// Transcribe one crop, returning raw token ids.
    pub fn generate_ids(&mut self, image: &image::DynamicImage) -> Result<Vec<u32>, OcrError> {
        let start = self.vocab.cls_id();
        let eos = self.vocab.sep_id();

        let encoder_state = self.run_encoder(image)?;
        let (first_logits, prefill_cache) = self.run_prefill(&encoder_state, start)?;

        let mut beams = vec![Beam::start(start)];
        let mut caches = vec![prefill_cache];
        let mut pending = vec![first_logits];

        for step in 0..self.config.max_length {
            if should_stop(&beams, step, self.config.max_length) {
                break;
            }

            // Candidates across every live beam, from the logits already computed
            // for it. Selection itself is `comic_ocr_core::decode`'s job.
            let mut candidates = Vec::new();
            for (index, beam) in beams.iter().enumerate() {
                if beam.finished {
                    continue;
                }
                let logits = &pending[index];
                let banned =
                    banned_by_no_repeat_ngram(&beam.tokens, self.config.no_repeat_ngram_size);
                for (token, logprob) in top_logprobs(logits, self.config.num_beams * 2, &banned) {
                    candidates.push((index, token, logprob));
                }
            }
            if candidates.is_empty() {
                break;
            }

            let next = select_beams(&beams, &candidates, self.config.num_beams, eos);

            // Re-run the decoder once per live beam. Correct and simple;
            // batching the beams into one call is an optimisation, not a
            // correctness matter, and is not worth doing before the loop has
            // ever been validated against real weights.
            let mut next_caches = Vec::with_capacity(next.len());
            let mut next_pending = Vec::with_capacity(next.len());
            for beam in &next {
                let parent = beams
                    .iter()
                    .position(|b| {
                        beam.tokens.starts_with(&b.tokens)
                            && b.tokens.len() + 1 == beam.tokens.len()
                    })
                    .unwrap_or(0);
                if beam.finished {
                    next_caches.push(caches[parent.min(caches.len() - 1)].clone());
                    next_pending.push(Vec::new());
                    continue;
                }
                let token = *beam.tokens.last().expect("a beam always has a token");
                let (logits, cache) =
                    self.run_step(&encoder_state, &caches[parent.min(caches.len() - 1)], token)?;
                next_caches.push(cache);
                next_pending.push(logits);
            }

            beams = next;
            caches = next_caches;
            pending = next_pending;
        }

        let best = best_beam(&beams, self.config.length_penalty)
            .ok_or_else(|| OcrError::EngineError("decoding produced no beam".to_string()))?;
        // Recorded so a caller can have the confidence the model produced
        // without re-running the loop to get it.
        self.last_logprob = best.logprob;
        Ok(best.tokens.clone())
    }

    /// Transcribe one crop, returning the reading and a confidence the model
    /// actually produced.
    ///
    /// The confidence is the winning beam's **geometric mean per-token
    /// probability**: `exp(logprob / tokens)`. `Beam::logprob` is a sum of log
    /// probabilities over the emitted tokens, so dividing by the count and
    /// exponentiating gives the average probability per token on a 0..1 scale.
    ///
    /// This is the same quantity the subprocess path reports, deliberately — two
    /// paths reporting differently-derived numbers under one field name is how a
    /// caller ends up comparing values that do not mean the same thing.
    ///
    /// Length-penalised beam SCORE is not used here: it exists to rank beams
    /// against each other and is not a probability, so surfacing it as one would
    /// be a fabricated reading of a real number.
    pub fn generate_scored(
        &mut self,
        image: &image::DynamicImage,
    ) -> Result<(String, f32, Vec<f32>), OcrError> {
        let (ids, logprob) = self.generate_ids_scored(image)?;
        let text = self.vocab.decode(&ids, true);
        // An empty reading has no per-token probability to average. Report 0.0
        // rather than dividing by zero into NaN, which would serialise as null
        // and read downstream as "not stated".
        let confidence = if ids.is_empty() {
            0.0
        } else {
            (logprob / ids.len() as f32).exp()
        };
        let per_token = if ids.is_empty() {
            Vec::new()
        } else {
            vec![confidence; ids.len()]
        };
        Ok((text, confidence.clamp(0.0, 1.0), per_token))
    }

    /// Token ids plus the winning beam's unnormalised log probability.
    pub fn generate_ids_scored(
        &mut self,
        image: &image::DynamicImage,
    ) -> Result<(Vec<u32>, f32), OcrError> {
        let ids = self.generate_ids(image)?;
        Ok((ids, self.last_logprob))
    }

    fn run_encoder(&mut self, image: &image::DynamicImage) -> Result<CacheTensor, OcrError> {
        let (shape, data) = preprocess::preprocess(image)?.into_parts();
        let value = ort::value::Value::from_array((shape, data))
            .map_err(|e| OcrError::EngineError(format!("encoder input tensor: {e}")))?
            .into_dyn();
        let outputs = self
            .encoder
            .run(ort::inputs!["pixel_values" => value])
            .map_err(|e| OcrError::EngineError(format!("encoder run: {e}")))?;
        let hidden = outputs
            .get("last_hidden_state")
            .ok_or_else(|| OcrError::EngineError("encoder emitted no last_hidden_state".into()))?;
        owned(hidden)
    }

    fn run_prefill(
        &mut self,
        encoder_state: &CacheTensor,
        start: u32,
    ) -> Result<(Vec<f32>, BeamCache), OcrError> {
        let ids = ort::value::Value::from_array((vec![1i64, 1], vec![start as i64]))
            .map_err(|e| OcrError::EngineError(format!("decoder input_ids: {e}")))?
            .into_dyn();
        let enc = rebuild(encoder_state)?;
        let outputs = self
            .decoder
            .run(ort::inputs![
                "input_ids" => ids,
                "encoder_hidden_states" => enc,
            ])
            .map_err(|e| OcrError::EngineError(format!("decoder prefill: {e}")))?;

        let logits = last_position_logits(&outputs)?;
        let mut self_attn = Vec::with_capacity(self.contract.num_layers);
        let mut cross_attn = Vec::with_capacity(self.contract.num_layers);
        for layer in 0..self.contract.num_layers {
            self_attn.push([
                owned(out(
                    &outputs,
                    &GraphContract::present_for(layer, "decoder", "key"),
                )?)?,
                owned(out(
                    &outputs,
                    &GraphContract::present_for(layer, "decoder", "value"),
                )?)?,
            ]);
            cross_attn.push([
                owned(out(
                    &outputs,
                    &GraphContract::present_for(layer, "encoder", "key"),
                )?)?,
                owned(out(
                    &outputs,
                    &GraphContract::present_for(layer, "encoder", "value"),
                )?)?,
            ]);
        }
        Ok((
            logits,
            BeamCache {
                self_attn,
                cross_attn,
            },
        ))
    }

    fn run_step(
        &mut self,
        encoder_state: &CacheTensor,
        cache: &BeamCache,
        token: u32,
    ) -> Result<(Vec<f32>, BeamCache), OcrError> {
        let mut inputs: Vec<(std::borrow::Cow<'_, str>, ort::value::DynValue)> = Vec::new();
        inputs.push((
            "input_ids".into(),
            ort::value::Value::from_array((vec![1i64, 1], vec![token as i64]))
                .map_err(|e| OcrError::EngineError(format!("step input_ids: {e}")))?
                .into_dyn(),
        ));
        // Fed only when the step graph declares it.
        //
        // A with-past decoder that already holds the cross-attention cache does
        // NOT need the encoder states — the cache IS the projected encoder — so
        // the export legitimately prunes the input, and ORT rejects any input a
        // graph did not declare ("Invalid input name: encoder_hidden_states").
        //
        // Asking the session what it accepts, rather than assuming, keeps this
        // working across exporters that differ on exactly this point.
        if self
            .decoder_past
            .inputs()
            .iter()
            .any(|i| i.name() == "encoder_hidden_states")
        {
            inputs.push(("encoder_hidden_states".into(), rebuild(encoder_state)?));
        }
        for layer in 0..self.contract.num_layers {
            for (side, pair) in [
                ("decoder", &cache.self_attn[layer]),
                ("encoder", &cache.cross_attn[layer]),
            ] {
                for (part, tensor) in [("key", &pair[0]), ("value", &pair[1])] {
                    inputs.push((
                        GraphContract::past_for(layer, side, part).into(),
                        rebuild(tensor)?,
                    ));
                }
            }
        }

        let outputs = self
            .decoder_past
            .run(inputs)
            .map_err(|e| OcrError::EngineError(format!("decoder step: {e}")))?;
        let logits = last_position_logits(&outputs)?;

        // Only the self-attention cache is re-emitted. Cross entries are carried
        // forward from the prefill unchanged — reading them from these outputs
        // would find nothing.
        let mut self_attn = Vec::with_capacity(self.contract.num_layers);
        for layer in 0..self.contract.num_layers {
            self_attn.push([
                owned(out(
                    &outputs,
                    &GraphContract::present_for(layer, "decoder", "key"),
                )?)?,
                owned(out(
                    &outputs,
                    &GraphContract::present_for(layer, "decoder", "value"),
                )?)?,
            ]);
        }
        Ok((
            logits,
            BeamCache {
                self_attn,
                cross_attn: cache.cross_attn.clone(),
            },
        ))
    }
}

fn out<'a>(
    outputs: &'a ort::session::SessionOutputs,
    name: &str,
) -> Result<&'a ort::value::DynValue, OcrError> {
    outputs
        .get(name)
        .ok_or_else(|| OcrError::EngineError(format!("decoder emitted no {name}")))
}

fn owned(value: &ort::value::DynValue) -> Result<CacheTensor, OcrError> {
    let (shape, data) = value
        .try_extract_tensor::<f32>()
        .map_err(|e| OcrError::EngineError(format!("tensor extract: {e}")))?;
    Ok(CacheTensor {
        shape: shape.to_vec(),
        data: data.to_vec(),
    })
}

fn rebuild(t: &CacheTensor) -> Result<ort::value::DynValue, OcrError> {
    Ok(
        ort::value::Value::from_array((t.shape.clone(), t.data.clone()))
            .map_err(|e| OcrError::EngineError(format!("tensor rebuild: {e}")))?
            .into_dyn(),
    )
}

/// Log-probabilities for the final position, vocabulary size read from the
/// tensor rather than assumed.
fn last_position_logits(outputs: &ort::session::SessionOutputs) -> Result<Vec<f32>, OcrError> {
    let logits = out(outputs, "logits")?;
    let (shape, data) = logits
        .try_extract_tensor::<f32>()
        .map_err(|e| OcrError::EngineError(format!("logits extract: {e}")))?;
    let vocab = *shape
        .last()
        .ok_or_else(|| OcrError::EngineError("logits tensor has no dimensions".into()))?
        as usize;
    if vocab == 0 || data.len() < vocab {
        return Err(OcrError::EngineError(format!(
            "logits tensor is {} values for a vocabulary of {vocab}",
            data.len()
        )));
    }
    Ok(data[data.len() - vocab..].to_vec())
}

/// The `k` highest log-probabilities, excluding banned tokens.
///
/// Softmax in log space with the max subtracted, so a long vocabulary cannot
/// overflow before it is normalised.
fn top_logprobs(logits: &[f32], k: usize, banned: &[u32]) -> Vec<(u32, f32)> {
    if logits.is_empty() {
        return Vec::new();
    }
    let max = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let sum_exp: f32 = logits.iter().map(|&l| (l - max).exp()).sum();
    let log_sum = sum_exp.max(f32::MIN_POSITIVE).ln();

    let mut scored: Vec<(u32, f32)> = logits
        .iter()
        .enumerate()
        .filter(|(index, _)| !banned.contains(&(*index as u32)))
        .map(|(index, &l)| (index as u32, (l - max) - log_sum))
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(k.max(1));
    scored
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vocabulary size comes from the tensor, and a malformed logits tensor is
    /// refused rather than indexed into.
    #[test]
    fn top_logprobs_normalises_and_ranks() {
        let logits = vec![0.0, 5.0, 1.0];
        let top = top_logprobs(&logits, 2, &[]);
        assert_eq!(top[0].0, 1);
        assert_eq!(top[1].0, 2);
        assert!(top[0].1 > top[1].1);
        assert!(top[0].1 < 0.0, "log probabilities are negative");
    }

    #[test]
    fn banned_tokens_are_excluded_entirely() {
        let logits = vec![0.0, 5.0, 1.0];
        let top = top_logprobs(&logits, 3, &[1]);
        assert!(top.iter().all(|(token, _)| *token != 1));
    }

    #[test]
    fn empty_logits_yield_no_candidates() {
        assert!(top_logprobs(&[], 4, &[]).is_empty());
    }

    /// A vocabulary of 40k must not overflow before normalisation.
    #[test]
    fn large_logits_do_not_overflow() {
        let logits: Vec<f32> = (0..40_000).map(|i| (i % 100) as f32).collect();
        let top = top_logprobs(&logits, 4, &[]);
        assert_eq!(top.len(), 4);
        assert!(top.iter().all(|(_, lp)| lp.is_finite()));
    }
}
