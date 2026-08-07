use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StandardizedEntry {
    pub source_id: String,
    pub word: String,
    pub lemma: String,
    pub phonetic_us: Option<String>,
    pub phonetic_uk: Option<String>,
    pub part_of_speech: Option<String>,
    pub meanings_zh: Vec<MeaningZh>,
    pub examples: Vec<EntryExample>,
    pub tags: Vec<String>,
    pub difficulty: Option<String>,
    pub frequency: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeaningZh {
    pub pos: String,
    pub meaning_cn: String,
    pub meaning_en: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryExample {
    pub sentence_en: String,
    pub sentence_cn: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum QuestionType {
    EnToCnChoice,
    ExampleToCnChoice,
    ExampleToCnChoiceNoTranslation,
    CnToEnChoice,
    EnToCnInput,
    WordSkeletonInput,
    GlossToRootInput,
    RootToGlossInput,
}

impl QuestionType {
    pub fn is_input_type(&self) -> bool {
        matches!(
            self,
            Self::EnToCnInput
                | Self::WordSkeletonInput
                | Self::GlossToRootInput
                | Self::RootToGlossInput
        )
    }

    pub fn is_choice_type(&self) -> bool {
        !self.is_input_type()
    }

    pub fn all_four() -> Vec<Self> {
        vec![
            Self::ExampleToCnChoice,
            Self::EnToCnChoice,
            Self::CnToEnChoice,
            Self::EnToCnInput,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChoiceOption {
    pub text: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyQuestion {
    pub question_id: String,
    pub question_type: QuestionType,
    pub entry_source_id: String,
    pub word: String,
    pub part_of_speech: Option<String>,
    pub phonetic_us: Option<String>,
    pub phonetic_uk: Option<String>,
    pub prompt: String,
    pub accepted_meanings: Vec<String>,
    pub example_sentence: Option<String>,
    pub example_translation: Option<String>,
    pub choices: Option<Vec<ChoiceOption>>,
    pub correct_choice_label: Option<String>,
    pub question_index: u32,
    pub total_questions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyAnswer {
    pub question_id: String,
    pub response: String,
    pub response_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AnswerOutcome {
    Correct,
    FuzzyCorrect,
    Incorrect,
    Skipped,
}

impl AnswerOutcome {
    pub fn is_positive(&self) -> bool {
        matches!(self, Self::Correct | Self::FuzzyCorrect)
    }

    pub fn is_clear_failure(&self) -> bool {
        matches!(self, Self::Incorrect | Self::Skipped)
    }

    pub fn wrong_word_weight(&self) -> f64 {
        match self {
            Self::Incorrect => 2.0,
            Self::Skipped => 2.5,
            Self::FuzzyCorrect => 0.5,
            Self::Correct => 0.0,
        }
    }

    pub fn enters_wrong_pool(&self) -> bool {
        matches!(self, Self::Incorrect | Self::Skipped)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyResult {
    pub question_id: String,
    pub entry_source_id: String,
    pub question_type: QuestionType,
    pub user_response: String,
    pub normalized_response: Option<String>,
    pub correct_answer: String,
    pub outcome: AnswerOutcome,
    pub response_time_ms: u64,
    pub answered_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub session_id: String,
    pub total_questions: u32,
    pub correct_count: u32,
    pub fuzzy_correct_count: u32,
    pub incorrect_count: u32,
    pub skipped_count: u32,
    pub total_words: u32,
    pub wrong_word_count: u32,
    pub accuracy_percent: f64,
    pub total_time_ms: u64,
    pub completed_at: String,
}

impl SessionSummary {
    pub fn from_results(session_id: &str, results: &[StudyResult], completed_at: &str) -> Self {
        Self::from_results_with_total(session_id, results, results.len() as u32, completed_at)
    }

    pub fn from_results_with_total(
        session_id: &str,
        results: &[StudyResult],
        expected_total_questions: u32,
        completed_at: &str,
    ) -> Self {
        let mut seen_questions = std::collections::HashSet::new();
        let summarized_results = results
            .iter()
            .filter(|result| seen_questions.insert(result.question_id.as_str()))
            .take(expected_total_questions as usize)
            .collect::<Vec<_>>();
        let correct_count = summarized_results
            .iter()
            .filter(|result| result.outcome == AnswerOutcome::Correct)
            .count() as u32;
        let fuzzy_correct_count = summarized_results
            .iter()
            .filter(|result| result.outcome == AnswerOutcome::FuzzyCorrect)
            .count() as u32;
        let incorrect_count = summarized_results
            .iter()
            .filter(|result| result.outcome == AnswerOutcome::Incorrect)
            .count() as u32;
        let skipped_count = summarized_results
            .iter()
            .filter(|result| result.outcome == AnswerOutcome::Skipped)
            .count() as u32;
        let total_words = summarized_results
            .iter()
            .map(|result| result.entry_source_id.as_str())
            .collect::<std::collections::HashSet<_>>()
            .len() as u32;
        let wrong_word_count = summarized_results
            .iter()
            .filter(|result| result.outcome.enters_wrong_pool())
            .map(|result| result.entry_source_id.as_str())
            .collect::<std::collections::HashSet<_>>()
            .len() as u32;
        let positive_count = (correct_count + fuzzy_correct_count) as f64;
        let accuracy_percent = if expected_total_questions > 0 {
            positive_count / expected_total_questions as f64 * 100.0
        } else {
            0.0
        };
        let total_time_ms = summarized_results
            .iter()
            .map(|result| result.response_time_ms)
            .sum();

        Self {
            session_id: session_id.to_string(),
            total_questions: expected_total_questions,
            correct_count,
            fuzzy_correct_count,
            incorrect_count,
            skipped_count,
            total_words,
            wrong_word_count,
            accuracy_percent,
            total_time_ms,
            completed_at: completed_at.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SessionMode {
    NewWord,
    Review,
    MixedTest,
    WrongWordReinforcement,
    HighFrequency,
    RootAffix,
}

impl Default for SessionMode {
    fn default() -> Self {
        Self::NewWord
    }
}

impl SessionMode {
    pub fn uses_four_question_loop(&self) -> bool {
        matches!(self, Self::NewWord | Self::Review)
    }

    pub fn produces_report_data(&self) -> bool {
        matches!(
            self,
            Self::Review
                | Self::MixedTest
                | Self::WrongWordReinforcement
                | Self::HighFrequency
                | Self::RootAffix
        )
    }

    pub fn display_label(&self) -> &str {
        match self {
            Self::NewWord => "鏂拌瘝瀛︿範",
            Self::Review => "鏃ц瘝澶嶄範",
            Self::MixedTest => "娣峰悎娴嬭瘯",
            Self::WrongWordReinforcement => "閿欒瘝寮哄寲",
            Self::HighFrequency => "\u{9ad8}\u{9891}\u{8bcd}",
            Self::RootAffix => "璇嶆牴璇嶇紑",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudySession {
    pub session_id: String,
    pub mode: SessionMode,
    pub total_words: u32,
    pub wordbook_id: Option<i64>,
    pub started_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartSessionMeaningPayload {
    pub pos: String,
    pub meaning_cn: String,
    #[serde(default)]
    pub meaning_en: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartSessionEntryPayload {
    pub source_id: String,
    pub word: String,
    pub part_of_speech: Option<String>,
    #[serde(default)]
    pub frequency: f64,
    pub phonetic_us: Option<String>,
    pub phonetic_uk: Option<String>,
    #[serde(default)]
    pub meaning_details: Vec<StartSessionMeaningPayload>,
    pub meanings: Vec<String>,
    pub example_sentence: Option<String>,
    pub example_translation: Option<String>,
    #[serde(default)]
    pub cn_choice_distractors: Vec<String>,
    #[serde(default)]
    pub en_choice_distractors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QuestionTypeWeight {
    pub question_type: QuestionType,
    pub weight: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartSessionRequest {
    pub mode: SessionMode,
    pub wordbook_id: Option<i64>,
    pub entry_source_ids: Vec<String>,
    #[serde(default)]
    pub entry_payloads: Vec<StartSessionEntryPayload>,
    #[serde(default)]
    pub distractor_payloads: Vec<StartSessionEntryPayload>,
    #[serde(default)]
    pub question_type_weights: Vec<QuestionTypeWeight>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnsweredStudyQuestion {
    pub question: StudyQuestion,
    pub result: StudyResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionProgress {
    pub current: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartSessionResponse {
    pub session: StudySession,
    pub current_question: StudyQuestion,
    pub progress: SessionProgress,
    #[serde(default)]
    pub answered_questions: Vec<AnsweredStudyQuestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitAnswerRequest {
    pub question_id: String,
    pub response: String,
    pub response_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitAnswerResponse {
    pub result: StudyResult,
    pub is_complete: bool,
    pub current_question: Option<StudyQuestion>,
    pub summary: Option<SessionSummary>,
    pub next_action: Option<String>,
    pub progress: SessionProgress,
    #[serde(default)]
    pub answered_questions: Vec<AnsweredStudyQuestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptDisputedMeaningRequest {
    pub question_id: String,
    #[serde(default)]
    pub submitted_answer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptDisputedMeaningResponse {
    pub result: StudyResult,
    pub progress: SessionProgress,
    #[serde(default)]
    pub answered_questions: Vec<AnsweredStudyQuestion>,
    pub entry_source_id: String,
    pub word: String,
    pub accepted_meaning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkStudyEntryMasteredRequest {
    pub entry_source_id: String,
    #[serde(default = "default_mastered_reason")]
    pub reason: String,
}

fn default_mastered_reason() -> String {
    "mastered".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkStudyEntryMasteredResponse {
    pub entry_source_id: String,
    pub entry_id: Option<i64>,
    pub pruned_question_count: u32,
    pub is_complete: bool,
    pub current_question: Option<StudyQuestion>,
    pub summary: Option<SessionSummary>,
    pub next_action: Option<String>,
    pub progress: SessionProgress,
    #[serde(default)]
    pub answered_questions: Vec<AnsweredStudyQuestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteSessionResponse {
    pub summary: SessionSummary,
    pub next_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Phase6QuestionRef {
    pub question_id: String,
    pub entry_source_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Phase6AcceptedAnswer {
    pub question_id: String,
    pub entry_source_id: String,
    pub outcome: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Phase6SourceRef {
    pub book_id: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Phase6SessionSnapshot {
    pub mode: SessionMode,
    pub target_questions: u32,
    #[serde(default)]
    pub source: Option<Phase6SourceRef>,
    pub accepted_answers: Vec<Phase6AcceptedAnswer>,
    pub unanswered: Vec<Phase6QuestionRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Phase6MasteryRevision {
    pub entry_source_id: String,
    pub reason: String,
    pub revision: u64,
}
