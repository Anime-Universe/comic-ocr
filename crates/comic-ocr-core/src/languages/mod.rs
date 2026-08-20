pub mod en;
pub mod jp;

pub use en::post_process_en;
pub use jp::post_process_jp;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    Japanese,
    English,
}

pub fn post_process_for_language(
    input: &str,
    language: Language,
    extract_furigana: bool,
) -> String {
    match language {
        Language::Japanese => post_process_jp(input, extract_furigana),
        Language::English => post_process_en(input),
    }
}
