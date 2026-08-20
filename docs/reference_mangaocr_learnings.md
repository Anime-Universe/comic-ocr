# Reference ComicOCR Analysis & Architectural Learnings

This document summarizes key takeaways, model footprint optimizations, dataset insights, and edge-case mitigations derived from analyzing the reference project [`_Reference-Projects/ComicOCR`](reference/ComicOCR).

---

## 1. Key Insights & Benchmark Comparison

| Dimension | Standard `the reference checkpoint` | Reference `ComicOCR` (PaddleOCR/TrOCR) | Our Rust Migration Target (`comic-ocr-rust`) |
| :--- | :--- | :--- | :--- |
| **Model Size** | 444 MB | **~8 MB** (55x smaller) | **8 MB (Nano) / 120 MB (Base INT8)** |
| **Character Error Rate (CER)** | 2.5% – 5.0% | ~14.4% | **2.5% (Base) / <8% (Nano)** |
| **Backbone Architecture** | ViT + Japanese BERT | MobileNetV3 / TrOCR-small | **ONNX Runtime (ViT/CNN + BERT/CTC)** |
| **Dataset Source** | Synthetic HTML | `Manga109-s` + `CC-100` Corpus | **Synthetic + Manga109-s Mix** |

---

## 2. Key Learnings & Technical Takeaways

### Learning 1: Ultra-Lightweight ~8MB "Nano" Model Target
- **Takeaway**: PyTorch ViT+BERT models require ~444MB, which creates heavy memory pressure. The reference project demonstrates that an ~8MB lightweight MobileNetV3/TrOCR model is viable.
- **Application to Rust Migration**: Provide dual model weight profiles inside `comic-ocr-ort`:
  - `comic-ocr-base` (INT8 Quantized ONNX, ~120 MB): High accuracy (~2.5% CER) for desktop/servers.
  - `comic-ocr-nano` (Quantized MobileNetV3/CTC, ~8 MB): Ultra-fast inference for mobile and low-RAM devices.

### Learning 2: Mitigating the Long Sequence Attention Trap (>100 characters)
- **Takeaway**: The reference author identified that for unusually long text blocks (>100 characters), auto-regressive attention degrades after 25–50% of the sequence, producing repetitive character hallucinations.
- **Application to Rust Migration**: Implement **sliding-window sequence truncation** and vertical aspect-ratio line-splitting in `comic-ocr-core` prior to feeding crops into the decoder.

### Learning 3: Training Corpus Mix (`Manga109-s` + `CC-100`)
- **Takeaway**: Relying purely on synthetic text fonts limits vocabulary richness. Blending real manga panel crops from `Manga109-s` with natural Japanese web text from `CC-100` improves Kanji coverage and conversational dialogue recognition.
- **Application to Rust Migration**: Expand `comic_ocr_dev` synthetic data generation scripts to ingest `CC-100` Japanese text seeds.

### Learning 4: Pluggable Backend Architecture (ViT vs CNN+CTC)
- **Takeaway**: Hardcoding a single model architecture restricts optimization options across different target platforms.
- **Application to Rust Migration**: In `comic-ocr-core`, define a generic `OcrEngine` Rust trait allowing transparent switching between Transformer VisionEncoderDecoder and CNN+CTC backbones.
