#[test]
fn end_to_end_native_generation_against_real_graphs() {
    // Skips rather than fails when the environment cannot run it, and says
    // WHICH precondition was missing. A skip that does not name its reason is
    // indistinguishable from a pass, and this one skips for two very different
    // reasons: no model directory, or no usable ONNX Runtime.
    //
    // As of 2026-08-20 this machine hits the second: ort 2.0.0-rc.13 refuses the
    // onnxruntime 1.24.1 that ships with the Python package (BadVersion), and no
    // matching dylib is installed. The wiring below is therefore COMPILED and
    // unit-tested but has never executed here.
    let dir = match std::env::var("COMIC_OCR_ONNX_DIR") {
        Ok(d) if !d.trim().is_empty() => d,
        _ => {
            eprintln!("SKIP: COMIC_OCR_ONNX_DIR not set — no model directory to load");
            return;
        }
    };
    if std::env::var("ORT_DYLIB_PATH").is_err() {
        eprintln!("SKIP: ORT_DYLIB_PATH not set — no ONNX Runtime to load the graphs with");
        return;
    }
    unsafe { std::env::set_var("COMIC_OCR_ONNX_DIR", &dir) };
    let engine = comic_ocr_ort::OrtEngine::new("native-e2e");
    assert!(engine.generator.is_some(), "the model directory must load");
    let img = image::open("assets/examples/00.jpg").expect("test crop");
    let out = comic_ocr_core::types::OcrEngine::predict(&engine, &img).expect("a reading");
    println!("TEXT={}", out.text);
    println!("CONF={:.4}", out.confidence);
    assert!(
        !out.text.is_empty(),
        "the native loop must produce a reading"
    );
    assert!(
        out.confidence > 0.0 && out.confidence <= 1.0,
        "confidence must be a real probability"
    );
}
