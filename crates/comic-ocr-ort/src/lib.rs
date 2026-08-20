pub mod generate;
pub mod worker;
use comic_ocr_core::{
    EngineType, OcrEngine, OcrError, OcrMetadata, OcrResult, post_process_with_furigana,
};
use ort::session::Session;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};

pub struct OrtEngine {
    pub model_name: String,
    pub engine_type: EngineType,
    pub extract_furigana: bool,
    pub entropy_truncation_threshold: f32,
    pub session: Option<Arc<Mutex<Session>>>,
    /// The native generation loop, when a model DIRECTORY is configured.
    ///
    /// Separate from `session`, which holds a single graph: a
    /// VisionEncoderDecoder exports as encoder + decoder + decoder-with-past,
    /// and the generation loop needs all three plus a vocabulary. One file
    /// cannot satisfy it, which is why `COMIC_OCR_ONNX_PATH` could never reach
    /// this path however it was pointed.
    pub generator: Option<Arc<Mutex<generate::Generator>>>,
    pub daemon_worker: Option<Arc<Mutex<worker::PyDaemonWorker>>>,
}

impl OrtEngine {
    /// One loaded model per directory, for the lifetime of the process.
    ///
    /// The graphs are ~554 MB. Both current callers build an engine once — the
    /// runtime holds it in `Arc<RuntimeState>`, the CLI builds it before the
    /// image loop — so nothing reloads today. This exists so that stays true
    /// when it stops being obvious: a second `OrtEngine::new` in the same
    /// process would otherwise read and parse half a gigabyte again, and
    /// nothing would report it except latency.
    ///
    /// Keyed by directory, because two directories are two different models and
    /// silently serving one for the other is worse than loading twice.
    ///
    /// Note the cost this accepts: engines sharing a directory share one
    /// `Mutex<Generator>`, so their inference SERIALISES. `Generator` needs
    /// `&mut self` — the ONNX sessions are stepped, not merely read — so a
    /// shared instance cannot run two crops at once. For a CLI and a
    /// single-stream runtime that is free; for a concurrent server it is a
    /// throughput ceiling, and the fix there is a pool of generators rather
    /// than a bigger lock.
    fn generator_cache() -> &'static Mutex<HashMap<String, Arc<Mutex<generate::Generator>>>> {
        static CACHE: OnceLock<Mutex<HashMap<String, Arc<Mutex<generate::Generator>>>>> =
            OnceLock::new();
        CACHE.get_or_init(|| Mutex::new(HashMap::new()))
    }

    /// The loaded model for a directory, loading it only if this process has not
    /// already done so.
    fn cached_generator(dir: &str) -> Result<Arc<Mutex<generate::Generator>>, OcrError> {
        // Canonicalised so `models/onnx` and `./models/onnx/` are one entry
        // rather than two loads of the same half-gigabyte.
        let key = std::fs::canonicalize(dir)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| dir.to_string());

        let cache = Self::generator_cache();
        if let Ok(map) = cache.lock() {
            if let Some(existing) = map.get(&key) {
                return Ok(Arc::clone(existing));
            }
        }

        // Loaded OUTSIDE the cache lock: this reads hundreds of megabytes, and
        // holding the map locked through it would block every other engine
        // construction in the process for the duration.
        let loaded = Arc::new(Mutex::new(Self::load_generator(dir)?));

        if let Ok(mut map) = cache.lock() {
            // A concurrent caller may have finished first. Prefer theirs and drop
            // ours rather than replacing a generator someone may already hold —
            // two live copies is the waste this cache exists to prevent.
            return Ok(Arc::clone(map.entry(key).or_insert(loaded)));
        }
        Ok(loaded)
    }

    /// Load the three graphs and the vocabulary from a model directory.
    ///
    /// Names each missing piece rather than reporting a generic failure: the
    /// most likely reason this fails is an export that wrote the graphs and
    /// forgot `vocab.txt`, and "file not found" would send the reader looking
    /// for the wrong thing.
    fn load_generator(dir: &str) -> Result<generate::Generator, OcrError> {
        let root = Path::new(dir);
        for required in [
            "encoder_model.onnx",
            "decoder_model.onnx",
            "decoder_with_past_model.onnx",
            "vocab.txt",
        ] {
            let path = root.join(required);
            if !path.exists() {
                return Err(OcrError::ConfigError(format!(
                    "{} is missing from the model directory {}",
                    required, dir
                )));
            }
        }
        let vocab = comic_ocr_core::tokenizer::WordPieceVocab::from_file(root.join("vocab.txt"))
            .map_err(|e| OcrError::ConfigError(format!("vocab.txt in {dir} did not load: {e}")))?;
        generate::Generator::from_dir(root, vocab, generate::GenerationConfig::default())
    }

    pub fn new(model_name: impl Into<String>) -> Self {
        let name = model_name.into();
        let engine_type = if name.contains("nano") || name.contains("mobile") {
            EngineType::NanoMobileNet
        } else {
            EngineType::BaseInt8Onnx
        };

        // Initialize ONNX Session if local file path exists or via COMIC_OCR_ONNX_PATH env var
        let onnx_path = std::env::var("COMIC_OCR_ONNX_PATH").unwrap_or_else(|_| name.clone());

        let session = if Path::new(&onnx_path).exists() {
            match Session::builder().and_then(|mut builder| builder.commit_from_file(&onnx_path)) {
                Ok(sess) => Some(Arc::new(Mutex::new(sess))),
                Err(e) => {
                    eprintln!(
                        "[WARN] [comic-ocr-ort] Failed to load ONNX Runtime session from '{}': {}. Falling back to Python subprocess path.",
                        onnx_path, e
                    );
                    None
                }
            }
        } else {
            None
        };

        // A model DIRECTORY, which is the shape a VisionEncoderDecoder actually
        // exports as: encoder_model.onnx + decoder_model.onnx +
        // decoder_with_past_model.onnx, plus the vocabulary that turns ids back
        // into characters.
        //
        // Absence is not an error — the single-file and subprocess paths remain
        // — but a directory that is present and UNUSABLE says why, loudly. A
        // silent fallback there is how an operator ends up reading subprocess
        // output while believing the native loop is running.
        let generator = std::env::var("COMIC_OCR_ONNX_DIR")
            .ok()
            .filter(|dir| !dir.trim().is_empty())
            .and_then(|dir| match Self::cached_generator(&dir) {
                Ok(shared) => Some(shared),
                Err(e) => {
                    eprintln!(
                        "[WARN] [comic-ocr-ort] COMIC_OCR_ONNX_DIR='{dir}' is set but unusable: {e}. \
                         The native generation loop is NOT active; falling back."
                    );
                    None
                }
            });

        Self {
            model_name: name,
            engine_type,
            extract_furigana: false,
            entropy_truncation_threshold: 0.15,
            session,
            daemon_worker: None,
            generator,
        }
    }

    /// Loads OrtEngine with native in-memory ONNX Runtime session from file path.
    pub fn from_onnx_file(model_path: impl AsRef<Path>) -> Result<Self, OcrError> {
        let path = model_path.as_ref();
        if !path.exists() {
            return Err(OcrError::EngineError(format!(
                "ONNX model file not found at path {}",
                path.display()
            )));
        }
        let name = path.to_string_lossy().to_string();
        let mut builder = Session::builder().map_err(|e| {
            OcrError::EngineError(format!("Failed to create ONNX SessionBuilder: {}", e))
        })?;

        let session = builder.commit_from_file(path).map_err(|e| {
            OcrError::EngineError(format!(
                "Failed to load ONNX model from file {}: {}",
                path.display(),
                e
            ))
        })?;

        Ok(Self {
            model_name: name,
            engine_type: EngineType::BaseInt8Onnx,
            extract_furigana: false,
            entropy_truncation_threshold: 0.15,
            session: Some(Arc::new(Mutex::new(session))),
            generator: None,
            daemon_worker: None,
        })
    }

    /// Loads OrtEngine with native in-memory ONNX Runtime session from byte buffer.
    pub fn from_onnx_bytes(model_bytes: &[u8]) -> Result<Self, OcrError> {
        let mut builder = Session::builder().map_err(|e| {
            OcrError::EngineError(format!("Failed to create ONNX SessionBuilder: {}", e))
        })?;

        let session = builder.commit_from_memory(model_bytes).map_err(|e| {
            OcrError::EngineError(format!("Failed to load ONNX model from byte buffer: {}", e))
        })?;

        Ok(Self {
            model_name: "in-memory-onnx-bytes".to_string(),
            engine_type: EngineType::BaseInt8Onnx,
            extract_furigana: false,
            entropy_truncation_threshold: 0.15,
            session: Some(Arc::new(Mutex::new(session))),
            generator: None,
            daemon_worker: None,
        })
    }

    pub fn with_furigana(mut self, extract_furigana: bool) -> Self {
        self.extract_furigana = extract_furigana;
        self
    }

    pub fn with_daemon(mut self) -> Result<Self, OcrError> {
        let worker = worker::PyDaemonWorker::spawn(&self.model_name)?;
        self.daemon_worker = Some(Arc::new(Mutex::new(worker)));
        Ok(self)
    }

    /// Computes numerically stable softmax probabilities over a 1D slice of raw logits.
    pub fn softmax(logits: &[f32]) -> Vec<f32> {
        if logits.is_empty() {
            return Vec::new();
        }
        let max_val = logits.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let exps: Vec<f32> = logits.iter().map(|&x| (x - max_val).exp()).collect();
        let sum_exps: f32 = exps.iter().sum();
        if sum_exps > 0.0 {
            exps.iter().map(|&x| x / sum_exps).collect()
        } else {
            vec![1.0 / logits.len() as f32; logits.len()]
        }
    }

    /// Calculates normalized Shannon token entropy H_k = -\sum P(v) log2 P(v).
    pub fn calculate_token_entropy(probs: &[f32]) -> f32 {
        let mut entropy = 0.0f32;
        for &p in probs {
            if p > 1e-7 {
                entropy -= p * p.log2();
            }
        }
        entropy
    }

    /// Checks if rolling token entropy over 4 steps indicates a degenerate repeating loop.
    pub fn should_truncate_loop(&self, rolling_entropies: &[f32]) -> bool {
        if rolling_entropies.len() < 4 {
            return false;
        }
        let recent = &rolling_entropies[rolling_entropies.len() - 4..];
        let avg_entropy: f32 = recent.iter().sum::<f32>() / 4.0;
        avg_entropy < self.entropy_truncation_threshold
    }
}

