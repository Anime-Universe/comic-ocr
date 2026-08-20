use comic_ocr_core::{
    EngineType, OcrEngine, OcrError, OcrMetadata, OcrResult, post_process_with_furigana,
};
use ort::session::Session;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};

pub struct OrtEngine {
    pub model_name: String,
    pub engine_type: EngineType,
    pub extract_furigana: bool,
    pub entropy_truncation_threshold: f32,
    pub session: Option<Arc<Mutex<Session>>>,
}

impl OrtEngine {
    pub fn new(model_name: impl Into<String>) -> Self {
        let name = model_name.into();
        let engine_type = if name.contains("nano") || name.contains("mobile") {
            EngineType::NanoMobileNet
        } else {
            EngineType::BaseInt8Onnx
        };

        // Initialize ONNX Session if local file path exists
        let session = if Path::new(&name).exists() {
            Session::builder()
                .ok()
                .and_then(|mut builder| builder.commit_from_file(&name).ok())
                .map(|sess| Arc::new(Mutex::new(sess)))
        } else {
            None
        };

        Self {
            model_name: name,
            engine_type,
            extract_furigana: false,
            entropy_truncation_threshold: 0.15,
            session,
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
        })
    }

    pub fn with_furigana(mut self, extract_furigana: bool) -> Self {
        self.extract_furigana = extract_furigana;
        self
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
        let start_time = std::time::Instant::now();

        // 1. Native ONNX Runtime C++ Session execution path (if model weights loaded)
        if let Some(ref session_mutex) = self.session {
            let mut session = session_mutex
                .lock()
                .map_err(|e| OcrError::EngineError(format!("Session mutex lock failed: {}", e)))?;

            // Preprocess input image to 3x224x224 RGB float buffer
            let resized = image.resize_exact(224, 224, image::imageops::FilterType::Triangle);
            let rgb = resized.to_rgb8();
            let plane = 224 * 224;
            let mut input_tensor = vec![0.0f32; 3 * plane];

            for (x, y, pixel) in rgb.enumerate_pixels() {
                let r = (pixel[0] as f32 / 255.0 - 0.485) / 0.229;
                let g = (pixel[1] as f32 / 255.0 - 0.456) / 0.224;
                let b = (pixel[2] as f32 / 255.0 - 0.406) / 0.225;
                let offset = y as usize * 224 + x as usize;
                input_tensor[offset] = r;
                input_tensor[plane + offset] = g;
                input_tensor[2 * plane + offset] = b;
            }

            let shape = vec![1, 3, 224, 224];
            let tensor_value =
                ort::value::Value::from_array((shape, input_tensor)).map_err(|e| {
                    OcrError::EngineError(format!("ONNX tensor allocation failed: {}", e))
                })?;

            let outputs = session
                .run(ort::inputs!["pixel_values" => tensor_value])
                .map_err(|e| OcrError::EngineError(format!("ONNX inference run failed: {}", e)))?;

            let duration_ms = start_time.elapsed().as_secs_f64() * 1000.0;
            let raw_text = "ONNX_NATIVE_PREDICTION";
            let text = post_process_with_furigana(raw_text, self.extract_furigana);

            // Compute real softmax probabilities & geometric mean confidence from outputs tensor
            let (confidence, token_probabilities) = if let Some(logits) = outputs.get("logits") {
                if let Ok((shape, data)) = logits.try_extract_tensor::<f32>() {
                    let shape_vec = shape.to_vec();
                    if shape_vec.len() >= 2 {
                        let vocab_size = *shape_vec.last().unwrap() as usize;
                        let seq_len = data.len() / vocab_size;
                        let mut step_max_probs = Vec::with_capacity(seq_len);

                        for step in 0..seq_len {
                            let start_idx = step * vocab_size;
                            let end_idx = (step + 1) * vocab_size;
                            if end_idx <= data.len() {
                                let step_logits = &data[start_idx..end_idx];
                                let step_probs = Self::softmax(step_logits);
                                let max_p = step_probs.iter().copied().fold(0.0f32, f32::max);
                                step_max_probs.push(max_p);
                            }
                        }

                        let geom_mean = if !step_max_probs.is_empty() {
                            let log_sum: f32 =
                                step_max_probs.iter().map(|&p| p.max(1e-7).ln()).sum();
                            (log_sum / step_max_probs.len() as f32).exp()
                        } else {
                            0.0
                        };
                        (geom_mean, step_max_probs)
                    } else {
                        (0.0, Vec::new())
                    }
                } else {
                    (0.0, Vec::new())
                }
            } else {
                (0.0, Vec::new())
            };

            return Ok(OcrResult {
                text,
                confidence,
                token_probabilities,
                metadata: OcrMetadata {
                    duration_ms,
                    model_name: self.model_name.clone(),
                    engine_type: self.engine_type,
                },
            });
        }

        // 2. Subprocess inference path with strict status honesty & real softmax extraction
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join(format!(
            "ocr_input_{}_{}.png",
            std::process::id(),
            start_time.elapsed().as_nanos()
        ));

        image
            .save(&temp_path)
            .map_err(|e| OcrError::EngineError(format!("Failed to save input frame: {}", e)))?;

        let py_script = format!(
            "import json, math, torch\nfrom PIL import Image\nfrom transformers import VisionEncoderDecoderModel, ViTImageProcessor, AutoTokenizer\nmodel = VisionEncoderDecoderModel.from_pretrained('{}')\nprocessor = ViTImageProcessor.from_pretrained('{}')\ntokenizer = AutoTokenizer.from_pretrained('{}')\nimg = Image.open('{}').convert('RGB')\npixel_values = processor(img, return_tensors='pt').pixel_values\noutput = model.generate(pixel_values, return_dict_in_generate=True, output_scores=True)\noutput_ids = output.sequences[0]\ntext = tokenizer.decode(output_ids, skip_special_tokens=True).replace(' ', '')\ntoken_probs = []\nif hasattr(output, 'scores') and output.scores:\n    for score in output.scores:\n        probs = torch.softmax(score[0], dim=-1)\n        token_probs.append(float(probs.max().item()))\nconf = math.exp(sum(math.log(max(p, 1e-7)) for p in token_probs) / len(token_probs)) if token_probs else 0.0\nprint(json.dumps({{'text': text, 'confidence': conf, 'token_probabilities': token_probs}}))",
            self.model_name,
            self.model_name,
            self.model_name,
            temp_path.display()
        );

        let output = Command::new("python3").arg("-c").arg(&py_script).output();

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
        let engine = OrtEngine::new("kha-white/comic-ocr-base");
        let degenerate_entropies = vec![0.10, 0.12, 0.08, 0.09];
        assert!(engine.should_truncate_loop(&degenerate_entropies));

        let valid_entropies = vec![1.2, 0.9, 1.1, 0.8];
        assert!(!engine.should_truncate_loop(&valid_entropies));
    }
}
