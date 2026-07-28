use word_domain_models::{EntryExample, MeaningZh, QuestionTypeWeight, SessionMode, StudyQuestion};

/// A typed, persistence-free entry prepared for question generation.
#[derive(Debug, Clone)]
pub struct WordForQuestion {
    pub source_id: String,
    pub word: String,
    pub part_of_speech: Option<String>,
    pub frequency: f64,
    pub phonetic_us: Option<String>,
    pub phonetic_uk: Option<String>,
    pub meanings: Vec<MeaningZh>,
    pub examples: Vec<EntryExample>,
    pub cn_choice_distractors: Vec<String>,
    pub en_choice_distractors: Vec<String>,
}

pub struct QuestionBuilder;

impl QuestionBuilder {
    pub fn build_session_questions(
        _mode: &SessionMode,
        _words: &[WordForQuestion],
        _distractors: &[WordForQuestion],
        _session_id: &str,
        _question_type_weights: &[QuestionTypeWeight],
    ) -> Vec<StudyQuestion> {
        panic!("study fixture not implemented")
    }
}