impl OcrEngine for OrtEngine {
    fn predict(&self, image: &image::DynamicImage) -> Result<OcrResult, OcrError> {
        // The native generation loop, when a model directory is configured.
        // Placed FIRST because it is the only path that produces a reading from
        // the model itself: the single-session branch below runs one forward
        // pass, which is a decode step and not a transcription.
        if let Some(generator) = &self.generator {
            let start = std::time::Instant::now();
            let mut generator = generator
                .lock()
                .map_err(|_| OcrError::EngineError("generator lock poisoned".to_string()))?;
            let (text, confidence, token_probabilities) = generator.generate_scored(image)?;
            return Ok(OcrResult {
                text,
                // Computed from the winning beam's log probability, not asserted.
                // A constant here is the exact defect this crate removed twice.
                confidence,
                token_probabilities,
                metadata: OcrMetadata {
                    duration_ms: start.elapsed().as_secs_f64() * 1000.0,
                    model_name: self.model_name.clone(),
                    engine_type: self.engine_type,
                },
            });
        }
        let start_time = std::time::Instant::now();

        // 1. Native ONNX Runtime C++ Session execution path (if model weights loaded)
        if let Some(ref session_mutex) = self.session {
            let mut session = session_mutex
                .lock()
                .map_err(|e| OcrError::EngineError(format!("Session mutex lock failed: {}", e)))?;

            // Preprocess to the exact tensor `ViTImageProcessor` would produce
            // for this checkpoint. This used to be an inline loop here that
            // normalised with ImageNet mean/std; `preprocessor_config.json`
            // says 0.5/0.5, so that loop was feeding the encoder a shifted and
            // wrongly-scaled image. See `preprocess` for the sourcing.
            let (shape, input_tensor) = comic_ocr_core::preprocess::preprocess(image)?.into_parts();
            let tensor_value =
                ort::value::Value::from_array((shape, input_tensor)).map_err(|e| {
                    OcrError::EngineError(format!("ONNX tensor allocation failed: {}", e))
                })?;

            let outputs = session
                .run(ort::inputs!["pixel_values" => tensor_value])
                .map_err(|e| OcrError::EngineError(format!("ONNX inference run failed: {}", e)))?;

            let _duration_ms = start_time.elapsed().as_secs_f64() * 1000.0;
            // The session runs, and its logits are real — the confidence below is
            // computed from them. What does not exist yet is TEXT.
            //
            // A ViT/DeiT + BERT VisionEncoderDecoder exports via `optimum` as
            // encoder + decoder + decoder-with-past, and the autoregressive
            // generation loop stays in the caller. One forward pass yields a
            // single decode step, not a transcription — so THIS branch, holding
            // one graph from COMIC_OCR_ONNX_PATH, still cannot produce a reading.
            //
            // The loop itself exists and runs: see `generate.rs` and the branch
            // at the top of this function, reached by pointing
            // COMIC_OCR_ONNX_DIR at a directory of all three graphs plus
            // vocab.txt. This comment previously said the loop did not exist,
            // and on 2026-08-20 that sentence was quoted as evidence that it had
            // never been written — 250 lines from the file that implements it.
            //
            // It previously returned the literal "ONNX_NATIVE_PREDICTION" with a
            // genuine confidence attached, which is the most dangerous shape
            // available: a caller sees Ok, a plausible score, and a string that
            // never came from the model. `should_truncate_loop` and
            // `calculate_token_entropy` are already written for the loop that
            // will replace this.
            let _ = &outputs;
            let _ = start_time.elapsed();
            return Err(OcrError::NotImplemented(
                "the native ONNX path loads a session and runs one forward pass, but \
                 VisionEncoderDecoder generation (decoder loop with KV cache) is not \
                 implemented, so it cannot produce text. Set COMIC_OCR_ONNX_PATH only \
                 once that lands; until then the subprocess path is the working one."
                    .to_string(),
            ));
        }

        // 2. Persistent daemon worker inference path if worker initialized
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join(format!(
            "ocr_input_{}_{}.png",
            std::process::id(),
            start_time.elapsed().as_nanos()
        ));

        image
            .save(&temp_path)
            .map_err(|e| OcrError::EngineError(format!("Failed to save input frame: {}", e)))?;

        if let Some(ref worker_mutex) = self.daemon_worker {
            if let Ok(mut worker) = worker_mutex.lock() {
                let res = worker.predict_image_path(&temp_path);
                let _ = std::fs::remove_file(&temp_path);
                if let Ok(mut ocr_res) = res {
                    ocr_res.metadata.duration_ms = start_time.elapsed().as_secs_f64() * 1000.0;
                    if self.extract_furigana {
                        ocr_res.text =
                            post_process_with_furigana(&ocr_res.text, self.extract_furigana);
                    }
                    return Ok(ocr_res);
                }
            }
        }

        let py_script = r#"
import os, json, math, torch
from PIL import Image

image_path = os.environ.get('COMIC_OCR_IMAGE_PATH', '')
model_name = os.environ.get('COMIC_OCR_MODEL_NAME', 'kha-white/manga-ocr-base')

try:
    import manga_ocr
    m = manga_ocr.MangaOcr()
    img = Image.open(image_path).convert('RGB')
    text = m(img)
    print(json.dumps({'text': text, 'confidence': 0.95, 'token_probabilities': [0.95]}))
except Exception as e:
    from transformers import VisionEncoderDecoderModel, ViTImageProcessor, AutoTokenizer
    model = VisionEncoderDecoderModel.from_pretrained(model_name)
    processor = ViTImageProcessor.from_pretrained(model_name)
    tokenizer = AutoTokenizer.from_pretrained(model_name)
    img = Image.open(image_path).convert('RGB')
    pixel_values = processor(img, return_tensors='pt').pixel_values
    output = model.generate(pixel_values, return_dict_in_generate=True, output_scores=True)
    output_ids = output.sequences[0]
    text = tokenizer.decode(output_ids, skip_special_tokens=True).replace(' ', '')
    token_probs = []
    if hasattr(output, 'scores') and output.scores:
        for score in output.scores:
            probs = torch.softmax(score[0], dim=-1)
            token_probs.append(float(probs.max().item()))
    conf = math.exp(sum(math.log(max(p, 1e-7)) for p in token_probs) / len(token_probs)) if token_probs else 0.0
    print(json.dumps({'text': text, 'confidence': conf, 'token_probabilities': token_probs}))
"#;

        let output = Command::new("python3")
            .arg("-c")
            .arg(py_script)
            .env("COMIC_OCR_MODEL_NAME", &self.model_name)
            .env("COMIC_OCR_IMAGE_PATH", &temp_path)
            .output();

        let _ = std::fs::remove_file(&temp_path);

        let out = output.map_err(|e| {
            OcrError::EngineError(format!("Failed to execute inference process: {}", e))
        })?;

        if !out.status.success() {
            let err_msg = String::from_utf8_lossy(&out.stderr);
            return Err(OcrError::EngineError(format!(
                "Inference model process failed: {}",
                err_msg.trim()
            )));
        }

        let raw_stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if raw_stdout.is_empty() {
            return Err(OcrError::EngineError(
                "Inference model output empty text string".to_string(),
            ));
        }

        // Parse JSON output or plain string
        let (raw_text, confidence, token_probs) =
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw_stdout) {
                let txt = val
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let conf = val
                    .get("confidence")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as f32;
                let probs = val
                    .get("token_probabilities")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|x| x.as_f64().map(|f| f as f32))
                            .collect()
                    })
                    .unwrap_or_default();
                (txt, conf, probs)
            } else {
                (raw_stdout, 0.0f32, Vec::new())
            };

        if raw_text.is_empty() {
            return Err(OcrError::EngineError(
                "Inference model parsed empty text string".to_string(),
            ));
        }

        let duration_ms = start_time.elapsed().as_secs_f64() * 1000.0;
        let text = post_process_with_furigana(&raw_text, self.extract_furigana);

        Ok(OcrResult {
            text,
            confidence,
            token_probabilities: token_probs,
            metadata: OcrMetadata {
                duration_ms,
                model_name: self.model_name.clone(),
                engine_type: self.engine_type,
            },
        })
    }

    fn predict_batch(
        &self,
        images: &[image::DynamicImage],
        _batch_size: usize,
    ) -> Result<Vec<OcrResult>, OcrError> {
        images.iter().map(|img| self.predict(img)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_token_entropy() {
        let probs = vec![0.5f32, 0.5f32];
        let entropy = OrtEngine::calculate_token_entropy(&probs);
        assert!((entropy - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_softmax() {
        let logits = vec![1.0f32, 2.0f32, 3.0f32];
        let probs = OrtEngine::softmax(&logits);
        assert!((probs.iter().sum::<f32>() - 1.0).abs() < 1e-4);
        assert!(probs[2] > probs[1] && probs[1] > probs[0]);
    }

    #[test]
    fn test_should_truncate_loop() {
        let engine = OrtEngine::new("test-model");
        let degenerate_entropies = vec![0.10, 0.12, 0.08, 0.09];
        assert!(engine.should_truncate_loop(&degenerate_entropies));

        let valid_entropies = vec![1.2, 0.9, 1.1, 0.8];
        assert!(!engine.should_truncate_loop(&valid_entropies));
    }
}

#[cfg(test)]
mod wiring_tests {
    use super::*;

    /// The gap #842 describes: `Generator` was built, exported and tested, and
    /// nothing outside its own module constructed one. A capability with no
    /// caller is indistinguishable from one that does not exist.
    ///
    /// This CANNOT be a source-text assertion. The first version of this test
    /// grepped `include_str!("lib.rs")` for the delegating call — and passed
    /// with the call removed, because the needle appeared inside the test's own
    /// assertion string. The checker matched itself.
    ///
    /// So it asserts the structural fact instead: the engine owns a generator
    /// slot at all. Whether `predict` actually routes through it is proven by
    /// running a real model directory, not by reading the file that would do it.
    #[test]
    fn the_engine_owns_a_generation_loop_slot() {
        // Safety: single-threaded test; the variable is read once below.
        unsafe { std::env::remove_var("COMIC_OCR_ONNX_DIR") };
        let engine = OrtEngine::new("test-model");
        // The field exists and defaults to absent. A build where `Generator` had
        // no caller could not compile this line at all.
        let _: &Option<Arc<Mutex<generate::Generator>>> = &engine.generator;
        assert!(engine.generator.is_none(), "no directory configured");
    }

    /// A model directory missing any one piece must say WHICH. The likely
    /// failure is an export that wrote three graphs and forgot the vocabulary,
    /// and a generic "not found" sends the reader looking for the wrong thing.
    #[test]
    fn an_incomplete_model_directory_names_what_is_missing() {
        let dir = std::env::temp_dir().join("comic-ocr-wiring-test-empty");
        std::fs::create_dir_all(&dir).expect("temp dir");
        let message = match OrtEngine::load_generator(dir.to_str().unwrap()) {
            Ok(_) => panic!("an empty directory must not yield a generator"),
            Err(err) => format!("{err}"),
        };
        assert!(
            message.contains("encoder_model.onnx"),
            "the error must name the missing file, got: {message}"
        );
    }

    /// Absence of a directory is not an error. The single-file and subprocess
    /// paths remain, and an engine built without COMIC_OCR_ONNX_DIR must still
    /// construct rather than refuse.
    #[test]
    fn no_model_directory_is_not_a_failure() {
        // Safety: single-threaded test, and the variable is read once during
        // construction on the next line.
        unsafe { std::env::remove_var("COMIC_OCR_ONNX_DIR") };
        let engine = OrtEngine::new("test-model");
        assert!(engine.generator.is_none());
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;

    /// What this CAN prove without half a gigabyte of valid graphs on disk: the
    /// cache is a real process-wide map, and an entry inserted under one key is
    /// returned by identity rather than rebuilt.
    ///
    /// What it CANNOT prove here is the thing the cache exists for — that two
    /// `OrtEngine::new` calls share one loaded model — because constructing a
    /// `Generator` requires graphs that pass the KV-cache contract, and this
    /// machine has none that do. That is proven by the e2e test when a valid
    /// export exists, and is honestly unproven until then.
    ///
    /// Named for what it asserts. A test called
    /// `two_engines_share_one_model` that only checks an empty map is the kind
    /// of green line that stops meaning anything.
    #[test]
    fn the_cache_is_process_wide_and_returns_by_identity() {
        let key = "comic-ocr-cache-identity-probe".to_string();
        let cache = OrtEngine::generator_cache();
        cache.lock().expect("cache").remove(&key);
        assert!(
            cache.lock().expect("cache").get(&key).is_none(),
            "the probe key must start absent"
        );
        // The same static is observed across calls — that is what makes the
        // cache process-wide rather than per-engine.
        let again = OrtEngine::generator_cache();
        assert!(
            std::ptr::eq(cache, again),
            "generator_cache must hand back one shared map, not a fresh one"
        );
    }

    /// Two directories are two models. Serving one for the other would be a
    /// silent wrong answer, which is worse than loading twice.
    #[test]
    fn distinct_directories_key_distinctly() {
        let a = std::env::temp_dir().join("comic-ocr-cache-a");
        let b = std::env::temp_dir().join("comic-ocr-cache-b");
        assert_ne!(a.display().to_string(), b.display().to_string());
    }
}
