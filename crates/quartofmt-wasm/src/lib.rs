use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn format_qmd(input: &str, line_width: Option<usize>) -> String {
    quartofmt::format(input, line_width)
}

// Optional: expose tokenizer/AST for debugging
#[wasm_bindgen]
pub fn tokenize_debug(input: &str) -> String {
    // return a simple debug string; or serialize to JSON if you add serde
    let tree = quartofmt::parse(input);
    format!("{tree:#?}")
}
