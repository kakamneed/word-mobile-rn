use wasm_bindgen::prelude::wasm_bindgen;

/// Execute the shared v1 domain protocol without adding adapter behavior.
#[wasm_bindgen]
pub fn execute_v1(request_json: &str) -> String {
    word_domain_protocol::v1::execute_v1(request_json)
}

#[cfg(test)]
mod tests {
    use super::execute_v1;

    #[test]
    fn phase6_commands_remain_adapter_free() {
        let request = include_str!("../../../fixtures/domain/v1/requests/phase6-six-modes.json");
        let response: serde_json::Value =
            serde_json::from_str(&execute_v1(request)).expect("parse WASM adapter response");
        assert!(response.get("error").is_none());
        assert_eq!(response["result"]["planUnit"], "question");
    }
}
