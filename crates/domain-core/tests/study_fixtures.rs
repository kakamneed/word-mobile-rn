use serde::Deserialize;
use serde_json::Value;
use word_domain_core::{
    question_progress, summarize_results, DomainContext, EntryExample, MeaningZh, QuestionBuilder,
    QuestionTypeWeight, SessionMode, StudyResult, WordForQuestion,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Envelope<T> {
    context: DomainContext,
    payload: T,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StudyPayload {
    entries: Vec<EntryPayload>,
    distractors: Vec<EntryPayload>,
    #[serde(default)]
    question_type_weights: Vec<QuestionTypeWeight>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EntryPayload {
    source_id: String,
    word: String,
    part_of_speech: Option<String>,
    #[serde(default)]
    frequency: f64,
    phonetic_us: Option<String>,
    phonetic_uk: Option<String>,
    #[serde(default)]
    meaning_details: Vec<MeaningZh>,
    #[serde(default)]
    meanings: Vec<String>,
    example_sentence: Option<String>,
    example_translation: Option<String>,
    #[serde(default)]
    cn_choice_distractors: Vec<String>,
    #[serde(default)]
    en_choice_distractors: Vec<String>,
}

impl EntryPayload {
    fn into_word(self) -> WordForQuestion {
        let meanings = if self.meaning_details.is_empty() {
            self.meanings
                .into_iter()
                .map(|meaning_cn| MeaningZh { pos: String::new(), meaning_cn, meaning_en: None })
                .collect()
        } else {
            self.meaning_details
        };
        let examples = self
            .example_sentence
            .map(|sentence_en| EntryExample {
                sentence_en,
                sentence_cn: self.example_translation.unwrap_or_default(),
            })
            .into_iter()
            .collect();
        WordForQuestion {
            source_id: self.source_id,
            word: self.word,
            part_of_speech: self.part_of_speech,
            frequency: self.frequency,
            phonetic_us: self.phonetic_us,
            phonetic_uk: self.phonetic_uk,
            meanings,
            examples,
            cn_choice_distractors: self.cn_choice_distractors,
            en_choice_distractors: self.en_choice_distractors,
        }
    }
}

fn fixture(path: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(path);
    serde_json::from_str(&std::fs::read_to_string(path).expect("read fixture")).expect("parse fixture")
}

#[test]
fn study_fixture_newword_matches_locked_type_major_four_question_contract() {
    let request: Envelope<StudyPayload> = serde_json::from_value(fixture(
        "fixtures/domain/v1/requests/study-newword.json",
    ))
    .expect("typed request");
    let expected = fixture("fixtures/domain/v1/expected/study-newword.json");
    let questions = QuestionBuilder::build_session_questions(
        &SessionMode::NewWord,
        &request.payload.entries.into_iter().map(EntryPayload::into_word).collect::<Vec<_>>(),
        &request.payload.distractors.into_iter().map(EntryPayload::into_word).collect::<Vec<_>>(),
        &request.context.session_id,
        &request.payload.question_type_weights,
    );

    assert_eq!(serde_json::to_value(questions).unwrap(), expected["questions"]);
}

#[test]
fn study_fixture_progress_deduplicates_question_identity_and_caps_total() {
    let request = fixture("fixtures/domain/v1/requests/study-progress-summary.json");
    let expected = fixture("fixtures/domain/v1/expected/study-progress-summary.json");
    let context: DomainContext = serde_json::from_value(request["context"].clone()).unwrap();
    let results: Vec<StudyResult> = serde_json::from_value(request["payload"]["results"].clone()).unwrap();
    let total = request["payload"]["expectedTotalQuestions"].as_u64().unwrap() as u32;

    assert_eq!(serde_json::to_value(question_progress(&results.iter().map(|r| r.question_id.clone()).collect::<Vec<_>>(), total)).unwrap(), expected["progress"]);
    assert_eq!(serde_json::to_value(summarize_results(&context.session_id, &results, total, &context.now_utc)).unwrap(), expected["summary"]);
}

