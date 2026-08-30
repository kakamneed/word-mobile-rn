use serde::Deserialize;
use serde_json::Value;
use word_domain_core::{
    project_report, project_wrong_words, DomainContext, ReportHistoryInput,
    WrongWordProjectionInput,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Envelope<T> {
    context: DomainContext,
    payload: T,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WrongPayload {
    filter: String,
    entries: Vec<WrongWordProjectionInput>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReportPayload {
    learned_count: u64,
    history: Vec<ReportHistoryInput>,
}

fn fixture(path: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path);
    serde_json::from_str(&std::fs::read_to_string(path).expect("read fixture"))
        .expect("parse fixture")
}

#[test]
fn projection_fixture_preserves_real_wrong_word_identity_and_priority() {
    let request: Envelope<WrongPayload> = serde_json::from_value(fixture(
        "fixtures/domain/v1/requests/wrong-word-identity.json",
    ))
    .unwrap();
    let expected = fixture("fixtures/domain/v1/expected/wrong-word-identity.json");

    assert_eq!(
        serde_json::to_value(project_wrong_words(
            &request.context,
            &request.payload.entries,
            &request.payload.filter,
        ))
        .unwrap(),
        expected["entries"]
    );
}

#[test]
fn projection_fixture_uses_explicit_local_day_and_filters_zero_answers() {
    let request: Envelope<ReportPayload> =
        serde_json::from_value(fixture("fixtures/domain/v1/requests/report-local-day.json"))
            .unwrap();
    let expected = fixture("fixtures/domain/v1/expected/report-local-day.json");

    assert_eq!(
        serde_json::to_value(project_report(
            &request.context,
            &request.payload.history,
            request.payload.learned_count,
        ))
        .unwrap(),
        expected["report"]
    );
}
