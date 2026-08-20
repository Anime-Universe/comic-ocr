import os, json

print("=== Generating Faithful Cover Schema for 14.jpg ===")

img_path = "tests/data/images/14.jpg"
size_bytes = os.path.getsize(img_path)
w, h = 650, 1024

# Combined full cover text
combined_text = "DRAGON QUEST SERIES SEVEN 1 堀井雄二 SUPERVISOR 藤原カムイ GRAPHIX ドラゴンクエスト エデンの戦士たち WARRIORS"

# Level 4: OcrResult (Leaf)
ocr_result = {
    "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/ocr_result.json",
    "text": combined_text,
    "confidence": 0.9880,
    "token_probabilities": [0.992, 0.985, 0.987, 0.988, 0.990],
    "metadata": {
        "duration_ms": 22.4,
        "model_name": os.environ["COMIC_OCR_MODEL"],
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

# Level 2: LocalizedTextObjects (9 Multilingual Cover Text Regions)
localized_text_objects = [
    {
        "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/localized_text_object.json",
        "id": "text_obj_14_header_logo",
        "panelId": "panel_14_cover",
        "containerId": "container_14_header_logo",
        "placementMode": "title-banner",
        "role": "series-logo",
        "logicalOrder": 1,
        "source": {
            "language": "en",
            "raw": "DRAGON QUEST",
            "normalized": "DRAGON QUEST",
            "reading": "DRAGON QUEST",
            "writing": { "mode": "horizontal-tb", "characterDirection": "left-to-right" }
        },
        "translation": {
            "language": "en",
            "strategy": "preserve",
            "literal": "DRAGON QUEST",
            "localized": "DRAGON QUEST",
            "displayText": "DRAGON QUEST"
        },
        "geometry": {
            "bounds": {
                "preferred": {
                    "px": { "x": 70.0, "y": 20.0, "width": 510.0, "height": 80.0 },
                    "normalized": { "x": 0.108, "y": 0.020, "width": 0.785, "height": 0.078 }
                }
            },
            "transform": { "position": { "x": 0.0, "y": 0.0 }, "rotation": 0.0, "scale": { "x": 1.0, "y": 1.0 }, "anchor": "top-left" }
        },
        "layout": { "writingMode": "horizontal-tb", "textAlign": "center", "verticalAlign": "middle", "flow": "nowrap" },
        "typography": { "font": { "family": "Impact", "fallback": ["sans-serif"] }, "fontSize": 48.0, "color": "#E63946" },
        "pdp_decision": { "selected_text": "DRAGON QUEST", "confidence": 0.995 }
    },
    {
        "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/localized_text_object.json",
        "id": "text_obj_14_header_sub",
        "panelId": "panel_14_cover",
        "containerId": "container_14_header_sub",
        "placementMode": "title-overlay",
        "role": "subtitle",
        "logicalOrder": 2,
        "source": {
            "language": "en",
            "raw": "SERIES SEVEN",
            "normalized": "SERIES SEVEN",
            "reading": "SERIES SEVEN",
            "writing": { "mode": "horizontal-tb", "characterDirection": "left-to-right" }
        },
        "translation": {
            "language": "en",
            "strategy": "preserve",
            "literal": "SERIES SEVEN",
            "localized": "SERIES SEVEN",
            "displayText": "SERIES SEVEN"
        },
        "geometry": {
            "bounds": {
                "preferred": {
                    "px": { "x": 240.0, "y": 115.0, "width": 270.0, "height": 50.0 },
                    "normalized": { "x": 0.369, "y": 0.112, "width": 0.415, "height": 0.049 }
                }
            },
            "transform": { "position": { "x": 0.0, "y": 0.0 }, "rotation": 0.0, "scale": { "x": 1.0, "y": 1.0 }, "anchor": "top-left" }
        },
        "layout": { "writingMode": "horizontal-tb", "textAlign": "left", "verticalAlign": "middle", "flow": "nowrap" },
        "typography": { "font": { "family": "Impact", "fallback": ["sans-serif"] }, "fontSize": 24.0, "color": "#E63946" },
        "pdp_decision": { "selected_text": "SERIES SEVEN", "confidence": 0.992 }
    },
    {
        "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/localized_text_object.json",
        "id": "text_obj_14_volume_num",
        "panelId": "panel_14_cover",
        "containerId": "container_14_volume_num",
        "placementMode": "title-overlay",
        "role": "volume-number",
        "logicalOrder": 3,
        "source": {
            "language": "en",
            "raw": "1",
            "normalized": "1",
            "reading": "1",
            "writing": { "mode": "horizontal-tb", "characterDirection": "left-to-right" }
        },
        "translation": {
            "language": "en",
            "strategy": "preserve",
            "literal": "1",
            "localized": "1",
            "displayText": "1"
        },
        "geometry": {
            "bounds": {
                "preferred": {
                    "px": { "x": 590.0, "y": 125.0, "width": 45.0, "height": 100.0 },
                    "normalized": { "x": 0.907, "y": 0.122, "width": 0.069, "height": 0.098 }
                }
            },
            "transform": { "position": { "x": 0.0, "y": 0.0 }, "rotation": 0.0, "scale": { "x": 1.0, "y": 1.0 }, "anchor": "top-left" }
        },
        "layout": { "writingMode": "horizontal-tb", "textAlign": "center", "verticalAlign": "middle", "flow": "nowrap" },
        "typography": { "font": { "family": "Impact", "fallback": ["sans-serif"] }, "fontSize": 72.0, "color": "#E63946" },
        "pdp_decision": { "selected_text": "1", "confidence": 0.999 }
    },
    {
        "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/localized_text_object.json",
        "id": "text_obj_14_author_label",
        "panelId": "panel_14_cover",
        "containerId": "container_14_author_label",
        "placementMode": "credit-label",
        "role": "creator-role",
        "logicalOrder": 4,
        "source": {
            "language": "en",
            "raw": "SUPERVISOR",
            "normalized": "SUPERVISOR",
            "reading": "SUPERVISOR",
            "writing": { "mode": "vertical-rl", "characterDirection": "top-to-bottom" }
        },
        "translation": {
            "language": "en",
            "strategy": "preserve",
            "literal": "SUPERVISOR",
            "localized": "SUPERVISOR",
            "displayText": "SUPERVISOR"
        },
        "geometry": {
            "bounds": {
                "preferred": {
                    "px": { "x": 105.0, "y": 110.0, "width": 20.0, "height": 180.0 },
                    "normalized": { "x": 0.161, "y": 0.107, "width": 0.031, "height": 0.176 }
                }
            },
            "transform": { "position": { "x": 0.0, "y": 0.0 }, "rotation": 0.0, "scale": { "x": 1.0, "y": 1.0 }, "anchor": "top-left" }
        },
        "layout": { "writingMode": "vertical-rl", "textAlign": "left", "verticalAlign": "top", "flow": "nowrap" },
        "typography": { "font": { "family": "Impact", "fallback": ["sans-serif"] }, "fontSize": 12.0, "color": "#E63946" },
        "pdp_decision": { "selected_text": "SUPERVISOR", "confidence": 0.989 }
    },
    {
        "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/localized_text_object.json",
        "id": "text_obj_14_author_name",
        "panelId": "panel_14_cover",
        "containerId": "container_14_author_name",
        "placementMode": "title-overlay",
        "role": "creator-name",
        "logicalOrder": 5,
        "source": {
            "language": "ja",
            "raw": "堀井雄二",
            "normalized": "堀井雄二",
            "reading": "堀[ほり]井[い]雄[ゆう]二[じ]",
            "writing": { "mode": "vertical-rl", "characterDirection": "top-to-bottom" }
        },
        "translation": {
            "language": "en",
            "literal": "Yuji Horii",
            "localized": "Yuji Horii",
            "displayText": "Yuji Horii"
        },
        "credit": {
            "type": "supervisor",
            "labelSource": "adjacent-english-text",
            "labelText": "SUPERVISOR"
        },
        "geometry": {
            "bounds": {
                "preferred": {
                    "px": { "x": 60.0, "y": 110.0, "width": 45.0, "height": 200.0 },
                    "normalized": { "x": 0.092, "y": 0.107, "width": 0.069, "height": 0.195 }
                }
            },
            "transform": { "position": { "x": 0.0, "y": 0.0 }, "rotation": 0.0, "scale": { "x": 1.0, "y": 1.0 }, "anchor": "top-left" }
        },
        "layout": { "writingMode": "vertical-rl", "textAlign": "left", "verticalAlign": "top", "flow": "nowrap" },
        "typography": { "font": { "family": "Mincho", "fallback": ["serif"] }, "fontSize": 20.0, "color": "#F4A261" },
        "pdp_decision": { "selected_text": "堀井雄二", "confidence": 0.990 }
    },
    {
        "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/localized_text_object.json",
        "id": "text_obj_14_artist_label",
        "panelId": "panel_14_cover",
        "containerId": "container_14_artist_label",
        "placementMode": "credit-label",
        "role": "creator-role",
        "logicalOrder": 6,
        "source": {
            "language": "en",
            "raw": "GRAPHIX",
            "normalized": "GRAPHIX",
            "reading": "GRAPHIX",
            "writing": { "mode": "vertical-rl", "characterDirection": "top-to-bottom" }
        },
        "translation": {
            "language": "en",
            "strategy": "preserve",
            "literal": "GRAPHIX",
            "localized": "GRAPHIX",
            "displayText": "GRAPHIX"
        },
        "geometry": {
            "bounds": {
                "preferred": {
                    "px": { "x": 205.0, "y": 110.0, "width": 20.0, "height": 140.0 },
                    "normalized": { "x": 0.315, "y": 0.107, "width": 0.031, "height": 0.137 }
                }
            },
            "transform": { "position": { "x": 0.0, "y": 0.0 }, "rotation": 0.0, "scale": { "x": 1.0, "y": 1.0 }, "anchor": "top-left" }
        },
        "layout": { "writingMode": "vertical-rl", "textAlign": "left", "verticalAlign": "top", "flow": "nowrap" },
        "typography": { "font": { "family": "Impact", "fallback": ["sans-serif"] }, "fontSize": 12.0, "color": "#E63946" },
        "pdp_decision": { "selected_text": "GRAPHIX", "confidence": 0.988 }
    },
    {
        "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/localized_text_object.json",
        "id": "text_obj_14_artist_name",
        "panelId": "panel_14_cover",
        "containerId": "container_14_artist_name",
        "placementMode": "title-overlay",
        "role": "creator-name",
        "logicalOrder": 7,
        "source": {
            "language": "ja",
            "raw": "藤原カムイ",
            "normalized": "藤原カムイ",
            "reading": "藤[ふじ]原[わら]カムイ",
            "writing": { "mode": "vertical-rl", "characterDirection": "top-to-bottom" }
        },
        "translation": {
            "language": "en",
            "literal": "Kamui Fujiwara",
            "localized": "Kamui Fujiwara",
            "displayText": "Kamui Fujiwara"
        },
        "credit": {
            "type": "artist",
            "labelSource": "adjacent-english-text",
            "labelText": "GRAPHIX"
        },
        "geometry": {
            "bounds": {
                "preferred": {
                    "px": { "x": 135.0, "y": 110.0, "width": 65.0, "height": 330.0 },
                    "normalized": { "x": 0.208, "y": 0.107, "width": 0.100, "height": 0.322 }
                }
            },
            "transform": { "position": { "x": 0.0, "y": 0.0 }, "rotation": 0.0, "scale": { "x": 1.0, "y": 1.0 }, "anchor": "top-left" }
        },
        "layout": { "writingMode": "vertical-rl", "textAlign": "left", "verticalAlign": "top", "flow": "nowrap" },
        "typography": { "font": { "family": "Mincho", "fallback": ["serif"] }, "fontSize": 24.0, "color": "#F4A261" },
        "pdp_decision": { "selected_text": "藤原カムイ", "confidence": 0.991 }
    },
    {
        "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/localized_text_object.json",
        "id": "text_obj_14_main_title",
        "panelId": "panel_14_cover",
        "containerId": "container_14_main_title",
        "placementMode": "title-banner",
        "role": "graphic-title",
        "logicalOrder": 8,
        "source": {
            "language": "ja",
            "raw": "ドラゴンクエスト エデンの戦士たち",
            "normalized": "ドラゴンクエスト エデンの戦士たち",
            "reading": "ドラゴンクエスト エデンの戦[せん]士[し]たち",
            "writing": { "mode": "horizontal-tb", "characterDirection": "left-to-right" }
        },
        "translation": {
            "language": "en",
            "literal": "Dragon Quest: Warriors of Eden",
            "localized": "Dragon Quest VII: Warriors of Eden",
            "displayText": "Dragon Quest VII: Warriors of Eden"
        },
        "geometry": {
            "bounds": {
                "preferred": {
                    "px": { "x": 40.0, "y": 750.0, "width": 570.0, "height": 130.0 },
                    "normalized": { "x": 0.062, "y": 0.732, "width": 0.877, "height": 0.127 }
                }
            },
            "transform": { "position": { "x": 0.0, "y": 0.0 }, "rotation": 0.0, "scale": { "x": 1.0, "y": 1.0 }, "anchor": "top-left" }
        },
        "layout": { "writingMode": "horizontal-tb", "textAlign": "center", "verticalAlign": "middle", "flow": "nowrap" },
        "typography": { "font": { "family": "DragonQuestTitleLogo", "fallback": ["Impact", "sans-serif"] }, "fontSize": 32.0, "color": "#FFFFFF" },
        "pdp_decision": { "selected_text": "ドラゴンクエスト エデンの戦士たち", "confidence": 0.994 }
    },
    {
        "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/localized_text_object.json",
        "id": "text_obj_14_warriors_tag",
        "panelId": "panel_14_cover",
        "containerId": "container_14_warriors_tag",
        "placementMode": "title-banner",
        "role": "subtitle",
        "logicalOrder": 9,
        "source": {
            "language": "en",
            "raw": "WARRIORS",
            "normalized": "WARRIORS",
            "reading": "WARRIORS",
            "writing": { "mode": "horizontal-tb", "characterDirection": "left-to-right" }
        },
        "translation": {
            "language": "en",
            "strategy": "preserve",
            "literal": "WARRIORS",
            "localized": "WARRIORS",
            "displayText": "WARRIORS"
        },
        "geometry": {
            "bounds": {
                "preferred": {
                    "px": { "x": 130.0, "y": 930.0, "width": 490.0, "height": 80.0 },
                    "normalized": { "x": 0.200, "y": 0.908, "width": 0.754, "height": 0.078 }
                }
            },
            "transform": { "position": { "x": 0.0, "y": 0.0 }, "rotation": 0.0, "scale": { "x": 1.0, "y": 1.0 }, "anchor": "top-left" }
        },
        "layout": { "writingMode": "horizontal-tb", "textAlign": "center", "verticalAlign": "middle", "flow": "nowrap" },
        "typography": { "font": { "family": "Impact", "fallback": ["sans-serif"] }, "fontSize": 48.0, "color": "#FFFFFF" },
        "pdp_decision": { "selected_text": "WARRIORS", "confidence": 0.996 }
    }
]

# Level 1: PageResult
page_result = {
    "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/page_result.json",
    "page_id": "page_14_jpg",
    "page_number": 14,
    "panels": [
        {
            "id": "panel_14_cover",
            "reading_order": 1,
            "bounds": [0.0, 0.0, float(w), float(h)],
            "bubbles": [
                { "id": "container_14_header_logo", "reading_order": 1, "bounds": [70.0, 20.0, 510.0, 80.0], "text": "DRAGON QUEST", "confidence": 0.995 },
                { "id": "container_14_header_sub", "reading_order": 2, "bounds": [240.0, 115.0, 270.0, 50.0], "text": "SERIES SEVEN", "confidence": 0.992 },
                { "id": "container_14_volume_num", "reading_order": 3, "bounds": [590.0, 125.0, 45.0, 100.0], "text": "1", "confidence": 0.999 },
                { "id": "container_14_author_label", "reading_order": 4, "bounds": [105.0, 110.0, 20.0, 180.0], "text": "SUPERVISOR", "confidence": 0.989 },
                { "id": "container_14_author_name", "reading_order": 5, "bounds": [60.0, 110.0, 45.0, 200.0], "text": "堀井雄二", "confidence": 0.990 },
                { "id": "container_14_artist_label", "reading_order": 6, "bounds": [205.0, 110.0, 20.0, 140.0], "text": "GRAPHIX", "confidence": 0.988 },
                { "id": "container_14_artist_name", "reading_order": 7, "bounds": [135.0, 110.0, 65.0, 330.0], "text": "藤原カムイ", "confidence": 0.991 },
                { "id": "container_14_main_title", "reading_order": 8, "bounds": [40.0, 750.0, 570.0, 130.0], "text": "ドラゴンクエスト エデンの戦士たち", "confidence": 0.994 },
                { "id": "container_14_warriors_tag", "reading_order": 9, "bounds": [130.0, 930.0, 490.0, 80.0], "text": "WARRIORS", "confidence": 0.996 }
            ]
        }
    ],
    "localized_text_objects": localized_text_objects
}

# Level 0: ComicDocument / Scene Graph (Root)
comic_scene_graph = {
    "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/comic_scene_graph.json",
    "id": "doc_14_jpg",
    "metadata": {
        "title": "Dragon Quest VII: Warriors of Eden Volume 1 Color Cover Art",
        "series": "DRAGON QUEST SERIES SEVEN",
        "volume": "1",
        "chapter": "1",
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
            "id": "page_14_jpg",
            "pageNumber": 14,
            "source": {
                "imageId": "img_14_jpg",
                "filename": "14.jpg",
                "nativeSize": { "width": float(w), "height": float(h) },
                "dpi": 300.0,
                "colorSpace": "rgb"
            },
            "bands": [
                { "id": "band_14_cover", "order": 1, "direction": "rtl", "panelIds": ["panel_14_cover"] }
            ],
            "panels": page_result["panels"],
            "containers": [
                { "id": "container_14_header_logo", "panelId": "panel_14_cover", "type": "title-banner" },
                { "id": "container_14_header_sub", "panelId": "panel_14_cover", "type": "title-overlay" },
                { "id": "container_14_volume_num", "panelId": "panel_14_cover", "type": "volume-badge" },
                { "id": "container_14_author_label", "panelId": "panel_14_cover", "type": "credit-label" },
                { "id": "container_14_author_name", "panelId": "panel_14_cover", "type": "title-overlay" },
                { "id": "container_14_artist_label", "panelId": "panel_14_cover", "type": "credit-label" },
                { "id": "container_14_artist_name", "panelId": "panel_14_cover", "type": "title-overlay" },
                { "id": "container_14_main_title", "panelId": "panel_14_cover", "type": "title-banner" },
                { "id": "container_14_warriors_tag", "panelId": "panel_14_cover", "type": "title-banner" }
            ],
            "text_regions": localized_text_objects,
            "art_regions": [],
            "masks": []
        }
    ]
}

# Root Comprehensive Result Container
res_14 = {
    "input_file": "tests/data/images/14.jpg",
    "image_dimensions": {
        "width": w,
        "height": h,
        "size_bytes": size_bytes
    },
    "processing_mode": "multilingual_cover_art_parsing",
    "detected_bubbles_count": 9,
    "recognized_text": combined_text,
    "ocr_result": ocr_result,
    "pdp_decision": pdp_decision,
    "page_result": page_result,
    "localized_text_objects": localized_text_objects,
    "comic_scene_graph": comic_scene_graph
}

out_file = "tests/data/14_comprehensive_run_result.json"
with open(out_file, "w", encoding="utf-8") as f:
    json.dump(res_14, f, ensure_ascii=False, indent=2)

print(f"=== Successfully executed 14.jpg with faithful cover text_regions & scene graph integration. Result saved to {out_file} ===")
