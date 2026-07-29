use serde_json::{json, Value};
use word_domain_protocol::v1::{execute_v1, ProtocolResponse, PROTOCOL_VERSION};

fn execute(request: Value) -> ProtocolResponse {
    serde_json::from_str(&execute_v1(&request.to_string())).expect("response envelope")
}

fn context() -> Value {
    json!({
        "nowUtc": "2026-07-29T00:01:00.000Z",
        "localDay": "2026-07-29",
        "sessionId": "session-1",
        "orderingSeed": "plan-1"
    })
}

fn lifecycle_snapshot() -> Value {
    json!({
        "schemaVersion": 1,
        "engineVersion": "phase-3",
        "sessionId": "session-1",
        "slotId": "guest:one",
        "localDay": "2026-07-29",
        "mode": "newWord",
        "state": "active",
        "dailySnapshotId": "daily-1",
        "planFingerprint": "plan-1",
        "sourceRefs": [{ "bookId": "book", "version": "1", "entrySourceId": "entry-1" }],
        "questionPlan": [{
            "questionId": "question-1",
            "questionType": "enToCnInput",
            "entrySourceId": "entry-1",
            "word": "orbit",
            "prompt": "orbit",
            "acceptedMeanings": ["path"],
            "choices": null,
            "correctChoiceLabel": null,
            "canonicalAnswer": "path",
            "questionIndex": 0,
            "totalQuestions": 1
        }],
        "acceptedAnswers": [],
        "currentQuestionIndex": 0,
        "ownerEpoch": 3,
        "startedAt": "2026-07-29T00:00:00.000Z",
        "updatedAt": "2026-07-29T00:00:00.000Z"
    })
}

fn lifecycle_request(command: &str, payload: Value) -> ProtocolResponse {
    execute(json!({
        "protocolVersion": 1,
        "command": command,
        "requestId": format!("request-{command}"),
        "context": context(),
        "payload": payload
    }))
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
    let malformed: ProtocolResponse =
        serde_json::from_str(&execute_v1("{")).expect("malformed input still returns JSON");
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
        let response: ProtocolResponse = serde_json::from_str(&execute_v1(request)).unwrap();
        assert_eq!(
            response.result.expect("canonical result"),
            serde_json::from_str::<Value>(expected).unwrap()
        );
        assert!(response.error.is_none());
    }
}

#[test]
fn transition_session_requires_an_answer_and_completes_the_last_question() {
    let unanswered = lifecycle_request(
        "transitionSession",
        json!({ "snapshot": lifecycle_snapshot(), "kind": "next", "now": "2026-07-29T00:01:00.000Z" }),
    );
    assert_eq!(unanswered.error.unwrap().code, "invalid_data");

    let mut snapshot = lifecycle_snapshot();
    snapshot["acceptedAnswers"] = json!([{
        "eventId": "accepted:submit-1",
        "submissionId": "submit-1",
        "sessionId": "session-1",
        "questionId": "question-1",
        "entrySourceId": "entry-1",
        "localDay": "2026-07-29",
        "mode": "newWord",
        "source": { "bookId": "book", "version": "1", "entrySourceId": "entry-1" },
        "questionFingerprint": "question-1",
        "userResponse": "path",
        "normalizedResponse": "path",
        "outcome": "correct",
        "acceptedAt": "2026-07-29T00:01:00.000Z",
        "elapsedMs": 50,
        "feedback": { "submitted": true, "outcome": "correct", "hintAvailable": false, "correctAnswer": "path" }
    }]);
    let response = lifecycle_request(
        "transitionSession",
        json!({ "snapshot": snapshot, "kind": "next", "now": "2026-07-29T00:02:00.000Z" }),
    );
    let result = response.result.expect("transition result");
    assert_eq!(result["state"], "completed");
    assert_eq!(result["currentQuestionIndex"], 1);
    assert_eq!(result["updatedAt"], "2026-07-29T00:02:00.000Z");
}

#[test]
fn resolve_dispute_returns_atomic_correct_resolution() {
    let response = lifecycle_request(
        "resolveDispute",
        json!({
            "snapshot": lifecycle_snapshot(),
            "questionId": "question-1",
            "proposedMeaning": "  Trajectory  ",
            "now": "2026-07-29T00:01:00.000Z"
        }),
    );
    let result = response.result.expect("dispute resolution");
    assert_eq!(result["snapshot"]["state"], "completed");
    assert_eq!(result["event"]["outcome"], "correct");
    assert_eq!(result["event"]["normalizedResponse"], "trajectory");
    assert_eq!(result["dispute"]["proposedMeaning"], "Trajectory");
    assert_eq!(
        result["dispute"]["source"],
        json!({ "bookId": "book", "version": "1" })
    );
    assert_eq!(result["completionFacts"].as_array().unwrap().len(), 1);
    assert_eq!(result["learningHistory"].as_array().unwrap().len(), 1);
}

#[test]
fn exclude_entry_returns_skipped_resolution_without_completion() {
    let response = lifecycle_request(
        "excludeEntry",
        json!({
            "snapshot": lifecycle_snapshot(),
            "entrySourceId": "entry-1",
            "now": "2026-07-29T00:01:00.000Z"
        }),
    );
    let result = response.result.expect("exclusion resolution");
    assert_eq!(result["snapshot"]["state"], "completed");
    assert_eq!(result["event"]["outcome"], "skipped");
    assert_eq!(result["exclusion"]["eventId"], "exclude:session-1:entry-1");
    assert!(result.get("completionFacts").is_none());
    assert!(result.get("learningHistory").is_none());
}

#[test]
fn abandon_session_marks_the_snapshot_without_dropping_history() {
    let response = lifecycle_request(
        "abandonSession",
        json!({ "snapshot": lifecycle_snapshot(), "now": "2026-07-29T00:03:00.000Z" }),
    );
    let result = response.result.expect("abandoned snapshot");
    assert_eq!(result["state"], "abandoned");
    assert_eq!(result["updatedAt"], "2026-07-29T00:03:00.000Z");
    assert_eq!(result["questionPlan"].as_array().unwrap().len(), 1);
}
