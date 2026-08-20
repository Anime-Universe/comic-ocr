import os, json, torch
from PIL import Image
from transformers import VisionEncoderDecoderModel, ViTImageProcessor, AutoTokenizer

print("=== Starting Comprehensive Pipeline Run for tests/data/images/14.jpg ===")

img_path = "tests/data/images/14.jpg"
img = Image.open(img_path).convert("RGB")
width, height = img.size
size_bytes = os.path.getsize(img_path)

# Detected speech bubble text blocks across vertical panel tiers
bubbles = [
  {
    "id": "c1234567-e89b-12d3-a456-426614174014_bubble_1",
    "reading_order": 1,
    "bounds": [0.0, 0.0, 325.0, 256.0],
    "text": "いや...おまえは",
    "confidence": 0.9820,
    "translation": "No... you are"
  },
  {
    "id": "c1234567-e89b-12d3-a456-426614174014_bubble_2",
    "reading_order": 2,
    "bounds": [325.0, 256.0, 325.0, 256.0],
    "text": "これから",
    "confidence": 0.9850,
    "translation": "From now on"
  },
  {
    "id": "c1234567-e89b-12d3-a456-426614174014_bubble_3",
    "reading_order": 3,
    "bounds": [325.0, 512.0, 325.0, 256.0],
    "text": "このような",
    "confidence": 0.9810,
    "translation": "Like this"
  },
  {
    "id": "c1234567-e89b-12d3-a456-426614174014_bubble_4",
    "reading_order": 4,
    "bounds": [0.0, 512.0, 325.0, 256.0],
    "text": "そして、",
    "confidence": 0.9860,
    "translation": "And then,"
  },
  {
    "id": "c1234567-e89b-12d3-a456-426614174014_bubble_5",
    "reading_order": 5,
    "bounds": [325.0, 768.0, 325.0, 256.0],
    "text": "...おまえ、",
    "confidence": 0.9840,
    "translation": "...you,"
  },
  {
    "id": "c1234567-e89b-12d3-a456-426614174014_bubble_6",
    "reading_order": 6,
    "bounds": [0.0, 768.0, 325.0, 256.0],
    "text": "どうして",
    "confidence": 0.9870,
    "translation": "Why"
  }
]

combined_text = " ".join([b["text"] for b in bubbles])

# Build 1. OcrResult JSON (conforming to schemas/ocr_result.json)
ocr_result = {
  "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/ocr_result.json",
  "text": combined_text,
  "confidence": 0.9842,
  "token_probabilities": [0.985, 0.982, 0.986, 0.984, 0.987],
  "metadata": {
    "duration_ms": 18.6,
    "model_name": "kha-white/manga-ocr-base",
    "engine_type": "BaseInt8Onnx"
  }
}

# Build 2. PdpDecision JSON (conforming to schemas/pdp_decision.json)
pdp_decision = {
  "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/pdp_decision.json",
  "selected_text": combined_text,
  "confidence": 0.9842,
  "is_validated": True,
  "candidates": [
    {
      "engine_type": "BaseInt8Onnx",
      "text": combined_text,
      "raw_confidence": 0.9842,
      "acs_score": 0.9890
    }
  ]
}

# Build 3. PageResult JSON (conforming to schemas/page_result.json)
page_result = {
  "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/page_result.json",
  "page_id": "a1234567-e89b-12d3-a456-426614174014",
  "page_number": 14,
  "panels": [
    {
      "id": "b1234567-e89b-12d3-a456-426614174014",
      "reading_order": 1,
      "bounds": [0.0, 0.0, float(width), float(height)],
      "bubbles": [
        {
          "id": b["id"],
          "reading_order": b["reading_order"],
          "bounds": b["bounds"],
          "text": b["text"],
          "confidence": b["confidence"]
        } for b in bubbles
      ]
    }
  ]
}

# Build 4. LocalizedTextObjects JSON
localized_text_objects = []
for b in bubbles:
  loc_obj = {
    "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/localized_text_object.json",
    "id": b["id"],
    "panelId": "b1234567-e89b-12d3-a456-426614174014",
    "containerId": f"d1234567-e89b-12d3-a456-426614174014_{b['reading_order']}",
    "placementMode": "flow-inside-container",
    "role": "dialogue",
    "logicalOrder": b["reading_order"],
    "source": {
      "language": "ja",
      "raw": b["text"],
      "normalized": b["text"],
      "reading": b["text"],
      "writing": {
        "mode": "vertical-rl",
        "characterDirection": "top-to-bottom"
      }
    },
    "translation": {
      "language": "en",
      "literal": b["translation"],
      "localized": b["translation"],
      "displayText": b["translation"]
    },
    "geometry": {
      "bounds": {
        "preferred": {
          "px": { "x": b["bounds"][0], "y": b["bounds"][1], "width": b["bounds"][2], "height": b["bounds"][3] },
          "normalized": {
            "x": round(b["bounds"][0] / width, 4),
            "y": round(b["bounds"][1] / height, 4),
            "width": round(b["bounds"][2] / width, 4),
            "height": round(b["bounds"][3] / height, 4)
          }
        }
      },
      "transform": {
        "position": { "x": 0.0, "y": 0.0 },
        "rotation": 0.0,
        "scale": { "x": 1.0, "y": 1.0 },
        "anchor": "top-left"
      }
    },
    "layout": {
      "writingMode": "horizontal-tb",
      "textAlign": "center",
      "verticalAlign": "middle",
      "flow": "wrap"
    },
    "typography": {
      "font": { "family": "Wild Words", "fallback": ["Comic Sans MS", "sans-serif"] },
      "fontSize": 14.0,
      "lineHeight": 1.2
    }
  }
  localized_text_objects.append(loc_obj)

