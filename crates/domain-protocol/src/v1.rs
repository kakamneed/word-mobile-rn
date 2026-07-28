use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Map, Value};
use word_domain_core::{
    project_report, project_wrong_words, summarize_results, AnswerEvaluator, DomainContext,
    QuestionBuilder, ReportHistoryInput, WordForQuestion, WrongWordProjectionInput,
};
use word_domain_models::{
    AnsweredStudyQuestion, EntryExample, MeaningZh, QuestionTypeWeight, SessionMode,
    SessionProgress, StartSessionEntryPayload, StartSessionRequest, StartSessionResponse,
    StudyAnswer, StudyQuestion, StudyResult, StudySession,
};

pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceIdentity {
    pub name: String,
    pub package: String,
    pub version: String,
}

impl Default for SourceIdentity {
    fn default() -> Self {
        Self {
            name: "word-mobile-domain".to_string(),
            package: env!("CARGO_PKG_NAME").to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolResponse {
    pub protocol_version: u32,
    pub request_id: String,
    pub source: SourceIdentity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ProtocolError>,
}

impl ProtocolResponse {
    fn success(request_id: String, result: Value) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            request_id,
            source: SourceIdentity::default(),
            result: Some(result),
            error: None,
        }
    }

    fn failure(
        request_id: String,
        code: &'static str,
        message: &'static str,
        details: Option<Value>,
    ) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            request_id,
            source: SourceIdentity::default(),
            result: None,
            error: Some(ProtocolError {
                code: code.to_string(),
                message: message.to_string(),
                details,
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    CaptureNewWord,
    CaptureNonNewWordModes,
    CapturePostSubmitFeedback,
    CaptureProgressSummary,
    CaptureResumeState,
    CaptureWrongWords,
    CaptureReport,
    BuildSession,
    EvaluateAnswer,
    CompleteSession,
}

impl Command {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "captureNewWord" => Some(Self::CaptureNewWord),
            "captureNonNewWordModes" => Some(Self::CaptureNonNewWordModes),
            "capturePostSubmitFeedback" => Some(Self::CapturePostSubmitFeedback),
            "captureProgressSummary" => Some(Self::CaptureProgressSummary),
            "captureResumeState" => Some(Self::CaptureResumeState),
            "captureWrongWords" => Some(Self::CaptureWrongWords),
            "captureReport" => Some(Self::CaptureReport),
            "buildSession" => Some(Self::BuildSession),
            "evaluateAnswer" => Some(Self::EvaluateAnswer),
            "completeSession" => Some(Self::CompleteSession),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestEnvelope {
    protocol_version: u32,
    #[serde(rename = "command")]
    _command: String,
    request_id: String,
    context: DomainContext,
    payload: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StudyFixturePayload {
    entries: Vec<StartSessionEntryPayload>,
    #[serde(default)]
    distractors: Vec<StartSessionEntryPayload>,
    #[serde(default)]
    question_type_weights: Vec<QuestionTypeWeight>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProgressSummaryPayload {
    expected_total_questions: u32,
    progress: SessionProgress,
    carry_over: Value,
    results: Vec<StudyResult>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WrongWordsPayload {
    filter: String,
    entries: Vec<WrongWordProjectionInput>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReportPayload {
    learned_count: u64,
    history: Vec<ReportHistoryInput>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EvaluateAnswerPayload {
    question: StudyQuestion,
    answer: StudyAnswer,
    #[serde(default)]
    answered_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompleteSessionPayload {
    session: StudySession,
    results: Vec<StudyResult>,
    #[serde(default)]
    completed_at: Option<String>,
}

/// Execute one v1 request. This is the only JSON-to-domain dispatch boundary used by adapters.
pub fn execute_v1(input: &str) -> String {
    let response = execute(input);
    serde_json::to_string(&response).unwrap_or_else(|_| {
        r#"{"protocolVersion":1,"requestId":"","source":{"name":"word-mobile-domain","package":"word-domain-protocol","version":"0.1.0"},"error":{"code":"serialization_error","message":"The protocol response could not be serialized."}}"#.to_string()
    })
}

fn execute(input: &str) -> ProtocolResponse {
    let value: Value = match serde_json::from_str(input) {
        Ok(value) => value,
        Err(_) => {
            return ProtocolResponse::failure(
                String::new(),
                "invalid_json",
                "The request is not valid JSON.",
                None,
            )
        }
    };
    let request_id = value
        .get("requestId")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let Some(object) = value.as_object() else {
        return ProtocolResponse::failure(
            request_id,
            "invalid_data",
            "The request envelope must be an object.",
            None,
        );
    };
    for field in [
        "protocolVersion",
        "command",
        "requestId",
        "context",
        "payload",
    ] {
        if !object.contains_key(field) {
            return ProtocolResponse::failure(
                request_id,
                "missing_required_field",
                "A required request field is missing.",
                Some(json!({ "field": field })),
            );
        }
    }
    let Some(version) = object.get("protocolVersion").and_then(Value::as_u64) else {
        return ProtocolResponse::failure(
            request_id,
            "invalid_data",
            "The protocol version must be an integer.",
            Some(json!({ "field": "protocolVersion" })),
        );
    };
    if version != u64::from(PROTOCOL_VERSION) {
        return ProtocolResponse::failure(
            request_id,
            "unsupported_version",
            "The requested protocol version is not supported.",
            Some(json!({ "supported": [PROTOCOL_VERSION], "received": version })),
        );
    }
    let Some(command_name) = object.get("command").and_then(Value::as_str) else {
        return ProtocolResponse::failure(
            request_id,
            "invalid_data",
            "The command must be a string.",
            Some(json!({ "field": "command" })),
        );
    };
    let Some(command) = Command::parse(command_name) else {
        return ProtocolResponse::failure(
            request_id,
            "unknown_command",
            "The requested command is not supported.",
            Some(json!({ "command": command_name })),
        );
    };
    let envelope: RequestEnvelope = match serde_json::from_value(value) {
        Ok(envelope) => envelope,
        Err(error) => return decode_failure(request_id, &error),
    };
    if envelope.protocol_version != PROTOCOL_VERSION
        || envelope.request_id.trim().is_empty()
        || envelope.context.now_utc.trim().is_empty()
        || envelope.context.local_day.trim().is_empty()
        || envelope.context.session_id.trim().is_empty()
        || envelope.context.ordering_seed.trim().is_empty()
    {
        return ProtocolResponse::failure(
            envelope.request_id,
            "invalid_data",
            "Required request values must be non-empty and valid.",
            None,
        );
    }
    match dispatch(command, &envelope.context, envelope.payload) {
        Ok(result) => ProtocolResponse::success(envelope.request_id, result),
        Err(error) => ProtocolResponse {
            request_id: envelope.request_id,
            ..error
        },
    }
}

fn decode_failure(request_id: String, error: &serde_json::Error) -> ProtocolResponse {
    let code = if error.to_string().starts_with("missing field") {
        "missing_required_field"
    } else {
        "invalid_data"
    };
    ProtocolResponse::failure(
        request_id,
        code,
        if code == "missing_required_field" {
            "A required request field is missing."
        } else {
            "The request contains invalid data."
        },
        None,
    )
}

fn typed_payload<T: DeserializeOwned>(payload: Value) -> Result<T, ProtocolResponse> {
    serde_json::from_value(payload).map_err(|error| decode_failure(String::new(), &error))
}

fn dispatch(
    command: Command,
    context: &DomainContext,
    payload: Value,
) -> Result<Value, ProtocolResponse> {
    match command {
        Command::CaptureNewWord => capture_new_word(context, typed_payload(payload)?),
        Command::CaptureNonNewWordModes => {
            capture_non_new_word_modes(context, typed_payload(payload)?)
        }
        Command::CapturePostSubmitFeedback => {
            capture_post_submit_feedback(context, typed_payload(payload)?)
        }
        Command::CaptureProgressSummary => {
            capture_progress_summary(context, typed_payload(payload)?)
        }
        Command::CaptureResumeState => capture_resume_state(context, typed_payload(payload)?),
        Command::CaptureWrongWords => capture_wrong_words(context, typed_payload(payload)?),
        Command::CaptureReport => capture_report(context, typed_payload(payload)?),
        Command::BuildSession => build_session(context, typed_payload(payload)?),
        Command::EvaluateAnswer => evaluate_answer(context, typed_payload(payload)?),
        Command::CompleteSession => complete_session(context, typed_payload(payload)?),
    }
}

fn to_value<T: Serialize>(value: T) -> Result<Value, ProtocolResponse> {
    serde_json::to_value(value).map_err(|_| {
        ProtocolResponse::failure(
            String::new(),
            "serialization_error",
            "The protocol response could not be serialized.",
            None,
        )
    })
}

fn fixture_words(payloads: &[StartSessionEntryPayload]) -> Vec<WordForQuestion> {
    payloads
        .iter()
        .map(|payload| WordForQuestion {
            source_id: payload.source_id.clone(),
            word: payload.word.clone(),
            part_of_speech: payload.part_of_speech.clone(),
            frequency: payload.frequency,
            phonetic_us: payload.phonetic_us.clone(),
            phonetic_uk: payload.phonetic_uk.clone(),
            meanings: if payload.meaning_details.is_empty() {
                payload
                    .meanings
                    .iter()
                    .map(|meaning| MeaningZh {
                        pos: String::new(),
                        meaning_cn: meaning.clone(),
                        meaning_en: None,
                    })
                    .collect()
            } else {
                payload
                    .meaning_details
                    .iter()
                    .map(|meaning| MeaningZh {
                        pos: meaning.pos.clone(),
                        meaning_cn: meaning.meaning_cn.clone(),
                        meaning_en: meaning.meaning_en.clone(),
                    })
                    .collect()
            },
            examples: payload
                .example_sentence
                .iter()
                .map(|sentence| EntryExample {
                    sentence_en: sentence.clone(),
                    sentence_cn: payload.example_translation.clone().unwrap_or_default(),
                })
                .collect(),
            cn_choice_distractors: payload.cn_choice_distractors.clone(),
            en_choice_distractors: payload.en_choice_distractors.clone(),
        })
        .collect()
}

fn fixture_questions(
    context: &DomainContext,
    payload: &StudyFixturePayload,
    mode: SessionMode,
    suffix: &str,
) -> Vec<StudyQuestion> {
    QuestionBuilder::build_session_questions(
        &mode,
        &fixture_words(&payload.entries),
        &fixture_words(&payload.distractors),
        &format!("{}{}", context.session_id, suffix),
        &payload.question_type_weights,
    )
}

fn hidden_questions(questions: &[StudyQuestion]) -> Result<Vec<Value>, ProtocolResponse> {
    questions
        .iter()
        .map(|question| {
            let mut value = to_value(question)?;
            value["exampleTranslation"] = Value::Null;
            Ok(value)
        })
        .collect()
}

fn capture_new_word(
    context: &DomainContext,
    payload: StudyFixturePayload,
) -> Result<Value, ProtocolResponse> {
    let questions = fixture_questions(context, &payload, SessionMode::NewWord, "");
    Ok(json!({
        "mode": "newWord",
        "orderingSeed": context.ordering_seed,
        "totalWords": payload.entries.len(),
        "totalQuestions": questions.len(),
        "questions": hidden_questions(&questions)?
    }))
}

fn capture_non_new_word_modes(
    context: &DomainContext,
    payload: StudyFixturePayload,
) -> Result<Value, ProtocolResponse> {
    let modes = [
        ("review", SessionMode::Review),
        ("mixedTest", SessionMode::MixedTest),
        (
            "wrongWordReinforcement",
            SessionMode::WrongWordReinforcement,
        ),
        ("rootAffix", SessionMode::RootAffix),
    ];
    let mut output = Map::new();
    for (name, mode) in modes {
        let questions = fixture_questions(context, &payload, mode, &format!("-{name}"));
        output.insert(
            name.to_string(),
            json!({
                "totalQuestions": questions.len(),
                "questions": hidden_questions(&questions)?
            }),
        );
    }
    Ok(Value::Object(output))
}

fn capture_post_submit_feedback(
    context: &DomainContext,
    payload: StudyFixturePayload,
) -> Result<Value, ProtocolResponse> {
    let questions = fixture_questions(context, &payload, SessionMode::NewWord, "-feedback");
    let Some(question) = questions.first() else {
        return Err(invalid_data("At least one study entry is required."));
    };
    let Some(entry) = payload
        .entries
        .iter()
        .find(|entry| entry.source_id == question.entry_source_id)
    else {
        return Err(invalid_data("The feedback source entry is unavailable."));
    };
    let response = question
        .correct_choice_label
        .clone()
        .or_else(|| question.accepted_meanings.first().cloned())
        .ok_or_else(|| invalid_data("The generated question has no accepted answer."))?;
    let result = AnswerEvaluator::evaluate(
        question,
        &StudyAnswer {
            question_id: question.question_id.clone(),
            response,
            response_time_ms: 725,
        },
        &context.now_utc,
    );
    Ok(json!({
        "questionId": question.question_id,
        "result": result,
        "feedback": {
            "exampleSentence": question.example_sentence,
            "exampleTranslation": entry.example_translation,
            "acceptedMeanings": question.accepted_meanings
        }
    }))
}

fn capture_progress_summary(
    context: &DomainContext,
    payload: ProgressSummaryPayload,
) -> Result<Value, ProtocolResponse> {
    let summary = summarize_results(
        &context.session_id,
        &payload.results,
        payload.expected_total_questions,
        &context.now_utc,
    );
    Ok(json!({
        "progress": payload.progress,
        "carryOver": payload.carry_over,
        "summary": summary
    }))
}

fn capture_resume_state(
    context: &DomainContext,
    payload: StudyFixturePayload,
) -> Result<Value, ProtocolResponse> {
    let questions = fixture_questions(context, &payload, SessionMode::MixedTest, "-resume");
    if questions.len() < 2 {
        return Err(invalid_data("At least two resume questions are required."));
    }
    let first = questions[0].clone();
    let response = first
        .correct_choice_label
        .clone()
        .or_else(|| first.accepted_meanings.first().cloned())
        .ok_or_else(|| invalid_data("The first resume question has no accepted answer."))?;
    let first_result = AnswerEvaluator::evaluate(
        &first,
        &StudyAnswer {
            question_id: first.question_id.clone(),
            response,
            response_time_ms: 900,
        },
        &context.now_utc,
    );
    to_value(StartSessionResponse {
        session: StudySession {
            session_id: context.session_id.clone(),
            mode: SessionMode::MixedTest,
            total_words: payload.entries.len() as u32,
            wordbook_id: Some(42),
            started_at: context.now_utc.clone(),
        },
        current_question: questions[1].clone(),
        progress: SessionProgress {
            current: 2,
            total: questions.len() as u32,
        },
        answered_questions: vec![AnsweredStudyQuestion {
            question: first,
            result: first_result,
        }],
    })
}

fn capture_wrong_words(
    context: &DomainContext,
    payload: WrongWordsPayload,
) -> Result<Value, ProtocolResponse> {
    Ok(json!({
        "entries": project_wrong_words(context, &payload.entries, &payload.filter)
    }))
}

fn capture_report(
    context: &DomainContext,
    payload: ReportPayload,
) -> Result<Value, ProtocolResponse> {
    Ok(json!({
        "localDay": context.local_day,
        "report": project_report(context, &payload.history, payload.learned_count)
    }))
}

fn build_session(
    context: &DomainContext,
    request: StartSessionRequest,
) -> Result<Value, ProtocolResponse> {
    let words = if request.entry_payloads.is_empty() {
        request
            .entry_source_ids
            .iter()
            .map(|id| WordForQuestion {
                source_id: id.clone(),
                word: id.clone(),
                part_of_speech: None,
                frequency: 0.0,
                phonetic_us: None,
                phonetic_uk: None,
                meanings: Vec::new(),
                examples: Vec::new(),
                cn_choice_distractors: Vec::new(),
                en_choice_distractors: Vec::new(),
            })
            .collect()
    } else {
        fixture_words(&request.entry_payloads)
    };
    let questions = QuestionBuilder::build_session_questions(
        &request.mode,
        &words,
        &fixture_words(&request.distractor_payloads),
        &context.session_id,
        &request.question_type_weights,
    );
    let Some(current_question) = questions.first().cloned() else {
        return Err(invalid_data("Not enough words to build a study session."));
    };
    let response = StartSessionResponse {
        session: StudySession {
            session_id: context.session_id.clone(),
            mode: request.mode,
            total_words: words.len() as u32,
            wordbook_id: request.wordbook_id,
            started_at: context.now_utc.clone(),
        },
        current_question,
        progress: SessionProgress {
            current: 1,
            total: questions.len() as u32,
        },
        answered_questions: Vec::new(),
    };
    let mut value = to_value(response)?;
    value["questions"] = to_value(questions)?;
    Ok(value)
}

fn evaluate_answer(
    context: &DomainContext,
    payload: EvaluateAnswerPayload,
) -> Result<Value, ProtocolResponse> {
    to_value(AnswerEvaluator::evaluate(
        &payload.question,
        &payload.answer,
        payload.answered_at.as_deref().unwrap_or(&context.now_utc),
    ))
}

fn complete_session(
    context: &DomainContext,
    payload: CompleteSessionPayload,
) -> Result<Value, ProtocolResponse> {
    let summary = summarize_results(
        &payload.session.session_id,
        &payload.results,
        payload.results.len() as u32,
        payload.completed_at.as_deref().unwrap_or(&context.now_utc),
    );
    let next_action = if summary.wrong_word_count > 0 {
        format!(
            "Wrong word reinforcement ({} words)",
            summary.wrong_word_count
        )
    } else if summary.accuracy_percent < 80.0 {
        "Review fuzzy answers".to_string()
    } else if summary.accuracy_percent < 100.0 {
        "Continue with next batch".to_string()
    } else {
        match payload.session.mode {
            SessionMode::NewWord => "Continue with next batch",
            SessionMode::Review => "Review complete",
            SessionMode::MixedTest => "Check wrong words",
            SessionMode::WrongWordReinforcement => "Continue with new words",
            SessionMode::RootAffix => "Continue with review",
        }
        .to_string()
    };
    Ok(json!({ "summary": summary, "nextAction": next_action }))
}

fn invalid_data(message: &'static str) -> ProtocolResponse {
    ProtocolResponse::failure(String::new(), "invalid_data", message, None)
}
