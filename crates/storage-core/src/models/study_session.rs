use serde::{Deserialize, Serialize};

/// The four distinct study session modes.
///
/// Each mode has its own behavioral contract regarding word selection,
/// question types, and completion conditions. Modes must not be mixed
/// or blurred into a single generic study flow (D-01).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SessionMode {
    /// First contact with a batch of new words.
    /// Uses the locked four-question loop per word.
    /// Testing is limited to the batch introduced in this session (D-02, D-06).
    NewWord,

    /// Reinforcement of already-learned vocabulary.
    /// Uses the same four-question loop as new-word learning (D-03, D-07).
    Review,

    /// Formal test-style mode.
    /// Follows old-project behavior for question selection and scoring (D-04, D-09).
    MixedTest,

    /// Focused practice on recently or frequently missed words.
    /// Draws from the wrong-word pool with graded reinforcement (D-05, D-09).
    WrongWordReinforcement,
}

impl SessionMode {
    /// Whether this mode uses the locked four-question loop per word.
    /// New-word and review share the same loop; mixed test and wrong-word
    /// reinforcement follow old-project behavior (D-07).
    pub fn uses_four_question_loop(&self) -> bool {
        matches!(self, SessionMode::NewWord | SessionMode::Review)
    }

    /// Whether this mode produces structured data suitable for
    /// chart-based follow-up reporting (D-23).
    pub fn produces_report_data(&self) -> bool {
        matches!(
            self,
            SessionMode::Review | SessionMode::MixedTest | SessionMode::WrongWordReinforcement
        )
    }

    /// Human-readable label for the session mode (Chinese).
    pub fn display_label(&self) -> &str {
        match self {
            SessionMode::NewWord => "新词学习",
            SessionMode::Review => "旧词复习",
            SessionMode::MixedTest => "混合测试",
            SessionMode::WrongWordReinforcement => "错词强化",
        }
    }
}

/// Top-level session descriptor passed from the backend to the frontend.
///
/// Contains all metadata the frontend needs to initialize and render
/// a study session UI for any of the four modes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudySession {
    /// Unique session identifier.
    pub session_id: String,

    /// Which session mode this run uses.
    pub mode: SessionMode,

    /// Total number of words in this session.
    pub total_words: u32,

    /// The wordbook this session draws vocabulary from (if applicable).
    pub wordbook_id: Option<i64>,

    /// ISO 8601 timestamp of when the session was created.
    pub started_at: String,
}
