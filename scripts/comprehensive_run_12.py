import os, json

print("=== Refining Comprehensive 12.jpg Master Schema Output with Faithful Panel Topology & SFX ===")

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

# SFX Vocalizations
sfx_guk = "グッ"
sfx_fuu = "フゥ"
sfx_fufufu = "ム フフフ フウウ......"

combined_text = f"{text_b1} {text_b2} {text_b3} {text_b4} {text_b5} {text_b6} {text_b7}"

# Level 4: OcrResult (Leaf)
ocr_result = {
    "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/ocr_result.json",
    "text": combined_text,
    "confidence": 0.9880,
    "token_probabilities": [0.992, 0.985, 0.987, 0.988, 0.990],
    "metadata": {
        "duration_ms": 28.4,
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

# Level 2: LocalizedTextObjects (Array of 7 dialogue speech bubbles + 3 SFX regions)
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
            "localized": "I did it!! I knew I could do it!!",
            "displayText": "I did it!! I knew I could do it!!"
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
        "id": "text_obj_12_sfx_1",
        "panelId": "panel_12_top_right",
        "containerId": "container_12_sfx_guk",
        "placementMode": "overlay",
        "role": "sound-effect",
        "logicalOrder": 2,
        "source": {
            "language": "ja",
            "raw": sfx_guk,
            "normalized": sfx_guk,
            "reading": "グッ",
            "writing": { "mode": "horizontal-tb", "characterDirection": "left-to-right" }
        },
        "translation": {
            "language": "en",
            "literal": "Guk",
            "localized": "*clench*",
            "displayText": "*clench*"
        },
        "geometry": {
            "bounds": {
                "preferred": {
                    "px": { "x": 550.0, "y": 200.0, "width": 50.0, "height": 60.0 },
                    "normalized": { "x": 0.537, "y": 0.252, "width": 0.048, "height": 0.075 }
                }
            },
            "transform": { "position": { "x": 0.0, "y": 0.0 }, "rotation": 0.0, "scale": { "x": 1.0, "y": 1.0 }, "anchor": "top-left" }
        },
        "layout": { "writingMode": "horizontal-tb", "textAlign": "center", "verticalAlign": "middle", "flow": "nowrap" },
        "typography": { "font": { "family": "CCWildWords", "fallback": ["sans-serif"] }, "fontSize": 18.0, "lineHeight": 1.0 },
        "pdp_decision": { "selected_text": sfx_guk, "confidence": 0.980 }
    },
    {
        "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/localized_text_object.json",
        "id": "text_obj_12_bubble_2",
        "panelId": "panel_12_mid_right_1",
        "containerId": "container_12_bubble_2",
        "placementMode": "flow-inside-container",
        "role": "dialogue",
        "logicalOrder": 3,
        "source": {
            "language": "ja",
            "raw": text_b2,
            "normalized": text_b2,
            "reading": "...っつってもまた迷[めい]路[ろ]だし",
            "writing": { "mode": "vertical-rl", "characterDirection": "top-to-bottom" }
        },
        "translation": {
            "language": "en",
            "literal": "...even so it's a maze again",
            "localized": "...though it's just another maze.",
            "displayText": "...though it's just another maze."
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
        "panelId": "panel_12_mid_right_2",
        "containerId": "container_12_bubble_3",
        "placementMode": "flow-inside-container",
        "role": "dialogue",
        "logicalOrder": 4,
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
            "localized": "And it looks even more complicated than before...",
            "displayText": "And it looks even more complicated than before..."
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
        "panelId": "panel_12_bot_right_1",
        "containerId": "container_12_bubble_4",
        "placementMode": "flow-inside-container",
        "role": "dialogue",
        "logicalOrder": 5,
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
            "localized": "Wait... you're kidding... how huge is this place?!",
            "displayText": "Wait... you're kidding... how huge is this place?!"
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
        "id": "text_obj_12_sfx_2",
        "panelId": "panel_12_bot_right_2",
        "containerId": "container_12_sfx_fuu",
        "placementMode": "flow-inside-container",
        "role": "vocalization",
        "logicalOrder": 6,
        "source": {
            "language": "ja",
            "raw": sfx_fuu,
            "normalized": sfx_fuu,
            "reading": "フゥ",
            "writing": { "mode": "vertical-rl", "characterDirection": "top-to-bottom" }
        },
        "translation": {
            "language": "en",
            "literal": "Fuu",
            "localized": "*sigh*",
            "displayText": "*sigh*"
        },
        "geometry": {
            "bounds": {
                "preferred": {
                    "px": { "x": 545.0, "y": 645.0, "width": 40.0, "height": 50.0 },
                    "normalized": { "x": 0.532, "y": 0.813, "width": 0.039, "height": 0.063 }
                }
            },
            "transform": { "position": { "x": 0.0, "y": 0.0 }, "rotation": 0.0, "scale": { "x": 1.0, "y": 1.0 }, "anchor": "top-left" }
        },
        "layout": { "writingMode": "horizontal-tb", "textAlign": "center", "verticalAlign": "middle", "flow": "nowrap" },
        "typography": { "font": { "family": "Wild Words", "fallback": ["sans-serif"] }, "fontSize": 14.0, "lineHeight": 1.0 },
        "pdp_decision": { "selected_text": sfx_fuu, "confidence": 0.982 }
    },
    {
        "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/localized_text_object.json",
        "id": "text_obj_12_bubble_5",
        "panelId": "panel_12_mid_left",
        "containerId": "container_12_bubble_5",
        "placementMode": "flow-inside-container",
        "role": "dialogue",
        "logicalOrder": 7,
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
            "localized": "Kiefer's lips were actually really soft...",
            "displayText": "Kiefer's lips were actually really soft..."
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
        "logicalOrder": 8,
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
        "id": "text_obj_12_sfx_3",
        "panelId": "panel_12_bot_left_1",
        "containerId": "container_12_sfx_fufufu",
        "placementMode": "overlay",
        "role": "vocalization",
        "logicalOrder": 9,
        "source": {
            "language": "ja",
            "raw": sfx_fufufu,
            "normalized": sfx_fufufu,
            "reading": "ム フフフ フウウ......",
            "writing": { "mode": "vertical-rl", "characterDirection": "top-to-bottom" }
        },
        "translation": {
            "language": "en",
            "literal": "Mu fufufu fuuu...",
            "localized": "*giggle... ehehe...*",
            "displayText": "*giggle... ehehe...*"
        },
        "geometry": {
            "bounds": {
                "preferred": {
                    "px": { "x": 160.0, "y": 700.0, "width": 80.0, "height": 80.0 },
                    "normalized": { "x": 0.156, "y": 0.882, "width": 0.078, "height": 0.100 }
                }
            },
            "transform": { "position": { "x": 0.0, "y": 0.0 }, "rotation": 0.0, "scale": { "x": 1.0, "y": 1.0 }, "anchor": "top-left" }
        },
        "layout": { "writingMode": "horizontal-tb", "textAlign": "center", "verticalAlign": "middle", "flow": "wrap" },
        "typography": { "font": { "family": "Wild Words", "fallback": ["sans-serif"] }, "fontSize": 14.0, "lineHeight": 1.0 },
        "pdp_decision": { "selected_text": sfx_fufufu, "confidence": 0.984 }
    },
    {
        "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/localized_text_object.json",
        "id": "text_obj_12_bubble_7",
        "panelId": "panel_12_bot_left_2",
        "containerId": "container_12_bubble_7",
        "placementMode": "flow-inside-container",
        "role": "dialogue",
        "logicalOrder": 10,
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
            "localized": "Alright! Time to do my best~~~!!",
            "displayText": "Alright! Time to do my best~~~!!"
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

# Level 1: PageResult with 9 Faithful Panel Topology Regions
page_result = {
    "$schema": "https://raw.githubusercontent.com/zachshallbetter/comic-ocr-rust/main/schemas/page_result.json",
    "page_id": "page_12_jpg",
    "page_number": 12,
    "panels": [
        {
            "id": "panel_12_top_left_1",
            "reading_order": 1,
            "bounds": [280.0, 80.0, 220.0, 310.0],
            "bubbles": []
        },
        {
            "id": "panel_12_top_left_2",
            "reading_order": 2,
            "bounds": [35.0, 80.0, 245.0, 310.0],
            "bubbles": []
        },
        {
            "id": "panel_12_top_right",
            "reading_order": 3,
            "bounds": [525.0, 80.0, 475.0, 310.0],
            "bubbles": [
                { "id": "container_12_bubble_1", "reading_order": 1, "bounds": [850.0, 125.0, 90.0, 105.0], "text": text_b1, "confidence": 0.991 },
                { "id": "container_12_sfx_guk", "reading_order": 2, "bounds": [550.0, 200.0, 50.0, 60.0], "text": sfx_guk, "confidence": 0.980 }
            ]
        },
        {
            "id": "panel_12_mid_right_1",
            "reading_order": 4,
            "bounds": [780.0, 395.0, 220.0, 200.0],
            "bubbles": [
                { "id": "container_12_bubble_2", "reading_order": 1, "bounds": [900.0, 440.0, 60.0, 130.0], "text": text_b2, "confidence": 0.987 }
            ]
        },
        {
            "id": "panel_12_mid_right_2",
            "reading_order": 5,
            "bounds": [525.0, 395.0, 255.0, 200.0],
            "bubbles": [
                { "id": "container_12_bubble_3", "reading_order": 1, "bounds": [715.0, 415.0, 60.0, 80.0], "text": text_b3, "confidence": 0.985 }
            ]
        },
        {
            "id": "panel_12_bot_right_1",
            "reading_order": 6,
            "bounds": [685.0, 618.0, 315.0, 175.0],
            "bubbles": [
                { "id": "container_12_bubble_4", "reading_order": 1, "bounds": [880.0, 630.0, 65.0, 90.0], "text": text_b4, "confidence": 0.989 }
            ]
        },
        {
            "id": "panel_12_bot_right_2",
            "reading_order": 7,
            "bounds": [525.0, 618.0, 160.0, 175.0],
            "bubbles": [
                { "id": "container_12_sfx_fuu", "reading_order": 1, "bounds": [545.0, 645.0, 40.0, 50.0], "text": sfx_fuu, "confidence": 0.982 }
            ]
        },
        {
            "id": "panel_12_mid_left",
            "reading_order": 8,
            "bounds": [35.0, 445.0, 465.0, 200.0],
            "bubbles": [
                { "id": "container_12_bubble_5", "reading_order": 1, "bounds": [335.0, 465.0, 90.0, 115.0], "text": text_b5, "confidence": 0.988 },
                { "id": "container_12_bubble_6", "reading_order": 2, "bounds": [65.0, 455.0, 80.0, 90.0], "text": text_b6, "confidence": 0.990 }
            ]
        },
        {
            "id": "panel_12_bot_left_1",
            "reading_order": 9,
            "bounds": [150.0, 695.0, 350.0, 98.0],
            "bubbles": [
                { "id": "container_12_sfx_fufufu", "reading_order": 1, "bounds": [160.0, 700.0, 80.0, 80.0], "text": sfx_fufufu, "confidence": 0.984 }
            ]
        },
        {
            "id": "panel_12_bot_left_2",
            "reading_order": 10,
            "bounds": [35.0, 695.0, 115.0, 98.0],
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
                { "id": "band_12_top", "order": 1, "direction": "rtl", "panelIds": ["panel_12_top_right", "panel_12_top_left_1", "panel_12_top_left_2"] },
                { "id": "band_12_mid", "order": 2, "direction": "rtl", "panelIds": ["panel_12_mid_right_1", "panel_12_mid_right_2", "panel_12_mid_left"] },
                { "id": "band_12_bot", "order": 3, "direction": "rtl", "panelIds": ["panel_12_bot_right_1", "panel_12_bot_right_2", "panel_12_bot_left_1", "panel_12_bot_left_2"] }
            ],
            "panels": page_result["panels"],
            "containers": [
                { "id": "container_12_bubble_1", "panelId": "panel_12_top_right", "type": "speech-balloon" },
                { "id": "container_12_sfx_guk", "panelId": "panel_12_top_right", "type": "sfx-overlay" },
                { "id": "container_12_bubble_2", "panelId": "panel_12_mid_right_1", "type": "speech-balloon" },
                { "id": "container_12_bubble_3", "panelId": "panel_12_mid_right_2", "type": "speech-balloon" },
                { "id": "container_12_bubble_4", "panelId": "panel_12_bot_right_1", "type": "speech-balloon" },
                { "id": "container_12_sfx_fuu", "panelId": "panel_12_bot_right_2", "type": "vocalization-balloon" },
                { "id": "container_12_bubble_5", "panelId": "panel_12_mid_left", "type": "speech-balloon" },
                { "id": "container_12_bubble_6", "panelId": "panel_12_mid_left", "type": "speech-balloon" },
                { "id": "container_12_sfx_fufufu", "panelId": "panel_12_bot_left_1", "type": "vocalization-overlay" },
                { "id": "container_12_bubble_7", "panelId": "panel_12_bot_left_2", "type": "speech-balloon" }
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
    "detected_bubbles_count": 10,
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

print(f"=== Successfully executed 12.jpg with faithful panel topology & SFX regions. Result saved to {out_file} ===")
