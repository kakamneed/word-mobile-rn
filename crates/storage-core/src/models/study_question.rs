use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum QuestionType {
    EnToCnChoice,
    ExampleToCnChoice,
    CnToEnChoice,
    EnToCnInput,
    GlossToRootInput,
    RootToGlossInput,
}

impl QuestionType {
    pub fn is_input_type(&self) -> bool {
        matches!(
            self,
            QuestionType::EnToCnInput
                | QuestionType::GlossToRootInput
                | QuestionType::RootToGlossInput
        )
    }

    pub fn is_choice_type(&self) -> bool {
        !self.is_input_type()
    }

    pub fn all_four() -> Vec<QuestionType> {
        vec![
            QuestionType::ExampleToCnChoice,
            QuestionType::EnToCnChoice,
            QuestionType::CnToEnChoice,
            QuestionType::EnToCnInput,
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
