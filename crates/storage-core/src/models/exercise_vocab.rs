use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExerciseArticleDraft {
    pub article_id: String,
    pub source_type: String,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub metadata_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExerciseArticle {
    pub id: i64,
    pub article_id: String,
    pub source_type: String,
    pub title: String,
    pub body: String,
    pub language: String,
    pub metadata_json: String,
    pub imported_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExerciseVocabOccurrenceDraft {
    pub article_id: i64,
    pub entry_id: Option<i64>,
    pub word_form: String,
    pub normalized_form: String,
    pub sentence_text: String,
    pub paragraph_index: i64,
    pub sentence_index: i64,
    pub start_offset: i64,
    pub end_offset: i64,
    #[serde(default)]
    pub lookup_status: String,
    #[serde(default)]
    pub user_mark: String,
    #[serde(default)]
    pub meaning_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExerciseVocabOccurrence {
    pub id: i64,
    pub article_id: i64,
    pub entry_id: Option<i64>,
    pub word_form: String,
    pub normalized_form: String,
    pub sentence_text: String,
    pub paragraph_index: i64,
    pub sentence_index: i64,
    pub start_offset: i64,
    pub end_offset: i64,
    pub lookup_status: String,
    pub user_mark: String,
    pub meaning_note: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExerciseVocabRelation {
    pub id: i64,
    pub article_id: i64,
    pub source_occurrence_id: i64,
    pub target_occurrence_id: i64,
    pub relation_type: String,
    pub relation_weight: f64,
    pub evidence_json: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExerciseAnnotationDraft {
    pub annotation_id: String,
    pub article_id: i64,
    pub question_id: Option<String>,
    pub scope: String,
    pub start_offset: i64,
    pub end_offset: i64,
    pub selected_text: String,
    #[serde(default)]
    pub note_text: String,
    #[serde(default)]
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExerciseAnnotation {
    pub id: i64,
    pub annotation_id: String,
    pub article_id: i64,
    pub question_id: Option<String>,
    pub scope: String,
    pub start_offset: i64,
    pub end_offset: i64,
    pub selected_text: String,
    pub note_text: String,
    pub color: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExerciseWordMarkState {
    pub normalized_form: String,
    pub current_article: bool,
    pub prior_article: bool,
    pub meaning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExerciseAttemptDraft {
    pub attempt_id: String,
    pub paper_id: String,
    pub section_id: String,
    pub question_id: String,
    pub selected_answer: Option<String>,
    pub is_correct: Option<bool>,
    #[serde(default)]
    pub answer_history_json: String,
    #[serde(default)]
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExerciseAttempt {
    pub id: i64,
    pub attempt_id: String,
    pub paper_id: String,
    pub section_id: String,
    pub question_id: String,
    pub selected_answer: Option<String>,
    pub is_correct: Option<bool>,
    pub answer_history_json: String,
    pub status: String,
    pub started_at: String,
    pub updated_at: String,
}
