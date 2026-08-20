# ONNX Graph Contract & Export Specification

This document specifies the exact ONNX graph contract, tensor naming conventions, and export procedures required by [`crates/comic-ocr-ort/src/generate.rs`](file:///Users/zachshallbetter/Projects/comic-ocr-rust/crates/comic-ocr-ort/src/generate.rs) to run native Rust autoregressive VisionTransformer decoding.

---

## 1. Required ONNX Graph Layout

The ONNX export for HuggingFace `VisionEncoderDecoderModel` checkpoints (e.g. `kha-white/manga-ocr`) must produce three ONNX model files inside a target directory (default `models/onnx/`):

| Model File | Purpose | Input Tensors | Output Tensors |
| :--- | :--- | :--- | :--- |
| **`encoder_model.onnx`** | Vision Transformer (ViT) patch image encoder | `pixel_values`: `[1, 3, 224, 224]` (float32) | `last_hidden_state`: `[1, seq_len, hidden_size]` (float32) |
| **`decoder_model.onnx`** | Transformer decoder prefill step (Step 0) | `input_ids`: `[1, 1]` (int64)<br/>`encoder_hidden_states`: `[1, seq_len, hidden_size]` | `logits`: `[1, 1, vocab_size]` (float32)<br/>`present.N.self.key/value`: KV-cache<br/>`present.N.cross.key/value`: KV-cache |
| **`decoder_with_past_model.onnx`** | Autoregressive KV-cache step (Steps $1..N$) | `input_ids`: `[1, 1]` (int64)<br/>`encoder_hidden_states`: `[1, seq_len, hidden_size]`<br/>`past_key_values.N.self/cross.key/value` | `logits`: `[1, 1, vocab_size]` (float32)<br/>`present.N.decoder.self.key/value` |

---

## 2. Key Graph Properties & KV-Cache Rules

1. **Cross-Attention Cache Persistence**:
   - The **cross-attention cache is emitted once** during prefill (`decoder_model.onnx`) and carried forward unchanged across all subsequent decoding steps.
   - `decoder_with_past_model.onnx` returns only self-attention updates (`present.N.decoder.self.key/value`).
2. **Dynamic Tensor Inference (`GraphContract`)**:
   - `generate.rs` derives the number of layers, head counts, and hidden dimensions directly from ONNX graph input/output tensor names (`present.N.*` / `past_key_values.N.*`) using `comic_ocr_core::decode::GraphContract`.
3. **Loop Truncation & No-Repeat Trigram**:
   - Beam search candidate selection enforces `banned_by_no_repeat_ngram` ($N=3$) and truncates generation when 4-step rolling token entropy drops below $\bar{H} < 0.15$.

---

## 3. Export Command & Pipeline

To generate the required ONNX models from a HuggingFace checkpoint, execute:

```bash
python3 scripts/export_onnx.py --model kha-white/manga-ocr --output-dir models/onnx
```

Or via Optimum CLI:

```bash
optimum-cli export onnx --model kha-white/manga-ocr --task vision-encoder-decoder-submodels models/onnx/
```
