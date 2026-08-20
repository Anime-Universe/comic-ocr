use comic_ocr_core::{post_process_with_furigana, OcrEngine};
use comic_ocr_ort::OrtEngine;
use pyo3::prelude::*;
use std::sync::Arc;

#[pyclass]
pub struct PyComicOcr {
    engine: Arc<OrtEngine>,
    extract_furigana: bool,
}

#[pymethods]
impl PyComicOcr {
    #[new]
    #[pyo3(signature = (model_name="kha-white/manga-ocr-base", extract_furigana=false))]
    fn new(model_name: &str, extract_furigana: bool) -> PyResult<Self> {
        let engine = OrtEngine::new(model_name).with_furigana(extract_furigana);
        Ok(Self {
            engine: Arc::new(engine),
            extract_furigana,
        })
    }

    /// Predict text from raw image bytes (JPEG/PNG).
    fn predict(&self, image_bytes: &[u8]) -> PyResult<String> {
        let img = image::load_from_memory(image_bytes)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        let mut res = self
            .engine
            .predict(&img)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        if self.extract_furigana {
            res.text = post_process_with_furigana(&res.text, true);
        }

        Ok(res.text)
    }

    /// Batch predict text from a list of image byte buffers.
    fn predict_batch(&self, py: Python, images: Vec<Vec<u8>>) -> PyResult<Vec<String>> {
        py.allow_threads(|| {
            let mut results = Vec::with_capacity(images.len());
            for buf in images {
                let img = image::load_from_memory(&buf)
                    .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
                let mut res = self
                    .engine
                    .predict(&img)
                    .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

                if self.extract_furigana {
                    res.text = post_process_with_furigana(&res.text, true);
                }
                results.push(res.text);
            }
            Ok(results)
        })
    }
}

#[pymodule]
fn comic_ocr_rs(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyComicOcr>()?;
    Ok(())
}
