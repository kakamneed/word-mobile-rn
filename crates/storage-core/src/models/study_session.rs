use serde::{Deserialize, Serialize};

/// Distinct study session modes exposed to the mobile frontend.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SessionMode {
    NewWord,
    Review,
    MixedTest,
    WrongWordReinforcement,
    RootAffix,
}

impl Default for SessionMode {
    fn default() -> Self {
        Self::NewWord
    }
}

impl SessionMode {
    pub fn uses_four_question_loop(&self) -> bool {
        matches!(self, SessionMode::NewWord | SessionMode::Review)
    }

    pub fn produces_report_data(&self) -> bool {
        matches!(
            self,
            SessionMode::Review
                | SessionMode::MixedTest
                | SessionMode::WrongWordReinforcement
                | SessionMode::RootAffix
        )
    }

    pub fn display_label(&self) -> &str {
        match self {
            SessionMode::NewWord => "新词学习",
            SessionMode::Review => "旧词复习",
            SessionMode::MixedTest => "混合测试",
            SessionMode::WrongWordReinforcement => "错词强化",
            SessionMode::RootAffix => "词根词缀",
        }
    }
}

/// Top-level session descriptor passed to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudySession {
    pub session_id: String,
    pub mode: SessionMode,
    pub total_words: u32,
    pub wordbook_id: Option<i64>,
    pub started_at: String,
}