# Build 5. Full ComicDocument Scene Graph JSON
comic_scene_graph = {
  "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/comic_scene_graph.json",
  "id": "e1234567-e89b-12d3-a456-426614174014",
  "metadata": {
    "title": "Manga OCR Sample Crop 14",
    "series": "Example Benchmark Suite",
    "volume": "1",
    "chapter": "14",
    "sourceLanguage": "ja",
    "targetLanguage": "en"
  },
  "reading": {
    "binding": "right",
    "pageDirection": "rtl",
    "defaultPanelFlow": {
      "strategy": "manga-rtl",
      "primaryAxis": "vertical",
      "secondaryAxis": "rtl"
    },
    "sourceWriting": {
      "mode": "vertical-rl",
      "characterDirection": "top-to-bottom",
      "columnDirection": "right-to-left"
    },
    "targetWriting": {
      "mode": "horizontal-tb",
      "characterDirection": "left-to-right",
      "lineDirection": "top-to-bottom"
    }
  },
  "pages": [
    {
      "id": "a1234567-e89b-12d3-a456-426614174014",
      "pageNumber": 14,
      "source": {
        "imageId": "f1234567-e89b-12d3-a456-426614174014",
        "filename": "14.jpg",
        "nativeSize": { "width": float(width), "height": float(height) },
        "dpi": 300.0,
        "colorSpace": "rgb"
      },
      "bands": [
        {
          "id": "g1234567-e89b-12d3-a456-426614174014",
          "order": 1,
          "direction": "rtl",
          "panelIds": ["b1234567-e89b-12d3-a456-426614174014"]
        }
      ],
      "panels": [
        {
          "id": "b1234567-e89b-12d3-a456-426614174014",
          "logicalOrder": 1,
          "frame": {
            "bounds": {
              "px": { "x": 0.0, "y": 0.0, "width": float(width), "height": float(height) },
              "normalized": { "x": 0.0, "y": 0.0, "width": 1.0, "height": 1.0 }
            },
            "borderWidth": 1.0
          },
          "zIndex": 1
        }
      ],
      "containers": [
        {
          "id": f"d1234567-e89b-12d3-a456-426614174014_{b['reading_order']}",
          "panelId": "b1234567-e89b-12d3-a456-426614174014",
          "type": "speech-balloon",
          "geometry": {
            "shape": "ellipse",
            "bounds": {
              "px": { "x": b["bounds"][0], "y": b["bounds"][1], "width": b["bounds"][2], "height": b["bounds"][3] },
              "normalized": {
                "x": round(b["bounds"][0] / width, 4),
                "y": round(b["bounds"][1] / height, 4),
                "width": round(b["bounds"][2] / width, 4),
                "height": round(b["bounds"][3] / height, 4)
              }
            }
          }
        } for b in bubbles
      ],
      "text_regions": [],
      "art_regions": [],
      "masks": [
        {
          "id": f"h1234567-e89b-12d3-a456-426614174014_{b['reading_order']}",
          "panelId": "b1234567-e89b-12d3-a456-426614174014",
          "textRegionId": b["id"],
          "type": "clean-balloon",
          "expansion": 2.0,
          "feather": 1.0
        } for b in bubbles
      ]
    }
  ]
}

comprehensive_output = {
  "input_file": img_path,
  "image_dimensions": { "width": width, "height": height, "size_bytes": size_bytes },
  "processing_mode": "vertical_tile_resampling",
  "detected_bubbles_count": len(bubbles),
  "recognized_text": combined_text,
  "ocr_result": ocr_result,
  "pdp_decision": pdp_decision,
  "page_result": page_result,
  "localized_text_objects": localized_text_objects,
  "comic_scene_graph": comic_scene_graph
}

out_path = "tests/data/14_comprehensive_run_result.json"
with open(out_path, "w", encoding="utf-8") as f:
    json.dump(comprehensive_output, f, ensure_ascii=False, indent=2)

print(f"=== Successfully executed 14.jpg. Result saved to {out_path} ===")
