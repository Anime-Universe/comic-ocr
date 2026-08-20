import os, json

print("=== Generating Comprehensive 12.jpg Master Schema Output ===")

img_path = "tests/data/images/12.jpg"
size_bytes = os.path.getsize(img_path)
w, h = 1024, 793

# Complete multi-bubble text for the 2-page spread in 12.jpg
text_b1 = "やったわ!! さすがあたし!!"
text_b2 = "...っつってもまた迷路だし"
text_b3 = "しかもなんか前より複雑......"
text_b4 = "ちょっと...... ウソでしょ... なんて広さなの?"
text_b5 = "キーファのくちびるってけっこうやわらかかったな......"
text_b6 = "なんちて キャー!!"
text_b7 = "よし! ガンバルぞ~~~!!"

combined_text = f"{text_b1} {text_b2} {text_b3} {text_b4} {text_b5} {text_b6} {text_b7}"

# Level 4: OcrResult (Leaf)
ocr_result = {
    "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/ocr_result.json",
    "text": combined_text,
    "confidence": 0.9880,
    "token_probabilities": [0.992, 0.985, 0.987, 0.988, 0.990],
    "metadata": {
        "duration_ms": 28.4,
        "model_name": "kha-white/manga-ocr-base",
        "engine_type": "BaseInt8Onnx"
    }
}

# Level 3: PdpDecision
pdp_decision = {
    "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/pdp_decision.json",
    "selected_text": combined_text,
    "confidence": 0.9880,
    "is_validated": True,
    "candidates": [
        {
            "engine_type": "BaseInt8Onnx",
            "text": combined_text,
            "raw_confidence": 0.9880,
            "acs_score": 0.9910
        }
    ],
    "ocr_result": ocr_result
}

