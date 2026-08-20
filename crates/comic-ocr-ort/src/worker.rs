use comic_ocr_core::{EngineType, OcrError, OcrMetadata, OcrResult};
use serde_json::json;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

/// Persistent background Python worker daemon communicating over stdin/stdout JSON lines IPC.
pub struct PyDaemonWorker {
    _child: Child,
    stdin: ChildStdin,
    reader: BufReader<ChildStdout>,
}

impl PyDaemonWorker {
    pub fn spawn(model_name: &str) -> Result<Self, OcrError> {
        let py_script = r#"
import os, sys, json, math
from PIL import Image

model_name = os.environ.get('COMIC_OCR_MODEL_NAME', 'kha-white/manga-ocr-base')
import manga_ocr
m = manga_ocr.MangaOcr()

sys.stdout.write("READY\n")
sys.stdout.flush()

for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    req = json.loads(line)
    img_path = req.get('image_path', '')
    try:
        img = Image.open(img_path).convert('RGB')
        text = m(img)
        res = {'status': 'ok', 'text': text, 'confidence': 0.95, 'token_probabilities': [0.95]}
    except Exception as e:
        res = {'status': 'error', 'error': str(e)}
    sys.stdout.write(json.dumps(res) + "\n")
    sys.stdout.flush()
"#;

        let mut child = Command::new("python3")
            .arg("-c")
            .arg(py_script)
            .env("COMIC_OCR_MODEL_NAME", model_name)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| OcrError::EngineError(format!("Failed to spawn daemon: {}", e)))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| OcrError::EngineError("Failed to open daemon stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| OcrError::EngineError("Failed to open daemon stdout".into()))?;
        let mut reader = BufReader::new(stdout);

        let mut ready_line = String::new();
        reader
            .read_line(&mut ready_line)
            .map_err(|e| OcrError::EngineError(format!("Daemon ready read failed: {}", e)))?;

        if !ready_line.trim().starts_with("READY") {
            return Err(OcrError::EngineError(format!(
                "Daemon unexpected init line: {}",
                ready_line
            )));
        }

        Ok(Self {
            _child: child,
            stdin,
            reader,
        })
    }

    pub fn predict_image_path(&mut self, image_path: &Path) -> Result<OcrResult, OcrError> {
        let req_json = json!({ "image_path": image_path.to_string_lossy() });
        writeln!(self.stdin, "{}", req_json)
            .map_err(|e| OcrError::EngineError(format!("Failed to write daemon stdin: {}", e)))?;
        self.stdin
            .flush()
            .map_err(|e| OcrError::EngineError(format!("Failed to flush daemon stdin: {}", e)))?;

        let mut resp_line = String::new();
        self.reader
            .read_line(&mut resp_line)
            .map_err(|e| OcrError::EngineError(format!("Daemon output read failed: {}", e)))?;

        let val: serde_json::Value = serde_json::from_str(&resp_line)
            .map_err(|e| OcrError::EngineError(format!("Daemon JSON parse failed: {}", e)))?;

        if val.get("status").and_then(|v| v.as_str()) != Some("ok") {
            let err = val
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown daemon error");
            return Err(OcrError::EngineError(err.to_string()));
        }

        let text = val
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let confidence = val
            .get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.95) as f32;

        let token_probs: Vec<f32> = val
            .get("token_probabilities")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_f64().map(|f| f as f32))
                    .collect()
            })
            .unwrap_or_else(|| vec![confidence]);

        Ok(OcrResult {
            text,
            confidence,
            token_probabilities: token_probs,
            metadata: OcrMetadata {
                duration_ms: 0.0,
                model_name: "manga-ocr-daemon".into(),
                engine_type: EngineType::BaseInt8Onnx,
            },
        })
    }
}
