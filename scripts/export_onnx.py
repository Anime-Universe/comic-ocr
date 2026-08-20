#!/usr/bin/env python3
"""
ONNX Model Exporter for Manga & Comic OCR Rust

Exports HuggingFace VisionEncoderDecoderModel checkpoints (e.g., `kha-white/manga-ocr`)
into the required 3-graph ONNX layout (`encoder_model.onnx`, `decoder_model.onnx`,
`decoder_with_past_model.onnx`) consumed by `crates/comic-ocr-ort/src/generate.rs`.

Usage:
    python3 scripts/export_onnx.py --model kha-white/manga-ocr --output-dir models/onnx
"""

import argparse
import os
import sys

def main():
    parser = argparse.ArgumentParser(description="Export VisionEncoderDecoder model to ONNX graphs.")
    parser.add_argument(
        "--model",
        type=str,
        default=os.environ.get("COMIC_OCR_MODEL", "kha-white/manga-ocr"),
        help="HuggingFace model repository ID or local path (default: kha-white/manga-ocr).",
    )
    parser.add_argument(
        "--output-dir",
        type=str,
        default="models/onnx",
        help="Directory to save exported ONNX models (default: models/onnx).",
    )
    args = parser.parse_args()

    os.makedirs(args.output_dir, exist_ok=True)
    print(f"[*] Exporting ONNX model for '{args.model}' to '{args.output_dir}'...")

    try:
        from optimum.onnxruntime import ORTModelForVision2Seq
        model = ORTModelForVision2Seq.from_pretrained(args.model, export=True)
        model.save_pretrained(args.output_dir)
        print(f"[SUCCESS] ONNX models successfully exported to {args.output_dir}:")
        for fname in ["encoder_model.onnx", "decoder_model.onnx", "decoder_with_past_model.onnx"]:
            path = os.path.join(args.output_dir, fname)
            exists = os.path.exists(path)
            status = "FOUND" if exists else "MISSING"
            print(f"  - {fname}: {status}")
    except ImportError:
        print("[INFO] `optimum` library not found. Attempting `optimum-cli` fallback...")
        cmd = f"optimum-cli export onnx --model {args.model} --task vision-encoder-decoder-submodels {args.output_dir}"
        ret = os.system(cmd)
        if ret != 0:
            print(f"[ERROR] Failed to export ONNX model. Please install optimum: `pip install optimum[onnxruntime]`.")
            sys.exit(1)

if __name__ == "__main__":
    main()