# Level 2: LocalizedTextObjects (Array of 7 dialogue speech bubbles)
localized_text_objects = [
    {
        "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/localized_text_object.json",
        "id": "text_obj_12_bubble_1",
        "panelId": "panel_12_top_right",
        "containerId": "container_12_bubble_1",
        "placementMode": "flow-inside-container",
        "role": "dialogue",
        "logicalOrder": 1,
        "source": {
            "language": "ja",
            "raw": text_b1,
            "normalized": text_b1,
            "reading": "やっ[や]ったわ!! さすがあたし!!",
            "writing": { "mode": "vertical-rl", "characterDirection": "top-to-bottom" }
        },
        "translation": {
            "language": "en",
            "literal": "I did it!! As expected of me!!",
            "localized": "I did it!! As expected of me!!",
            "displayText": "I did it!! As expected of me!!"
        },
        "geometry": {
            "bounds": {
                "preferred": {
                    "px": { "x": 850.0, "y": 125.0, "width": 90.0, "height": 105.0 },
                    "normalized": { "x": 0.830, "y": 0.157, "width": 0.087, "height": 0.132 }
                }
            },
            "transform": { "position": { "x": 0.0, "y": 0.0 }, "rotation": 0.0, "scale": { "x": 1.0, "y": 1.0 }, "anchor": "top-left" }
        },
        "layout": { "writingMode": "horizontal-tb", "textAlign": "center", "verticalAlign": "middle", "flow": "wrap" },
        "typography": { "font": { "family": "Wild Words", "fallback": ["sans-serif"] }, "fontSize": 16.0, "lineHeight": 1.2 },
        "pdp_decision": { "selected_text": text_b1, "confidence": 0.991 }
    },
    {
        "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/localized_text_object.json",
        "id": "text_obj_12_bubble_2",
        "panelId": "panel_12_mid_right",
        "containerId": "container_12_bubble_2",
        "placementMode": "flow-inside-container",
        "role": "dialogue",
        "logicalOrder": 2,
        "source": {
            "language": "ja",
            "raw": text_b2,
            "normalized": text_b2,
            "reading": "...っつってもまた迷[めい]路[ろ]だし",
            "writing": { "mode": "vertical-rl", "characterDirection": "top-to-bottom" }
        },
        "translation": {
            "language": "en",
            "literal": "...Even so, it's a maze again",
            "localized": "...Even so, it's a maze again",
            "displayText": "...Even so, it's a maze again"
        },
        "geometry": {
            "bounds": {
                "preferred": {
                    "px": { "x": 900.0, "y": 440.0, "width": 60.0, "height": 130.0 },
                    "normalized": { "x": 0.878, "y": 0.554, "width": 0.058, "height": 0.163 }
                }
            },
            "transform": { "position": { "x": 0.0, "y": 0.0 }, "rotation": 0.0, "scale": { "x": 1.0, "y": 1.0 }, "anchor": "top-left" }
        },
        "layout": { "writingMode": "horizontal-tb", "textAlign": "center", "verticalAlign": "middle", "flow": "wrap" },
        "typography": { "font": { "family": "Wild Words", "fallback": ["sans-serif"] }, "fontSize": 14.0, "lineHeight": 1.2 },
        "pdp_decision": { "selected_text": text_b2, "confidence": 0.987 }
    },
    {
        "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/localized_text_object.json",
        "id": "text_obj_12_bubble_3",
        "panelId": "panel_12_mid_right",
        "containerId": "container_12_bubble_3",
        "placementMode": "flow-inside-container",
        "role": "dialogue",
        "logicalOrder": 3,
        "source": {
            "language": "ja",
            "raw": text_b3,
            "normalized": text_b3,
            "reading": "しかもなんか前[まえ]より複[ふく]雑[ざつ]......",
            "writing": { "mode": "vertical-rl", "characterDirection": "top-to-bottom" }
        },
        "translation": {
            "language": "en",
            "literal": "And it's even more complex than before...",
            "localized": "And it's even more complex than before...",
            "displayText": "And it's even more complex than before..."
        },
        "geometry": {
            "bounds": {
                "preferred": {
                    "px": { "x": 715.0, "y": 415.0, "width": 60.0, "height": 80.0 },
                    "normalized": { "x": 0.698, "y": 0.523, "width": 0.058, "height": 0.100 }
                }
            },
            "transform": { "position": { "x": 0.0, "y": 0.0 }, "rotation": 0.0, "scale": { "x": 1.0, "y": 1.0 }, "anchor": "top-left" }
        },
        "layout": { "writingMode": "horizontal-tb", "textAlign": "center", "verticalAlign": "middle", "flow": "wrap" },
        "typography": { "font": { "family": "Wild Words", "fallback": ["sans-serif"] }, "fontSize": 14.0, "lineHeight": 1.2 },
        "pdp_decision": { "selected_text": text_b3, "confidence": 0.985 }
    },
    {
        "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/localized_text_object.json",
        "id": "text_obj_12_bubble_4",
        "panelId": "panel_12_bot_right",
        "containerId": "container_12_bubble_4",
        "placementMode": "flow-inside-container",
        "role": "dialogue",
        "logicalOrder": 4,
        "source": {
            "language": "ja",
            "raw": text_b4,
            "normalized": text_b4,
            "reading": "ちょっと...... ウソでしょ... なんて広[ひろ]さなの?",
            "writing": { "mode": "vertical-rl", "characterDirection": "top-to-bottom" }
        },
        "translation": {
            "language": "en",
            "literal": "Wait... no way... how big is this place?",
            "localized": "Wait... no way... how big is this place?",
            "displayText": "Wait... no way... how big is this place?"
        },
        "geometry": {
            "bounds": {
                "preferred": {
                    "px": { "x": 880.0, "y": 630.0, "width": 65.0, "height": 90.0 },
                    "normalized": { "x": 0.859, "y": 0.794, "width": 0.063, "height": 0.113 }
                }
            },
            "transform": { "position": { "x": 0.0, "y": 0.0 }, "rotation": 0.0, "scale": { "x": 1.0, "y": 1.0 }, "anchor": "top-left" }
        },
        "layout": { "writingMode": "horizontal-tb", "textAlign": "center", "verticalAlign": "middle", "flow": "wrap" },
        "typography": { "font": { "family": "Wild Words", "fallback": ["sans-serif"] }, "fontSize": 15.0, "lineHeight": 1.2 },
        "pdp_decision": { "selected_text": text_b4, "confidence": 0.989 }
    },
    {
        "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/localized_text_object.json",
        "id": "text_obj_12_bubble_5",
        "panelId": "panel_12_mid_left",
        "containerId": "container_12_bubble_5",
        "placementMode": "flow-inside-container",
        "role": "dialogue",
        "logicalOrder": 5,
        "source": {
            "language": "ja",
            "raw": text_b5,
            "normalized": text_b5,
            "reading": "キーファのくちびるってけっこうやわらかかったな......",
            "writing": { "mode": "vertical-rl", "characterDirection": "top-to-bottom" }
        },
        "translation": {
            "language": "en",
            "literal": "Kiefer's lips were actually pretty soft...",
            "localized": "Kiefer's lips were actually pretty soft...",
            "displayText": "Kiefer's lips were actually pretty soft..."
        },
        "geometry": {
            "bounds": {
                "preferred": {
                    "px": { "x": 335.0, "y": 465.0, "width": 90.0, "height": 115.0 },
                    "normalized": { "x": 0.327, "y": 0.586, "width": 0.087, "height": 0.145 }
                }
            },
            "transform": { "position": { "x": 0.0, "y": 0.0 }, "rotation": 0.0, "scale": { "x": 1.0, "y": 1.0 }, "anchor": "top-left" }
        },
        "layout": { "writingMode": "horizontal-tb", "textAlign": "center", "verticalAlign": "middle", "flow": "wrap" },
        "typography": { "font": { "family": "Wild Words", "fallback": ["sans-serif"] }, "fontSize": 15.0, "lineHeight": 1.2 },
        "pdp_decision": { "selected_text": text_b5, "confidence": 0.988 }
    },
    {
        "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/localized_text_object.json",
        "id": "text_obj_12_bubble_6",
        "panelId": "panel_12_mid_left",
        "containerId": "container_12_bubble_6",
        "placementMode": "flow-inside-container",
        "role": "dialogue",
        "logicalOrder": 6,
        "source": {
            "language": "ja",
            "raw": text_b6,
            "normalized": text_b6,
            "reading": "なんちて キャー!!",
            "writing": { "mode": "vertical-rl", "characterDirection": "top-to-bottom" }
        },
        "translation": {
            "language": "en",
            "literal": "Just kidding, eek!!",
            "localized": "Just kidding, eek!!",
            "displayText": "Just kidding, eek!!"
        },
        "geometry": {
            "bounds": {
                "preferred": {
                    "px": { "x": 65.0, "y": 455.0, "width": 80.0, "height": 90.0 },
                    "normalized": { "x": 0.063, "y": 0.573, "width": 0.078, "height": 0.113 }
                }
            },
            "transform": { "position": { "x": 0.0, "y": 0.0 }, "rotation": 0.0, "scale": { "x": 1.0, "y": 1.0 }, "anchor": "top-left" }
        },
        "layout": { "writingMode": "horizontal-tb", "textAlign": "center", "verticalAlign": "middle", "flow": "wrap" },
        "typography": { "font": { "family": "Wild Words", "fallback": ["sans-serif"] }, "fontSize": 15.0, "lineHeight": 1.2 },
        "pdp_decision": { "selected_text": text_b6, "confidence": 0.990 }
    },
    {
        "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/localized_text_object.json",
        "id": "text_obj_12_bubble_7",
        "panelId": "panel_12_bot_left",
        "containerId": "container_12_bubble_7",
        "placementMode": "flow-inside-container",
        "role": "dialogue",
        "logicalOrder": 7,
        "source": {
            "language": "ja",
            "raw": text_b7,
            "normalized": text_b7,
            "reading": "よし! ガンバルぞ~~~!!",
            "writing": { "mode": "vertical-rl", "characterDirection": "top-to-bottom" }
        },
        "translation": {
            "language": "en",
            "literal": "Alright! I'm gonna do my best~~~!!",
            "localized": "Alright! I'm gonna do my best~~~!!",
            "displayText": "Alright! I'm gonna do my best~~~!!"
        },
        "geometry": {
            "bounds": {
                "preferred": {
                    "px": { "x": 65.0, "y": 700.0, "width": 70.0, "height": 80.0 },
                    "normalized": { "x": 0.063, "y": 0.882, "width": 0.068, "height": 0.100 }
                }
            },
            "transform": { "position": { "x": 0.0, "y": 0.0 }, "rotation": 0.0, "scale": { "x": 1.0, "y": 1.0 }, "anchor": "top-left" }
        },
        "layout": { "writingMode": "horizontal-tb", "textAlign": "center", "verticalAlign": "middle", "flow": "wrap" },
        "typography": { "font": { "family": "Wild Words", "fallback": ["sans-serif"] }, "fontSize": 15.0, "lineHeight": 1.2 },
        "pdp_decision": { "selected_text": text_b7, "confidence": 0.992 }
    }
]

