# Comic OCR Rust: API & Schema Specification

This document provides the canonical specification for the library traits, data schemas, REST API endpoints, CLI parameters, and container contracts of **Comic OCR Rust**.

---

## 0. JSON Schemas

Formal JSON Schema Draft-07 contracts are maintained in [`schemas/`](schemas/):

- [`schemas/ocr_result.json`](schemas/ocr_result.json): Authoritative payload schema for single text crop OCR predictions.
- [`schemas/page_result.json`](schemas/page_result.json): Authoritative payload schema for full-page OCR with 2-Level topological reading order panel graphs.
- [`schemas/pdp_decision.json`](schemas/pdp_decision.json): Authoritative payload schema for Polymorphic Decision Protocol (PDP) multi-engine panel evaluation.

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

## 5. Scene Graph REST API Endpoints (`comic-ocr-runtime`)

### `POST /v1/scene/compile`

Compiles a full authoring `MangaDocument` scene graph into a compact list of `LocalizedTextObject` runtime payloads.

- **Request Header**: `Content-Type: application/json`
- **Request Body**: `MangaDocument` JSON payload (conforming to `schemas/comic_scene_graph.json`).
- **Response**: Array of `LocalizedTextObject` JSON objects (conforming to `schemas/localized_text_object.json`).

```bash
curl -X POST "http://127.0.0.1:8000/v1/scene/compile" \
  -H "Content-Type: application/json" \
  -d @document_scene.json
```

---

### `POST /v1/scene/validate`

Validates a comic page scene graph against collision, overflow, face-obstruction, and reading order constraints.

- **Request Header**: `Content-Type: application/json`
- **Response**:
```json
{
  "status": "valid",
  "checked_at": "2026-08-20T00:40:00Z",
  "issues_count": 0,
  "issues": []
}
```

---

## 6. Docker Deployment & Containerization

The multi-stage release `Dockerfile` builds a lightweight Debian bookworm-slim container running `comic-ocr-runtime`:

```dockerfile
# Build image
docker build -t comic-ocr-runtime:v0.2.0 .

# Run container
docker run -d -p 8000:8000 --name manga-runtime comic-ocr-runtime:v0.2.0
```

---

## 7. Benchmark Ground Truth Dataset (`tests/data/expected_results.json`)

The test suite includes a canonical benchmark dataset located at [`tests/data/expected_results.json`](file:///Users/zachshallbetter/Projects/comic-ocr-rust/tests/data/expected_results.json) alongside baseline crop images in `tests/data/images/`.

### Expected Results Schema & Sample Pairs

```json
[
  {
    "filename": "00.jpg",
    "result": "素直にあやまるしか"
  },
  {
    "filename": "01.jpg",
    "result": "立川で見た〝穴〟の下の巨大な眼は："
  },
  {
    "filename": "02.jpg",
    "result": "実戦剣術も一流です"
  },
  {
    "filename": "03.jpg",
    "result": "第３０話重苦しい闇の奥で静かに呼吸づきながら"
  },
  {
    "filename": "04.jpg",
    "result": "きのうハンパーヶとって、ゴメン！！！"
  },
  {
    "filename": "05.jpg",
    "result": "ぎゃっ"
  },
  {
    "filename": "06.jpg",
    "result": "ピンポーーン"
  },
  {
    "filename": "07.jpg",
    "result": "ＬＩＮＫ！私達７人の力でガノンの塔の結界をやぶります"
  },
  {
    "filename": "08.jpg",
    "result": "ファイアパンチ"
  },
  {
    "filename": "09.jpg",
    "result": "少し黙っている"
  },
  {
    "filename": "10.jpg",
    "result": "わかるかな〜？"
  },
  {
    "filename": "11.jpg",
    "result": "警察にも先生にも町中の人達に！！"
  }
]
```

### Programmatic Usage in Rust Tests

```rust
use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
struct GroundTruthPair {
    filename: String,
    result: String,
}

#[test]
fn test_expected_results_dataset() {
    let json_str = fs::read_to_string("tests/data/expected_results.json")
        .expect("Failed to read expected_results.json");
    let pairs: Vec<GroundTruthPair> = serde_json::from_str(&json_str)
        .expect("Failed to parse expected_results.json");

    assert_eq!(pairs.len(), 12);
    assert_eq!(pairs[0].filename, "00.jpg");
    assert_eq!(pairs[0].result, "素直にあやまるしか");
}
```
