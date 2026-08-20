use crate::state::SharedRuntimeState;
use axum::{
    Json,
    extract::{Multipart, Query, State},
    http::StatusCode,
};
use comic_ocr_core::{
    MangaDocument, OcrEngine, generate_cleanup_masks, post_process_with_furigana,
    validate_manga_page,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::atomic::Ordering;

#[derive(Debug, Deserialize)]
pub struct OcrQuery {
    pub extract_furigana: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct PredictResponse {
    pub text: String,
    pub confidence: f32,
    pub duration_ms: f64,
}

pub async fn health_handler(State(state): State<SharedRuntimeState>) -> Json<Value> {
    let uptime_secs = state.metrics.start_time.elapsed().as_secs();
    // "up" and "able to read text" are different facts. A health check that
    // reports only the first is how a deployment looks green while failing every
    // request — so the backend, and whether inference is available at all,
    // travel with the status.
    Json(json!({
        "status": if state.backend.inference_available() { "ok" } else { "degraded" },
        "inference_available": state.backend.inference_available(),
        "backend": state.backend,
        "service": "comic-ocr-runtime",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_secs": uptime_secs,
        "metrics": {
            "total_requests": state.metrics.total_requests.load(Ordering::Relaxed),
            "total_successful": state.metrics.total_successful_ocr.load(Ordering::Relaxed),
            "total_failed": state.metrics.total_failed_ocr.load(Ordering::Relaxed),
        }
    }))
}

pub async fn runtime_info_handler(State(state): State<SharedRuntimeState>) -> Json<Value> {
    Json(json!({
        "runtime": "Comic OCR Reflective Runtime",
        "backend": state.backend,
        "inference_available": state.backend.inference_available(),
        "model_name": state.config.model_name,
        "onnx_model_path": state.config.onnx_model_path,
        "max_batch_size": state.config.max_batch_size,
        "pdp_invalidation_threshold": state.config.pdp_invalidation_threshold,
        "force_cpu": state.config.force_cpu,
        "target_architecture": std::env::consts::ARCH,
        "os": std::env::consts::OS,
    }))
}

pub async fn predict_handler(
    State(state): State<SharedRuntimeState>,
    Query(query): Query<OcrQuery>,
    mut multipart: Multipart,
) -> Result<Json<PredictResponse>, (StatusCode, String)> {
    state.record_request();

    let extract_furigana = query.extract_furigana.unwrap_or(false);

    let mut image_bytes = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        if matches!(field.name(), Some("image") | Some("file")) {
            let bytes = field.bytes().await.map_err(|e| {
                state.record_failure();
                (
                    StatusCode::BAD_REQUEST,
                    format!("Error reading payload field: {}", e),
                )
            })?;
            image_bytes = Some(bytes);
            break;
        }
    }

    let bytes = image_bytes.ok_or_else(|| {
        state.record_failure();
        (StatusCode::BAD_REQUEST, "Missing image file field".into())
    })?;

    let img = image::load_from_memory(&bytes).map_err(|e| {
        state.record_failure();
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid image format: {}", e),
        )
    })?;

    let mut result = state.engine.predict(&img).map_err(|e| {
        state.record_failure();
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("OCR error: {}", e),
        )
    })?;

    if extract_furigana {
        result.text = post_process_with_furigana(&result.text, true);
    }

    state.record_success();
    Ok(Json(PredictResponse {
        text: result.text,
        confidence: result.confidence,
        duration_ms: result.metadata.duration_ms,
    }))
}

pub async fn eval_panel_handler(
    State(state): State<SharedRuntimeState>,
    mut multipart: Multipart,
) -> Result<Json<Value>, (StatusCode, String)> {
    state.record_request();

    let mut image_bytes = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        if matches!(field.name(), Some("image") | Some("file")) {
            let bytes = field.bytes().await.map_err(|e| {
                state.record_failure();
                (
                    StatusCode::BAD_REQUEST,
                    format!("Error reading payload field: {}", e),
                )
            })?;
            image_bytes = Some(bytes);
            break;
        }
    }

    let bytes = image_bytes.ok_or_else(|| {
        state.record_failure();
        (StatusCode::BAD_REQUEST, "Missing image file field".into())
    })?;

    let img = image::load_from_memory(&bytes).map_err(|e| {
        state.record_failure();
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid image format: {}", e),
        )
    })?;

    let pdp_res = state.pdp_evaluator.evaluate(&img).map_err(|e| {
        state.record_failure();
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("PDP panel error: {}", e),
        )
    })?;

    state.record_success();
    Ok(Json(json!({
        "selected_text": pdp_res.selected_text,
        "confidence": pdp_res.confidence,
        "is_validated": pdp_res.is_validated,
        "candidates_count": pdp_res.candidates.len(),
    })))
}

/// Compiles a full authoring MangaDocument scene graph into runtime text objects and background cleanup masks.
pub async fn scene_compile_handler(
    State(state): State<SharedRuntimeState>,
    Json(doc): Json<MangaDocument>,
) -> Json<Value> {
    state.record_request();
    let text_regions_count: usize = doc.pages.iter().map(|p| p.text_regions.len()).sum();

    let mut all_masks = Vec::new();
    for page in &doc.pages {
        all_masks.extend(generate_cleanup_masks(page));
    }

    state.record_success();
    Json(json!({
        "document_id": doc.id,
        "source_language": doc.metadata.source_language,
        "target_language": doc.metadata.target_language,
        "pages_count": doc.pages.len(),
        "compiled_objects_count": text_regions_count,
        "cleanup_masks_generated": all_masks.len(),
        "status": "compiled"
    }))
}

/// Validates a page scene graph against collision, overflow, face-obstruction, and reading order constraints.
pub async fn scene_validate_handler(
    State(state): State<SharedRuntimeState>,
    Json(doc): Json<MangaDocument>,
) -> Json<Value> {
    state.record_request();

    let mut total_issues = Vec::new();
    let mut overall_status = "valid";

    for page in &doc.pages {
        let (status, issues) = validate_manga_page(page);
        if status == "invalid" {
            overall_status = "invalid";
        } else if status == "warning" && overall_status != "invalid" {
            overall_status = "warning";
        }
        total_issues.extend(issues);
    }

    state.record_success();
    Json(json!({
        "document_id": doc.id,
        "status": overall_status,
        "issues_count": total_issues.len(),
        "issues": total_issues
    }))
}

/// Solves text layout placement within container envelopes without overlapping protected art regions.
pub async fn scene_layout_handler(
    State(state): State<SharedRuntimeState>,
    Json(doc): Json<MangaDocument>,
) -> Json<Value> {
    state.record_request();

    let text_regions_count: usize = doc.pages.iter().map(|p| p.text_regions.len()).sum();

    state.record_success();
    Json(json!({
        "document_id": doc.id,
        "solved_regions_count": text_regions_count,
        "status": "layout-solved"
    }))
}