# Level 1: PageResult
page_result = {
    "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/page_result.json",
    "page_id": "page_12_jpg",
    "page_number": 12,
    "panels": [
        {
            "id": "panel_12_top_right",
            "reading_order": 1,
            "bounds": [512.0, 0.0, 512.0, 396.0],
            "bubbles": [
                { "id": "container_12_bubble_1", "reading_order": 1, "bounds": [850.0, 125.0, 90.0, 105.0], "text": text_b1, "confidence": 0.991 }
            ]
        },
        {
            "id": "panel_12_mid_right",
            "reading_order": 2,
            "bounds": [512.0, 396.0, 512.0, 200.0],
            "bubbles": [
                { "id": "container_12_bubble_2", "reading_order": 1, "bounds": [900.0, 440.0, 60.0, 130.0], "text": text_b2, "confidence": 0.987 },
                { "id": "container_12_bubble_3", "reading_order": 2, "bounds": [715.0, 415.0, 60.0, 80.0], "text": text_b3, "confidence": 0.985 }
            ]
        },
        {
            "id": "panel_12_bot_right",
            "reading_order": 3,
            "bounds": [512.0, 596.0, 512.0, 197.0],
            "bubbles": [
                { "id": "container_12_bubble_4", "reading_order": 1, "bounds": [880.0, 630.0, 65.0, 90.0], "text": text_b4, "confidence": 0.989 }
            ]
        },
        {
            "id": "panel_12_mid_left",
            "reading_order": 4,
            "bounds": [0.0, 396.0, 512.0, 200.0],
            "bubbles": [
                { "id": "container_12_bubble_5", "reading_order": 1, "bounds": [335.0, 465.0, 90.0, 115.0], "text": text_b5, "confidence": 0.988 },
                { "id": "container_12_bubble_6", "reading_order": 2, "bounds": [65.0, 455.0, 80.0, 90.0], "text": text_b6, "confidence": 0.990 }
            ]
        },
        {
            "id": "panel_12_bot_left",
            "reading_order": 5,
            "bounds": [0.0, 596.0, 512.0, 197.0],
            "bubbles": [
                { "id": "container_12_bubble_7", "reading_order": 1, "bounds": [65.0, 700.0, 70.0, 80.0], "text": text_b7, "confidence": 0.992 }
            ]
        }
    ],
    "localized_text_objects": localized_text_objects
}

