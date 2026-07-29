use serde_json::Value;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn wasm_adapter_matches_shared_dispatcher_and_canonical_results() {
    for (request, expected) in [
        (
            include_str!("../../../fixtures/domain/v1/requests/study-newword.json"),
            include_str!("../../../fixtures/domain/v1/expected/study-newword.json"),
        ),
        (
            include_str!("../../../fixtures/domain/v1/requests/study-non-newword-modes.json"),
            include_str!("../../../fixtures/domain/v1/expected/study-non-newword-modes.json"),
        ),
        (
            include_str!("../../../fixtures/domain/v1/requests/study-post-submit-feedback.json"),
            include_str!("../../../fixtures/domain/v1/expected/study-post-submit-feedback.json"),
        ),
        (
            include_str!("../../../fixtures/domain/v1/requests/study-progress-summary.json"),
            include_str!("../../../fixtures/domain/v1/expected/study-progress-summary.json"),
        ),
        (
            include_str!("../../../fixtures/domain/v1/requests/study-resume-state.json"),
            include_str!("../../../fixtures/domain/v1/expected/study-resume-state.json"),
        ),
        (
            include_str!("../../../fixtures/domain/v1/requests/wrong-word-identity.json"),
            include_str!("../../../fixtures/domain/v1/expected/wrong-word-identity.json"),
        ),
        (
            include_str!("../../../fixtures/domain/v1/requests/report-local-day.json"),
            include_str!("../../../fixtures/domain/v1/expected/report-local-day.json"),
        ),
        (
            include_str!("../../../fixtures/domain/v1/requests/lifecycle-transition.json"),
            include_str!("../../../fixtures/domain/v1/expected/lifecycle-transition.json"),
        ),
        (
            include_str!("../../../fixtures/domain/v1/requests/lifecycle-dispute.json"),
            include_str!("../../../fixtures/domain/v1/expected/lifecycle-dispute.json"),
        ),
        (
            include_str!("../../../fixtures/domain/v1/requests/lifecycle-exclusion.json"),
            include_str!("../../../fixtures/domain/v1/expected/lifecycle-exclusion.json"),
        ),
        (
            include_str!("../../../fixtures/domain/v1/requests/lifecycle-abandon.json"),
            include_str!("../../../fixtures/domain/v1/expected/lifecycle-abandon.json"),
        ),
    ] {
        let wasm: Value = serde_json::from_str(&word_domain_wasm::execute_v1(request)).unwrap();
        let native: Value =
            serde_json::from_str(&word_domain_protocol::v1::execute_v1(request)).unwrap();
        let expected: Value = serde_json::from_str(expected).unwrap();

        assert_eq!(wasm, native);
        assert_eq!(wasm["result"], expected);
        assert!(wasm.get("error").is_none());
    }
}
