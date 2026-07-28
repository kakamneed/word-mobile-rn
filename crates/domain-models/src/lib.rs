//! Platform- and persistence-free JSON domain contracts.

pub mod projections;
pub mod study;

pub use projections::*;
pub use study::*;

/// Temporary namespace compatibility for native crates migrating from
/// `word_storage_core::models::*` to direct domain-model imports.
pub mod models {
    pub use crate::{projections::*, study::*};
}

#[cfg(test)]
mod tests {
    use super::{AnsweredStudyQuestion, SessionProgress, StudyQuestion, StudySession};
    use serde_json::{json, Value};

    fn fixture(relative_path: &str) -> Value {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(relative_path);
        serde_json::from_str(&std::fs::read_to_string(path).expect("read domain fixture"))
            .expect("parse domain fixture")
    }

    fn assert_semantic_round_trip<T>(value: &Value)
    where
        T: serde::de::DeserializeOwned + serde::Serialize,
    {
        let model: T = serde_json::from_value(value.clone()).expect("deserialize canonical DTO");
        assert_eq!(
            serde_json::to_value(model).expect("serialize canonical DTO"),
            *value
        );
    }

    #[test]
    fn resume_fixture_round_trips_through_canonical_models() {
        let fixture = fixture("fixtures/domain/v1/expected/study-resume-state.json");
        assert_semantic_round_trip::<StudySession>(&fixture["session"]);
        assert_semantic_round_trip::<StudyQuestion>(&fixture["currentQuestion"]);
        assert_semantic_round_trip::<Vec<AnsweredStudyQuestion>>(&fixture["answeredQuestions"]);
        assert_semantic_round_trip::<SessionProgress>(&fixture["progress"]);
    }

    #[test]
    fn canonical_models_accept_unknown_additive_fields() {
        let fixture = fixture("fixtures/domain/v1/expected/study-resume-state.json");
        let mut question = fixture["currentQuestion"].clone();
        question
            .as_object_mut()
            .expect("question object")
            .insert("futureProtocolField".to_string(), json!({ "version": 2 }));

        serde_json::from_value::<StudyQuestion>(question)
            .expect("unknown additive fields remain forward compatible");
    }
}
