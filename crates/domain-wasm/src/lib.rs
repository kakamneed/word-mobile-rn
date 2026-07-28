use wasm_bindgen::prelude::wasm_bindgen;

/// Execute the shared v1 domain protocol without adding adapter behavior.
#[wasm_bindgen]
pub fn execute_v1(request_json: &str) -> String {
    word_domain_protocol::v1::execute_v1(request_json)
}
