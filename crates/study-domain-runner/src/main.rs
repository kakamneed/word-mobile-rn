use std::io::{self, Read};

use serde::Deserialize;
use word_storage_core::models::{
    EntryExample, MeaningZh, SessionProgress, StartSessionEntryPayload, StartSessionRequest,
    StartSessionResponse, StudyAnswer, StudyQuestion, StudyResult, StudySession,
};
use word_study_core::{AnswerEvaluator, QuestionBuilder, SessionSummaryService, WordForQuestion};

#[derive(Debug, thiserror::Error)]
enum RunnerError {
    #[error("missing command")]
    MissingCommand,
    #[error("unknown command: {0}")]
    UnknownCommand(String),
    #[error("failed to read stdin: {0}")]
    ReadStdin(#[from] io::Error),
    #[error("invalid json input: {0}")]
    Json(#[from] serde_json::Error),
    #[error("not enough words to build a study session")]
    NotEnoughWords,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EvaluateAnswerInput {
    question: StudyQuestion,
    answer: StudyAnswer,
    answered_at: Option<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct BuildSessionRunnerResponse {
    #[serde(flatten)]
    response: StartSessionResponse,
    questions: Vec<StudyQuestion>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompleteSessionInput {
    session: StudySession,
    results: Vec<StudyResult>,
    completed_at: Option<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CompleteSessionRunnerResponse {
    summary: word_storage_core::models::SessionSummary,
    next_action: String,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), RunnerError> {
    let command = std::env::args().nth(1).ok_or(RunnerError::MissingCommand)?;
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    match command.as_str() {
        "build-session" => {
            let request: StartSessionRequest = serde_json::from_str(&input)?;
            let response = build_session(request)?;
            write_json(&response)?;
        }
        "evaluate-answer" => {
            let request: EvaluateAnswerInput = serde_json::from_str(&input)?;
            let response = evaluate_answer(request);
            write_json(&response)?;
        }
        "complete-session" => {
            let request: CompleteSessionInput = serde_json::from_str(&input)?;
            let response = complete_session(request);
            write_json(&response)?;
        }
        _ => return Err(RunnerError::UnknownCommand(command)),
    }

    Ok(())
}

fn write_json<T: serde::Serialize>(value: &T) -> Result<(), RunnerError> {
    println!("{}", serde_json::to_string(value)?);
    Ok(())
}

fn build_session(request: StartSessionRequest) -> Result<BuildSessionRunnerResponse, RunnerError> {
    let words = if !request.entry_payloads.is_empty() {
        payloads_to_words(&request.entry_payloads)
    } else {
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
    };
    let distractors = payloads_to_words(&request.distractor_payloads);
    let now = chrono::Utc::now();
    let session_id = format!("sess_{}", now.format("%Y%m%d%H%M%S%f"));
    let questions = QuestionBuilder::build_session_questions(
        &request.mode,
        &words,
        &distractors,
        &session_id,
        &request.question_type_weights,
    );
    let current_question = questions
        .first()
        .cloned()
        .ok_or(RunnerError::NotEnoughWords)?;
    let total_words = words.len() as u32;
    let total_questions = questions.len() as u32;

    let response = StartSessionResponse {
        session: StudySession {
            session_id,
            mode: request.mode,
            total_words,
            wordbook_id: request.wordbook_id,
            started_at: now.to_rfc3339(),
        },
        current_question,
        progress: SessionProgress {
            current: 1,
            total: total_questions,
        },
        answered_questions: Vec::new(),
    };

    Ok(BuildSessionRunnerResponse {
        response,
        questions,
    })
}

fn evaluate_answer(request: EvaluateAnswerInput) -> StudyResult {
    let answered_at = request
        .answered_at
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
    AnswerEvaluator::evaluate(&request.question, &request.answer, &answered_at)
}

fn complete_session(request: CompleteSessionInput) -> CompleteSessionRunnerResponse {
    let completed_at = request
        .completed_at
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
    let summary =
        SessionSummaryService::build_summary(&request.session, &request.results, &completed_at);
    let next_action = SessionSummaryService::next_action(&summary, &request.session.mode);
    CompleteSessionRunnerResponse {
        summary,
        next_action,
    }
}

fn payloads_to_words(payloads: &[StartSessionEntryPayload]) -> Vec<WordForQuestion> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use word_storage_core::models::{
        AnswerOutcome, QuestionType, QuestionTypeWeight, SessionMode, StartSessionMeaningPayload,
    };

    fn entry(source_id: &str, word: &str, meaning: &str) -> StartSessionEntryPayload {
        StartSessionEntryPayload {
            source_id: source_id.to_string(),
            word: word.to_string(),
            part_of_speech: Some("n".to_string()),
            frequency: 1.0,
            phonetic_us: None,
            phonetic_uk: None,
            meaning_details: vec![StartSessionMeaningPayload {
                pos: "n".to_string(),
                meaning_cn: meaning.to_string(),
                meaning_en: None,
            }],
            meanings: vec![meaning.to_string()],
            example_sentence: Some(format!("{word} example")),
            example_translation: Some(format!("{meaning} translation")),
        }
    }

    #[test]
    fn build_session_uses_new_word_all_four_loop_from_rust_domain() {
        let response = build_session(StartSessionRequest {
            mode: SessionMode::NewWord,
            wordbook_id: Some(7),
            entry_source_ids: Vec::new(),
            entry_payloads: vec![
                entry("alpha", "alpha", "alpha meaning"),
                entry("beta", "beta", "beta meaning"),
            ],
            distractor_payloads: vec![
                entry("gamma", "gamma", "gamma meaning"),
                entry("delta", "delta", "delta meaning"),
                entry("epsilon", "epsilon", "epsilon meaning"),
            ],
            question_type_weights: vec![QuestionTypeWeight {
                question_type: QuestionType::WordSkeletonInput,
                weight: 100,
            }],
        })
        .expect("session should build");

        assert_eq!(response.response.session.wordbook_id, Some(7));
        assert_eq!(response.response.session.total_words, 2);
        assert_eq!(response.response.progress.current, 1);
        assert_eq!(response.response.progress.total, 8);
        assert_eq!(response.questions.len(), 8);
        assert_eq!(
            response.response.current_question.question_type,
            QuestionType::ExampleToCnChoice
        );
        assert!(response.response.answered_questions.is_empty());
    }

    #[test]
    fn build_session_uses_weighted_mixed_test_questions_from_rust_domain() {
        let response = build_session(StartSessionRequest {
            mode: SessionMode::MixedTest,
            wordbook_id: None,
            entry_source_ids: Vec::new(),
            entry_payloads: vec![
                entry("alpha", "alpha", "alpha meaning"),
                entry("beta", "beta", "beta meaning"),
            ],
            distractor_payloads: Vec::new(),
            question_type_weights: vec![QuestionTypeWeight {
                question_type: QuestionType::WordSkeletonInput,
                weight: 100,
            }],
        })
        .expect("session should build");

        assert_eq!(response.response.progress.total, 2);
        assert_eq!(response.questions.len(), 2);
        assert_eq!(
            response.response.current_question.question_type,
            QuestionType::WordSkeletonInput
        );
    }

    #[test]
    fn evaluate_answer_uses_rust_choice_label_authority() {
        let start = build_session(StartSessionRequest {
            mode: SessionMode::MixedTest,
            wordbook_id: None,
            entry_source_ids: Vec::new(),
            entry_payloads: vec![entry("alpha", "alpha", "alpha meaning")],
            distractor_payloads: vec![
                entry("beta", "beta", "beta meaning"),
                entry("gamma", "gamma", "gamma meaning"),
                entry("delta", "delta", "delta meaning"),
            ],
            question_type_weights: vec![QuestionTypeWeight {
                question_type: QuestionType::EnToCnChoice,
                weight: 100,
            }],
        })
        .expect("session should build");

        let correct_label = start
            .response
            .current_question
            .correct_choice_label
            .clone()
            .expect("choice question should include correct label");
        let result = evaluate_answer(EvaluateAnswerInput {
            question: start.response.current_question,
            answer: StudyAnswer {
                question_id: "ignored_by_evaluator".to_string(),
                response: correct_label,
                response_time_ms: 321,
            },
            answered_at: Some("2026-05-20T00:00:00Z".to_string()),
        });

        assert_eq!(result.outcome, AnswerOutcome::Correct);
        assert_eq!(result.answered_at, "2026-05-20T00:00:00Z");
    }

    #[test]
    fn complete_session_uses_rust_summary_service() {
        let start = build_session(StartSessionRequest {
            mode: SessionMode::MixedTest,
            wordbook_id: None,
            entry_source_ids: Vec::new(),
            entry_payloads: vec![entry("alpha", "alpha", "alpha meaning")],
            distractor_payloads: Vec::new(),
            question_type_weights: vec![QuestionTypeWeight {
                question_type: QuestionType::WordSkeletonInput,
                weight: 100,
            }],
        })
        .expect("session should build");

        let result = evaluate_answer(EvaluateAnswerInput {
            question: start.response.current_question.clone(),
            answer: StudyAnswer {
                question_id: start.response.current_question.question_id.clone(),
                response: start.response.current_question.accepted_meanings[0].clone(),
                response_time_ms: 123,
            },
            answered_at: Some("2026-05-20T00:00:00Z".to_string()),
        });
        let complete = complete_session(CompleteSessionInput {
            session: start.response.session,
            results: vec![result],
            completed_at: Some("2026-05-20T00:01:00Z".to_string()),
        });

        assert_eq!(complete.summary.total_questions, 1);
        assert_eq!(complete.summary.correct_count, 1);
        assert_eq!(complete.summary.total_time_ms, 123);
        assert_eq!(complete.next_action, "Check wrong words");
    }
}
