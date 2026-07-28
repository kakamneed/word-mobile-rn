use serde_json::{json, Value};
use word_domain_protocol::v1::{execute_v1, ProtocolResponse, PROTOCOL_VERSION};

fn execute(request: Value) -> ProtocolResponse {
    serde_json::from_str(&execute_v1(&request.to_string())).expect("response envelope")
}

#[test]
fn successful_response_has_stable_envelope_and_exactly_one_body() {
    let response = execute(json!({
        "protocolVersion": 1,
        "command": "captureProgressSummary",
        "requestId": "req-1",
        "context": {
            "nowUtc": "2026-07-28T00:05:00Z",
            "localDay": "2026-07-28",
            "sessionId": "session-1",
            "orderingSeed": "seed-1"
        },
        "payload": {
            "expectedTotalQuestions": 0,
            "progress": { "current": 0, "total": 0 },
            "carryOver": { "todayCompleted": 0, "todayTotal": 0 },
            "results": []
        }
    }));

    assert_eq!(response.protocol_version, PROTOCOL_VERSION);
    assert_eq!(response.request_id, "req-1");
    assert_eq!(response.source.name, "word-mobile-domain");
    assert!(response.result.is_some());
    assert!(response.error.is_none());
}

#[test]
fn additive_unknown_fields_are_accepted() {
    let mut request: Value = serde_json::from_str(include_str!(
        "../../../fixtures/domain/v1/requests/wrong-word-identity.json"
    ))
    .unwrap();
    request["futureEnvelopeField"] = json!({ "v": 2 });
    request["context"]["futureContextField"] = json!(true);
    request["payload"]["futurePayloadField"] = json!([1, 2, 3]);

    let response = execute(request);
    assert!(response.result.is_some());
    assert!(response.error.is_none());
}

#[test]
fn unsupported_version_unknown_command_and_missing_field_use_stable_codes() {
    let unsupported = execute(json!({
        "protocolVersion": 2,
        "command": "captureWrongWords",
        "requestId": "bad-version",
        "context": {},
        "payload": {}
    }));
    assert_eq!(unsupported.error.unwrap().code, "unsupported_version");

    let unknown = execute(json!({
        "protocolVersion": 1,
        "command": "futureCommand",
        "requestId": "bad-command",
        "context": {},
        "payload": {}
    }));
    assert_eq!(unknown.error.unwrap().code, "unknown_command");

    let missing = execute(json!({
        "protocolVersion": 1,
        "command": "captureWrongWords",
        "requestId": "missing-context",
        "payload": {}
    }));
    assert_eq!(missing.error.unwrap().code, "missing_required_field");
}

#[test]
fn malformed_and_invalid_typed_data_never_escape_as_raw_exceptions() {
    let malformed: ProtocolResponse = serde_json::from_str(&execute_v1("{"))
        .expect("malformed input still returns JSON");
    let error = malformed.error.expect("structured malformed-json error");
    assert_eq!(error.code, "invalid_json");
    assert!(!error.message.contains("serde"));

    let invalid = execute(json!({
        "protocolVersion": 1,
        "command": "captureWrongWords",
        "requestId": "invalid-data",
        "context": {
            "nowUtc": "2026-07-28T00:00:00Z",
            "localDay": "2026-07-28",
            "sessionId": "session-1",
            "orderingSeed": "seed-1"
        },
        "payload": { "filter": "all", "entries": "not-an-array" }
    }));
    assert_eq!(invalid.error.unwrap().code, "invalid_data");
}

#[test]
fn every_canonical_fixture_dispatches_to_the_accepted_semantic_result() {
    for (request, expected) in [
        (include_str!("../../../fixtures/domain/v1/requests/study-newword.json"), include_str!("../../../fixtures/domain/v1/expected/study-newword.json")),
        (include_str!("../../../fixtures/domain/v1/requests/study-non-newword-modes.json"), include_str!("../../../fixtures/domain/v1/expected/study-non-newword-modes.json")),
        (include_str!("../../../fixtures/domain/v1/requests/study-post-submit-feedback.json"), include_str!("../../../fixtures/domain/v1/expected/study-post-submit-feedback.json")),
        (include_str!("../../../fixtures/domain/v1/requests/study-progress-summary.json"), include_str!("../../../fixtures/domain/v1/expected/study-progress-summary.json")),
        (include_str!("../../../fixtures/domain/v1/requests/study-resume-state.json"), include_str!("../../../fixtures/domain/v1/expected/study-resume-state.json")),
        (include_str!("../../../fixtures/domain/v1/requests/wrong-word-identity.json"), include_str!("../../../fixtures/domain/v1/expected/wrong-word-identity.json")),
        (include_str!("../../../fixtures/domain/v1/requests/report-local-day.json"), include_str!("../../../fixtures/domain/v1/expected/report-local-day.json")),
    ] {
        let response: ProtocolResponse = serde_json::from_str(&execute_v1(request)).unwrap();
        assert_eq!(
            response.result.expect("canonical result"),
            serde_json::from_str::<Value>(expected).unwrap()
        );
        assert!(response.error.is_none());
    }
}