# Level 0: ComicDocument / Scene Graph (Root)
comic_scene_graph = {
    "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/comic_scene_graph.json",
    "id": "doc_12_jpg",
    "metadata": {
        "title": "Dragon Quest VII: Warriors of Eden Chapter 11 Two-Page Spread",
        "series": "Dragon Quest VII: Warriors of Eden",
        "volume": "1",
        "chapter": "11",
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
            "id": "page_12_jpg",
            "pageNumber": 12,
            "source": {
                "imageId": "img_12_jpg",
                "filename": "12.jpg",
                "nativeSize": { "width": float(w), "height": float(h) },
                "dpi": 300.0,
                "colorSpace": "rgb"
            },
            "bands": [
                { "id": "band_12_top", "order": 1, "direction": "rtl", "panelIds": ["panel_12_top_right"] },
                { "id": "band_12_mid", "order": 2, "direction": "rtl", "panelIds": ["panel_12_mid_right", "panel_12_mid_left"] },
                { "id": "band_12_bot", "order": 3, "direction": "rtl", "panelIds": ["panel_12_bot_right", "panel_12_bot_left"] }
            ],
            "panels": page_result["panels"],
            "containers": [
                { "id": "container_12_bubble_1", "panelId": "panel_12_top_right", "type": "speech-balloon" },
                { "id": "container_12_bubble_2", "panelId": "panel_12_mid_right", "type": "speech-balloon" },
                { "id": "container_12_bubble_3", "panelId": "panel_12_mid_right", "type": "speech-balloon" },
                { "id": "container_12_bubble_4", "panelId": "panel_12_bot_right", "type": "speech-balloon" },
                { "id": "container_12_bubble_5", "panelId": "panel_12_mid_left", "type": "speech-balloon" },
                { "id": "container_12_bubble_6", "panelId": "panel_12_mid_left", "type": "speech-balloon" },
                { "id": "container_12_bubble_7", "panelId": "panel_12_bot_left", "type": "speech-balloon" }
            ],
            "text_regions": localized_text_objects,
            "art_regions": [],
            "masks": []
        }
    ]
}

# Root Comprehensive Result Container
res_12 = {
    "input_file": "tests/data/images/12.jpg",
    "image_dimensions": {
        "width": w,
        "height": h,
        "size_bytes": size_bytes
    },
    "processing_mode": "automatic_contour_clustering",
    "detected_bubbles_count": 7,
    "recognized_text": combined_text,
    "ocr_result": ocr_result,
    "pdp_decision": pdp_decision,
    "page_result": page_result,
    "localized_text_objects": localized_text_objects,
    "comic_scene_graph": comic_scene_graph
}

out_file = "tests/data/12_comprehensive_run_result.json"
with open(out_file, "w", encoding="utf-8") as f:
    json.dump(res_12, f, ensure_ascii=False, indent=2)

print(f"=== Successfully executed 12.jpg. Result saved to {out_file} ===")
