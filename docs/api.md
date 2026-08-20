# Comic OCR Rust: API & Schema Specification

This document provides the canonical specification for the library traits, data schemas, REST API endpoints, CLI parameters, and container contracts of **Comic OCR Rust**.

---

## 1. Rust Core Library API (`comic-ocr-core`)

Located in [`crates/comic-ocr-core`](crates/comic-ocr-core).

### `OcrEngine` Trait Definition

```rust
pub trait OcrEngine: Send + Sync {
    fn predict(&self, image: &image::DynamicImage) -> Result<OcrResult, OcrError>;
    fn predict_batch(
        &self,
        images: &[image::DynamicImage],
        batch_size: usize,
    ) -> Result<Vec<OcrResult>, OcrError>;
}
```

### Data Schemas

#### `OcrResult` (Rust Struct & JSON Schema)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    pub text: String,
    pub confidence: f32,
    pub token_probabilities: Vec<f32>,
    pub metadata: OcrMetadata,
}
```

**JSON Schema Representation**:
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "OcrResult",
  "type": "object",
  "properties": {
    "text": { "type": "string", "description": "Recognized & normalized Japanese text" },
    "confidence": { "type": "number", "minimum": 0.0, "maximum": 1.0, "description": "Geometric mean sequence confidence score" },
    "token_probabilities": {
      "type": "array",
      "items": { "type": "number", "minimum": 0.0, "maximum": 1.0 },
      "description": "Autoregressive token probability sequence"
    },
    "metadata": { "$ref": "#/$defs/OcrMetadata" }
  },
  "required": ["text", "confidence", "token_probabilities", "metadata"]
}
```

#### `OcrMetadata` & `EngineType`

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EngineType {
    BaseInt8Onnx,
    NanoMobileNet,
    Fallback,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrMetadata {
    pub duration_ms: f64,
    pub model_name: String,
    pub engine_type: EngineType,
}
```

---

### Japanese Post-Processing Algorithm (`post_process`)

The `post_process(input: &str) -> String` function cleans OCR output tokens:

1. **Ellipsis Normalization**: Replaces variants `…` with `...` (or full-width `．．．`).
2. **ASCII Full-Width Conversion (jaconv h2z)**: Converts printable ASCII range `!` (`0x21`) through `~` (`0x7E`) to Japanese full-width Unicode characters (`0xFF01` through `0xFF5E`).
3. **Space Conversion**: Converts half-width space `' '` (`0x20`) to Japanese full-width ideographic space `'　'` (`0x3000`).

```rust
use comic_ocr_core::post_process;

assert_eq!(post_process("…"), "．．．");
assert_eq!(post_process("テスト 123"), "テスト　１２３");
```

---

## 2. Polymorphic Decision Protocol API (`comic-ocr-pdp`)

Located in [`crates/comic-ocr-pdp`](crates/comic-ocr-pdp).

### `PanelEvaluator` Struct

```rust
pub struct PanelEvaluator {
    engines: Vec<Box<dyn OcrEngine>>,
    invalidation_threshold: f32,
}

impl PanelEvaluator {
    pub fn new(engines: Vec<Box<dyn OcrEngine>>, invalidation_threshold: f32) -> Self;
    pub fn evaluate(&self, image: &image::DynamicImage) -> Result<PdpDecision, PdpError>;
}
```

### `PdpDecision` Schema

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdpDecision {
    pub selected_text: String,
    pub confidence: f32,
    pub is_validated: bool,
    pub candidates: Vec<OcrResult>,
}
```

---

## 3. Reflective Runtime Service REST API (`comic-ocr-runtime`)

Located in [`crates/comic-ocr-runtime`](crates/comic-ocr-runtime).

### Environment Configuration Schema (`RuntimeConfig`)

| Environment Variable | Default Value | Description |
| :--- | :--- | :--- |
| `MANGA_OCR_HOST` | `"0.0.0.0"` | Network bind address |
| `MANGA_OCR_PORT` | `8000` | HTTP TCP listening port |
| `MANGA_OCR_MODEL` | `"kha-white/comic-ocr-base"` | Target ONNX model identifier |
| `MANGA_OCR_FORCE_CPU` | `false` | Force CPU execution provider |
| `RUST_LOG` | `"info"` | Tracing log level (`debug`, `info`, `warn`) |

---

### REST API Endpoints Table

| Method | Endpoint | Payload | HTTP Status | Description |
| :--- | :--- | :--- | :--- | :--- |
| `GET` | `/v1/runtime/health` | None | `200 OK` | Telemetry request counters & uptime |
| `GET` | `/v1/runtime/info` | None | `200 OK` | Reflective CSG model & platform metadata |
| `POST` | `/v1/ocr/predict` | Multipart (`image` or `file`) | `200 OK`, `400 Bad Request` | Single crop image OCR prediction |
| `POST` | `/v1/ocr/eval_panel` | Multipart (`image` or `file`) | `200 OK`, `400 Bad Request` | PDP multi-engine candidate selection |

---

### Endpoint Payloads & Examples

#### 1. `GET /v1/runtime/health`

**cURL Request**:
```bash
curl -s http://localhost:8000/v1/runtime/health
```

**JSON Response Payload**:
```json
{
  "service": "comic-ocr-runtime",
  "status": "ok",
  "version": "0.2.0",
  "uptime_secs": 42,
  "metrics": {
    "total_requests": 15,
    "total_successful": 15,
    "total_failed": 0
  }
}
```

#### 2. `GET /v1/runtime/info`

**cURL Request**:
```bash
curl -s http://localhost:8000/v1/runtime/info
```

**JSON Response Payload**:
```json
{
  "runtime": "Comic OCR Reflective Runtime",
  "model_name": "kha-white/comic-ocr-base",
  "max_batch_size": 16,
  "pdp_invalidation_threshold": 0.7,
  "force_cpu": false,
  "target_architecture": "aarch64",
  "os": "macos"
}
```

#### 3. `POST /v1/ocr/predict`

**cURL Request**:
```bash
curl -s -F "image=@assets/examples/00.jpg" http://localhost:8000/v1/ocr/predict
```

**JSON Response Payload**:
```json
{
  "text": "．．．",
  "confidence": 0.985,
  "duration_ms": 4.20
}
```

#### 4. `POST /v1/ocr/eval_panel`

**cURL Request**:
```bash
curl -s -F "image=@assets/examples/00.jpg" http://localhost:8000/v1/ocr/eval_panel
```

**JSON Response Payload**:
```json
{
  "selected_text": "．．．",
  "confidence": 0.985,
  "is_validated": true,
  "candidates_count": 1
}
```

---

## 4. Command-Line Interface (`comic-ocr-cli`)

Located in [`crates/comic-ocr-cli`](crates/comic-ocr-cli).

```bash
comic-ocr --image <PATH_TO_IMAGE> [FLAGS]
```

### CLI Arguments & Options

```text
Usage: comic-ocr --image <IMAGE> [--force-cpu]

Options:
  -i, --image <IMAGE>      Path to input image file
      --force-cpu          Force CPU execution provider [default: false]
  -h, --help               Print help information
  -V, --version            Print version information
```

### CLI Invocation Example

```bash
cargo run --release -p comic-ocr-cli -- --image assets/examples/00.jpg
```

---

## 5. Docker Container Deployment

The multi-stage release `Dockerfile` builds a lightweight Debian bookworm-slim container running `comic-ocr-runtime`:

```dockerfile
# Build image
docker build -t comic-ocr-runtime:v0.2.0 .

# Run container
docker run -d -p 8000:8000 --name manga-runtime comic-ocr-runtime:v0.2.0
```
