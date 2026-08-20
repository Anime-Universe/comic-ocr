#!/usr/bin/env python3
"""
ONNX Model Exporter for Manga & Comic OCR Rust

Exports HuggingFace VisionEncoderDecoderModel checkpoints (`kha-white/manga-ocr-base`)
into the required ONNX model graphs (`encoder_model.onnx`, `decoder_model.onnx`,
`decoder_with_past_model.onnx`) consumed by `crates/comic-ocr-ort/src/generate.rs`.

Usage:
    python3 scripts/export_onnx.py --output-dir models/onnx
"""

import argparse
import os
import sys
import torch

def export_manga_ocr_onnx(output_dir="models/onnx"):
    import manga_ocr

    os.makedirs(output_dir, exist_ok=True)
    print(f"[*] Loading PyTorch model from `manga-ocr`...")
    m = manga_ocr.MangaOcr()
    model = m.model
    model.cpu()
    model.eval()

    encoder_path = os.path.join(output_dir, "encoder_model.onnx")
    decoder_path = os.path.join(output_dir, "decoder_model.onnx")
    decoder_past_path = os.path.join(output_dir, "decoder_with_past_model.onnx")

    # 1. Export Vision Encoder (ViTModel: pixel_values [1, 3, 224, 224] -> last_hidden_state [1, 197, 768])
    print(f"[*] Exporting Vision Encoder -> {encoder_path}...")
    dummy_pixel_values = torch.randn(1, 3, 224, 224)
    torch.onnx.export(
        model.encoder,
        (dummy_pixel_values,),
        encoder_path,
        input_names=["pixel_values"],
        output_names=["last_hidden_state"],
        dynamic_axes={
            "pixel_values": {0: "batch_size"},
            "last_hidden_state": {0: "batch_size"},
        },
        opset_version=14,
    )

    # 2. Export Causal Decoder (BertLMHeadModel: input_ids [1, seq_len], encoder_hidden_states [1, 197, 768] -> logits [1, seq_len, vocab_size])
    print(f"[*] Exporting Causal Decoder -> {decoder_path}...")
    dummy_input_ids = torch.tensor([[101, 200, 300]], dtype=torch.long)
    dummy_encoder_hidden_states = torch.randn(1, 197, 768)
    
    # The FIRST-PASS decoder must emit the cache it builds, not just logits.
    #
    # Generation runs this graph once over the prompt, then hands its `present.*`
    # to the with-past graph as `past_key_values.*` for every subsequent token.
    # Exporting logits alone leaves the loop with nothing to seed the cache from,
    # which surfaces as "decoder emitted no present.0.decoder.key" at the first
    # step — the loop asking for a cache the graph was never told to return.
    _cfg = model.decoder.config
    _n_layers = _cfg.num_hidden_layers

    def _present_names(n_layers):
        names = []
        for layer in range(n_layers):
            for side in ("decoder", "encoder"):
                for part in ("key", "value"):
                    names.append(f"present.{layer}.{side}.{part}")
        return names

    class DecoderWrapper(torch.nn.Module):
        """Prefill decoder: full prompt in, logits plus the cache it built out."""

        def __init__(self, decoder, n_layers):
            super().__init__()
            self.decoder = decoder
            self.n_layers = n_layers

        def forward(self, input_ids, encoder_hidden_states):
            out = self.decoder(
                input_ids=input_ids,
                encoder_hidden_states=encoder_hidden_states,
                use_cache=True,
                return_dict=True,
            )
            present = []
            for layer in range(self.n_layers):
                s = out.past_key_values.self_attention_cache.layers[layer]
                c = out.past_key_values.cross_attention_cache.layers[layer]
                present.extend([s.keys, s.values, c.keys, c.values])
            return (out.logits, *present)

    decoder_wrapper = DecoderWrapper(model.decoder, _n_layers)
    decoder_wrapper.eval()

    torch.onnx.export(
        decoder_wrapper,
        (dummy_input_ids, dummy_encoder_hidden_states),
        decoder_path,
        input_names=["input_ids", "encoder_hidden_states"],
        output_names=["logits", *_present_names(_n_layers)],
        dynamic_axes={
            "input_ids": {0: "batch_size", 1: "sequence_length"},
            "encoder_hidden_states": {0: "batch_size"},
            "logits": {0: "batch_size", 1: "sequence_length"},
            # Self-attention and cross-attention MUST NOT share an axis name.
            # ONNX treats one dim_param as one size everywhere it appears, so
            # naming both "present_sequence_length" asserts prompt length ==
            # encoder length and the graph fails at run time with
            # "{1,12,64,1} != {1,12,64,197}". They are different dimensions that
            # happen to sit in the same position.
            **{
                name: {
                    0: "batch_size",
                    2: "encoder_sequence_length" if ".encoder." in name else "present_sequence_length",
                }
                for name in _present_names(_n_layers)
            },
        },
        opset_version=14,
    )

    # 3. Export the WITH-PAST decoder: one token in, reusing the cache.
    #
    # This was previously `shutil.copyfile(decoder_path, decoder_past_path)` — a
    # copy, under a comment calling it an "alias". The two files were
    # byte-identical, which is why the Rust loader refused the directory with
    # "no past_key_values.* inputs on the decoder graph": there was no cache
    # graph, so single-token decoding was impossible and the generation loop
    # could never run.
    #
    # A with-past decoder is a different graph, not a renamed one. It takes ONE
    # token plus the accumulated keys and values, and returns the next logits
    # plus the extended cache — which is what makes generation O(n) instead of
    # re-running the whole prefix at every step.
    print(f"[*] Exporting With-Past Decoder -> {decoder_past_path}...")

    cfg = model.decoder.config
    n_layers = cfg.num_hidden_layers
    n_heads = cfg.num_attention_heads
    head_dim = cfg.hidden_size // n_heads

    # Names the Rust GraphContract expects, in the order the flattened tuple is
    # passed. `decoder` is self-attention, `encoder` is cross-attention: the
    # names describe which side of the encoder-decoder the cache belongs to.
    def past_names(prefix):
        names = []
        for layer in range(n_layers):
            for side in ("decoder", "encoder"):
                for part in ("key", "value"):
                    names.append(f"{prefix}.{layer}.{side}.{part}")
        return names

    class DecoderWithPastWrapper(torch.nn.Module):
        """Flattens `EncoderDecoderCache` into named ONNX inputs and outputs.

        transformers 5.x carries the cache as objects, not tuples, so the graph
        boundary has to flatten and rebuild it. Order is
        (layer, self/cross, key/value) and MUST match `past_names` — a
        transposition here produces a graph that loads, runs, and decodes
        nonsense, which is far worse than one that refuses.
        """

        def __init__(self, decoder, n_layers):
            super().__init__()
            self.decoder = decoder
            self.n_layers = n_layers

        def forward(self, input_ids, encoder_hidden_states, *flat_past):
            from transformers.cache_utils import DynamicCache, EncoderDecoderCache

            self_cache, cross_cache = DynamicCache(), DynamicCache()
            for layer in range(self.n_layers):
                base = layer * 4
                self_cache.update(flat_past[base], flat_past[base + 1], layer)
                cross_cache.update(flat_past[base + 2], flat_past[base + 3], layer)

            out = self.decoder(
                input_ids=input_ids,
                encoder_hidden_states=encoder_hidden_states,
                past_key_values=EncoderDecoderCache(self_cache, cross_cache),
                use_cache=True,
                return_dict=True,
            )

            present = []
            for layer in range(self.n_layers):
                s = out.past_key_values.self_attention_cache.layers[layer]
                c = out.past_key_values.cross_attention_cache.layers[layer]
                present.extend([s.keys, s.values, c.keys, c.values])
            return (out.logits, *present)

    # One already-decoded token of self-attention history, and the encoder cache
    # over all 197 patches — the shapes generation actually presents at step 2.
    dummy_self = torch.randn(1, n_heads, 1, head_dim)
    dummy_cross = torch.randn(1, n_heads, 197, head_dim)
    flat_past = []
    for _ in range(n_layers):
        flat_past += [dummy_self.clone(), dummy_self.clone(),
                      dummy_cross.clone(), dummy_cross.clone()]

    past_in = past_names("past_key_values")
    present_out = past_names("present")

    # Self-attention history grows by one token each step; the cross-attention
    # cache is fixed at the encoder length and never grows. They need DISTINCT
    # axis names — one dim_param is one size everywhere it appears, so sharing a
    # name asserts they are equal and the graph fails at run time.
    past_axes = {}
    for name in past_in:
        past_axes[name] = {
            0: "batch_size",
            2: "encoder_sequence_length" if ".encoder." in name else "past_sequence_length",
        }
    for name in present_out:
        past_axes[name] = {
            0: "batch_size",
            2: "encoder_sequence_length" if ".encoder." in name else "present_sequence_length",
        }

    with_past_wrapper = DecoderWithPastWrapper(model.decoder, n_layers)
    with_past_wrapper.eval()

    torch.onnx.export(
        with_past_wrapper,
        (dummy_input_ids[:, :1], dummy_encoder_hidden_states, *flat_past),
        decoder_past_path,
        input_names=["input_ids", "encoder_hidden_states", *past_in],
        output_names=["logits", *present_out],
        dynamic_axes={
            "input_ids": {0: "batch_size", 1: "sequence_length"},
            "encoder_hidden_states": {0: "batch_size"},
            "logits": {0: "batch_size", 1: "sequence_length"},
            **past_axes,
        },
        opset_version=14,
    )

    print(f"[SUCCESS] ONNX models exported to {output_dir}:")
    for fname in ["encoder_model.onnx", "decoder_model.onnx", "decoder_with_past_model.onnx"]:
        p = os.path.join(output_dir, fname)
        size_mb = os.path.getsize(p) / (1024 * 1024) if os.path.exists(p) else 0
        print(f"  - {fname}: {size_mb:.2f} MB")

def main():
    parser = argparse.ArgumentParser(description="Export Manga OCR PyTorch model to 3-graph ONNX layout.")
    parser.add_argument("--output-dir", type=str, default="models/onnx", help="Directory to save ONNX models")
    args = parser.parse_args()
    export_manga_ocr_onnx(args.output_dir)

if __name__ == "__main__":
    main()
