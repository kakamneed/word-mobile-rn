use std::collections::{BTreeMap, BTreeSet};

use rusqlite::{params, Connection};

use crate::models::{
    ExerciseAnnotation, ExerciseAnnotationDraft, ExerciseArticle, ExerciseArticleDraft,
    ExerciseAttempt, ExerciseAttemptDraft, ExerciseVocabOccurrence, ExerciseVocabOccurrenceDraft,
    ExerciseVocabRelation, ExerciseWordMarkState,
};
use crate::StorageError;

pub fn upsert_exercise_article(
    conn: &Connection,
    draft: &ExerciseArticleDraft,
) -> Result<i64, StorageError> {
    conn.execute(
        "INSERT INTO exercise_articles (
            article_id, source_type, title, body, language, metadata_json, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))
        ON CONFLICT(article_id) DO UPDATE SET
            source_type = excluded.source_type,
            title = excluded.title,
            body = excluded.body,
            language = excluded.language,
            metadata_json = excluded.metadata_json,
            updated_at = excluded.updated_at",
        params![
            draft.article_id,
            draft.source_type,
            draft.title,
            draft.body,
            empty_default(&draft.language, "en"),
            empty_default(&draft.metadata_json, "{}"),
        ],
    )
    .map_err(|e| StorageError::Database(format!("Failed to upsert exercise article: {e}")))?;

    conn.query_row(
        "SELECT id FROM exercise_articles WHERE article_id = ?1",
        params![draft.article_id],
        |row| row.get(0),
    )
    .map_err(|e| StorageError::Database(format!("Failed to load exercise article id: {e}")))
}

pub fn get_exercise_article_by_source_id(
    conn: &Connection,
    article_id: &str,
) -> Result<Option<ExerciseArticle>, StorageError> {
    let result = conn.query_row(
        "SELECT id, article_id, source_type, title, body, language, metadata_json, imported_at, updated_at
         FROM exercise_articles WHERE article_id = ?1",
        params![article_id],
        row_to_article,
    );
    match result {
        Ok(article) => Ok(Some(article)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(StorageError::Database(format!(
            "Failed to load exercise article: {e}"
        ))),
    }
}

pub fn upsert_exercise_vocab_occurrence(
    conn: &Connection,
    draft: &ExerciseVocabOccurrenceDraft,
) -> Result<i64, StorageError> {
    conn.execute(
        "INSERT INTO exercise_vocab_occurrences (
            article_id, entry_id, word_form, normalized_form, sentence_text,
            paragraph_index, sentence_index, start_offset, end_offset,
            lookup_status, user_mark, meaning_note, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, datetime('now'))
        ON CONFLICT(article_id, start_offset, end_offset, word_form) DO UPDATE SET
            entry_id = excluded.entry_id,
            normalized_form = excluded.normalized_form,
            sentence_text = excluded.sentence_text,
            paragraph_index = excluded.paragraph_index,
            sentence_index = excluded.sentence_index,
            lookup_status = excluded.lookup_status,
            user_mark = excluded.user_mark,
            meaning_note = excluded.meaning_note,
            updated_at = excluded.updated_at",
        params![
            draft.article_id,
            draft.entry_id,
            draft.word_form,
            draft.normalized_form,
            draft.sentence_text,
            draft.paragraph_index,
            draft.sentence_index,
            draft.start_offset,
            draft.end_offset,
            empty_default(&draft.lookup_status, "unseen"),
            empty_default(&draft.user_mark, "none"),
            draft.meaning_note,
        ],
    )
    .map_err(|e| {
        StorageError::Database(format!("Failed to upsert exercise vocab occurrence: {e}"))
    })?;

    conn.query_row(
        "SELECT id FROM exercise_vocab_occurrences
         WHERE article_id = ?1 AND start_offset = ?2 AND end_offset = ?3 AND word_form = ?4",
        params![
            draft.article_id,
            draft.start_offset,
            draft.end_offset,
            draft.word_form
        ],
        |row| row.get(0),
    )
    .map_err(|e| StorageError::Database(format!("Failed to load occurrence id: {e}")))
}

pub fn mark_exercise_vocab_occurrence(
    conn: &Connection,
    occurrence_id: i64,
    user_mark: &str,
    meaning_note: Option<&str>,
) -> Result<(), StorageError> {
    conn.execute(
        "UPDATE exercise_vocab_occurrences
         SET user_mark = ?1,
             lookup_status = CASE WHEN lookup_status = 'unseen' THEN 'looked_up' ELSE lookup_status END,
             meaning_note = COALESCE(?2, meaning_note),
             updated_at = datetime('now')
         WHERE id = ?3",
        params![user_mark, meaning_note, occurrence_id],
    )
    .map_err(|e| StorageError::Database(format!("Failed to mark exercise occurrence: {e}")))?;
    Ok(())
}

pub fn mark_exercise_vocab_word_in_article(
    conn: &Connection,
    article_id: i64,
    normalized_form: &str,
    user_mark: &str,
    mark_level: &str,
    meaning_note: Option<&str>,
) -> Result<usize, StorageError> {
    conn.execute(
        "UPDATE exercise_vocab_occurrences
         SET user_mark = ?1,
             mark_level = ?2,
             lookup_status = CASE WHEN lookup_status = 'unseen' THEN 'looked_up' ELSE lookup_status END,
             meaning_note = COALESCE(?3, meaning_note),
             updated_at = datetime('now')
         WHERE article_id = ?4 AND LOWER(normalized_form) = LOWER(?5)",
        params![user_mark, mark_level, meaning_note, article_id, normalized_form],
    )
    .map_err(|error| {
        StorageError::Database(format!("Failed to mark exercise word in article: {error}"))
    })
}

pub fn mark_exercise_vocab_entry_in_article(
    conn: &Connection,
    article_id: i64,
    entry_id: i64,
    user_mark: &str,
    mark_level: &str,
    meaning_note: Option<&str>,
) -> Result<usize, StorageError> {
    conn.execute(
        "UPDATE exercise_vocab_occurrences
         SET user_mark = ?1,
             mark_level = ?2,
             lookup_status = CASE WHEN lookup_status = 'unseen' THEN 'looked_up' ELSE lookup_status END,
             meaning_note = COALESCE(?3, meaning_note),
             updated_at = datetime('now')
         WHERE article_id = ?4 AND entry_id = ?5",
        params![user_mark, mark_level, meaning_note, article_id, entry_id],
    )
    .map_err(|error| {
        StorageError::Database(format!("Failed to mark exercise entry in article: {error}"))
    })
}

pub fn list_exercise_vocab_occurrences(
    conn: &Connection,
    article_id: i64,
) -> Result<Vec<ExerciseVocabOccurrence>, StorageError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, article_id, entry_id, word_form, normalized_form, sentence_text,
                    paragraph_index, sentence_index, start_offset, end_offset,
                    lookup_status, user_mark, meaning_note, created_at, updated_at
             FROM exercise_vocab_occurrences
             WHERE article_id = ?1
             ORDER BY start_offset ASC, id ASC",
        )
        .map_err(|e| StorageError::Database(format!("Failed to prepare occurrences: {e}")))?;
    let rows = stmt
        .query_map(params![article_id], row_to_occurrence)
        .map_err(|e| StorageError::Database(format!("Failed to query occurrences: {e}")))?;
    collect_rows(rows, "occurrence")
}

pub fn upsert_exercise_vocab_relation(
    conn: &Connection,
    article_id: i64,
    source_occurrence_id: i64,
    target_occurrence_id: i64,
    relation_type: &str,
    relation_weight: f64,
    evidence_json: &str,
) -> Result<i64, StorageError> {
    conn.execute(
        "INSERT INTO exercise_vocab_relations (
            article_id, source_occurrence_id, target_occurrence_id,
            relation_type, relation_weight, evidence_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(article_id, source_occurrence_id, target_occurrence_id, relation_type)
        DO UPDATE SET
            relation_weight = excluded.relation_weight,
            evidence_json = excluded.evidence_json",
        params![
            article_id,
            source_occurrence_id,
            target_occurrence_id,
            relation_type,
            relation_weight,
            empty_default(evidence_json, "{}"),
        ],
    )
    .map_err(|e| StorageError::Database(format!("Failed to upsert exercise relation: {e}")))?;

    conn.query_row(
        "SELECT id FROM exercise_vocab_relations
         WHERE article_id = ?1
           AND source_occurrence_id = ?2
           AND target_occurrence_id = ?3
           AND relation_type = ?4",
        params![
            article_id,
            source_occurrence_id,
            target_occurrence_id,
            relation_type
        ],
        |row| row.get(0),
    )
    .map_err(|e| StorageError::Database(format!("Failed to load exercise relation id: {e}")))
}

pub fn list_exercise_vocab_relations(
    conn: &Connection,
    article_id: i64,
) -> Result<Vec<ExerciseVocabRelation>, StorageError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, article_id, source_occurrence_id, target_occurrence_id,
                    relation_type, relation_weight, evidence_json, created_at
             FROM exercise_vocab_relations
             WHERE article_id = ?1
             ORDER BY id ASC",
        )
        .map_err(|e| StorageError::Database(format!("Failed to prepare relations: {e}")))?;
    let rows = stmt
        .query_map(params![article_id], row_to_relation)
        .map_err(|e| StorageError::Database(format!("Failed to query relations: {e}")))?;
    collect_rows(rows, "relation")
}

pub fn refresh_same_article_relations(
    conn: &Connection,
    article_id: i64,
) -> Result<usize, StorageError> {
    conn.execute(
        "DELETE FROM exercise_vocab_relations
         WHERE article_id = ?1 AND relation_type = 'same_article'",
        params![article_id],
    )
    .map_err(|error| {
        StorageError::Database(format!("Failed to clear same-article relations: {error}"))
    })?;
    let mut stmt = conn
        .prepare(
            "SELECT id FROM exercise_vocab_occurrences
             WHERE article_id = ?1 AND user_mark IN ('ignored', 'unknown', 'wrong')
             ORDER BY id ASC",
        )
        .map_err(|error| {
            StorageError::Database(format!("Failed to prepare marked occurrences: {error}"))
        })?;
    let occurrence_ids = stmt
        .query_map(params![article_id], |row| row.get::<_, i64>(0))
        .map_err(|error| {
            StorageError::Database(format!("Failed to query marked occurrences: {error}"))
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            StorageError::Database(format!("Failed to read marked occurrence: {error}"))
        })?;
    drop(stmt);
    let mut count = 0;
    for source_index in 0..occurrence_ids.len() {
        for target_index in (source_index + 1)..occurrence_ids.len() {
            upsert_exercise_vocab_relation(
                conn,
                article_id,
                occurrence_ids[source_index],
                occurrence_ids[target_index],
                "same_article",
                1.0,
                &format!("{{\"articleId\":{article_id}}}"),
            )?;
            count += 1;
        }
    }
    Ok(count)
}

pub fn upsert_exercise_annotation(
    conn: &Connection,
    draft: &ExerciseAnnotationDraft,
) -> Result<i64, StorageError> {
    conn.execute(
        "INSERT INTO exercise_annotations (
            annotation_id, article_id, question_id, scope, start_offset,
            end_offset, selected_text, note_text, color, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, datetime('now'))
         ON CONFLICT(annotation_id) DO UPDATE SET
            question_id = excluded.question_id,
            scope = excluded.scope,
            start_offset = excluded.start_offset,
            end_offset = excluded.end_offset,
            selected_text = excluded.selected_text,
            note_text = excluded.note_text,
            color = excluded.color,
            updated_at = excluded.updated_at",
        params![
            draft.annotation_id,
            draft.article_id,
            draft.question_id,
            draft.scope,
            draft.start_offset,
            draft.end_offset,
            draft.selected_text,
            draft.note_text,
            empty_default(&draft.color, "yellow"),
        ],
    )
    .map_err(|error| {
        StorageError::Database(format!("Failed to upsert exercise annotation: {error}"))
    })?;
    conn.query_row(
        "SELECT id FROM exercise_annotations WHERE annotation_id = ?1",
        [&draft.annotation_id],
        |row| row.get(0),
    )
    .map_err(|error| StorageError::Database(format!("Failed to load annotation id: {error}")))
}

pub fn list_exercise_annotations(
    conn: &Connection,
    article_id: i64,
) -> Result<Vec<ExerciseAnnotation>, StorageError> {
    let mut statement = conn
        .prepare(
            "SELECT id, annotation_id, article_id, question_id, scope,
                    start_offset, end_offset, selected_text, note_text, color,
                    created_at, updated_at
             FROM exercise_annotations
             WHERE article_id = ?1
             ORDER BY start_offset, id",
        )
        .map_err(|error| {
            StorageError::Database(format!("Failed to prepare annotations: {error}"))
        })?;
    let rows = statement
        .query_map([article_id], |row| {
            Ok(ExerciseAnnotation {
                id: row.get(0)?,
                annotation_id: row.get(1)?,
                article_id: row.get(2)?,
                question_id: row.get(3)?,
                scope: row.get(4)?,
                start_offset: row.get(5)?,
                end_offset: row.get(6)?,
                selected_text: row.get(7)?,
                note_text: row.get(8)?,
                color: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        })
        .map_err(|error| StorageError::Database(format!("Failed to query annotations: {error}")))?;
    collect_rows(rows, "annotation")
}

pub fn load_exercise_word_mark_state(
    conn: &Connection,
    current_article_id: &str,
    normalized_words: &[String],
) -> Result<Vec<ExerciseWordMarkState>, StorageError> {
    let wanted = normalized_words.iter().cloned().collect::<BTreeSet<_>>();
    if wanted.is_empty() {
        return Ok(Vec::new());
    }
    let mut statement = conn
        .prepare(
            "SELECT o.normalized_form, a.article_id, o.meaning_note, o.user_mark,
                    o.mark_level
             FROM exercise_vocab_occurrences o
             JOIN exercise_articles a ON a.id = o.article_id
             WHERE o.user_mark IN ('ignored', 'unknown', 'wrong')
             ORDER BY o.updated_at DESC",
        )
        .map_err(|error| {
            StorageError::Database(format!("Failed to prepare exercise mark state: {error}"))
        })?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })
        .map_err(|error| {
            StorageError::Database(format!("Failed to query exercise mark state: {error}"))
        })?;
    let mut states = BTreeMap::<String, ExerciseWordMarkState>::new();
    for row in rows {
        let (normalized, article_id, meaning, user_mark, stored_level) = row.map_err(|error| {
            StorageError::Database(format!("Failed to read exercise mark state: {error}"))
        })?;
        if !wanted.contains(&normalized) {
            continue;
        }
        let state = states
            .entry(normalized.clone())
            .or_insert(ExerciseWordMarkState {
                normalized_form: normalized,
                current_article: false,
                prior_article: false,
                meaning: String::new(),
                current_mark_level: String::new(),
                prior_mark_level: String::new(),
            });
        let mark_level = public_mark_level(&user_mark, &stored_level);
        if article_id == current_article_id {
            state.current_article = true;
            if mark_weight(&mark_level) > mark_weight(&state.current_mark_level) {
                state.current_mark_level = mark_level.clone();
            }
        } else {
            state.prior_article = true;
            if mark_weight(&mark_level) > mark_weight(&state.prior_mark_level) {
                state.prior_mark_level = mark_level.clone();
            }
        }
        if state.meaning.is_empty() && !meaning.trim().is_empty() {
            state.meaning = meaning;
        }
    }
    Ok(states.into_values().collect())
}

fn public_mark_level(user_mark: &str, stored_level: &str) -> String {
    if matches!(stored_level, "fuzzy" | "familiar" | "unknown") {
        return stored_level.to_string();
    }
    match user_mark {
        "ignored" => "fuzzy",
        "unknown" | "wrong" => "unknown",
        _ => "none",
    }
    .to_string()
}

fn mark_weight(mark: &str) -> u8 {
    match mark {
        "unknown" => 3,
        "familiar" => 2,
        "fuzzy" => 1,
        _ => 0,
    }
}

pub fn upsert_exercise_attempt(
    conn: &Connection,
    draft: &ExerciseAttemptDraft,
) -> Result<i64, StorageError> {
    conn.execute(
        "INSERT INTO exercise_attempts (
            attempt_id, paper_id, section_id, question_id, selected_answer,
            is_correct, answer_history_json, status, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, datetime('now'))
        ON CONFLICT(attempt_id) DO UPDATE SET
            paper_id = excluded.paper_id,
            section_id = excluded.section_id,
            question_id = excluded.question_id,
            selected_answer = excluded.selected_answer,
            is_correct = excluded.is_correct,
            answer_history_json = excluded.answer_history_json,
            status = excluded.status,
            updated_at = excluded.updated_at",
        params![
            draft.attempt_id,
            draft.paper_id,
            draft.section_id,
            draft.question_id,
            draft.selected_answer,
            draft.is_correct,
            empty_default(&draft.answer_history_json, "[]"),
            empty_default(&draft.status, "in_progress"),
        ],
    )
    .map_err(|error| {
        StorageError::Database(format!("Failed to upsert exercise attempt: {error}"))
    })?;
    conn.query_row(
        "SELECT id FROM exercise_attempts WHERE attempt_id = ?1",
        params![draft.attempt_id],
        |row| row.get(0),
    )
    .map_err(|error| StorageError::Database(format!("Failed to load exercise attempt id: {error}")))
}

pub fn get_exercise_attempt(
    conn: &Connection,
    attempt_id: &str,
) -> Result<Option<ExerciseAttempt>, StorageError> {
    let result = conn.query_row(
        "SELECT id, attempt_id, paper_id, section_id, question_id, selected_answer,
                is_correct, answer_history_json, status, started_at, updated_at
         FROM exercise_attempts WHERE attempt_id = ?1",
        params![attempt_id],
        row_to_attempt,
    );
    match result {
        Ok(attempt) => Ok(Some(attempt)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(StorageError::Database(format!(
            "Failed to load exercise attempt: {error}"
        ))),
    }
}

fn row_to_attempt(row: &rusqlite::Row<'_>) -> rusqlite::Result<ExerciseAttempt> {
    Ok(ExerciseAttempt {
        id: row.get(0)?,
        attempt_id: row.get(1)?,
        paper_id: row.get(2)?,
        section_id: row.get(3)?,
        question_id: row.get(4)?,
        selected_answer: row.get(5)?,
        is_correct: row.get(6)?,
        answer_history_json: row.get(7)?,
        status: row.get(8)?,
        started_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn row_to_article(row: &rusqlite::Row<'_>) -> rusqlite::Result<ExerciseArticle> {
    Ok(ExerciseArticle {
        id: row.get(0)?,
        article_id: row.get(1)?,
        source_type: row.get(2)?,
        title: row.get(3)?,
        body: row.get(4)?,
        language: row.get(5)?,
        metadata_json: row.get(6)?,
        imported_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

fn row_to_occurrence(row: &rusqlite::Row<'_>) -> rusqlite::Result<ExerciseVocabOccurrence> {
    Ok(ExerciseVocabOccurrence {
        id: row.get(0)?,
        article_id: row.get(1)?,
        entry_id: row.get(2)?,
        word_form: row.get(3)?,
        normalized_form: row.get(4)?,
        sentence_text: row.get(5)?,
        paragraph_index: row.get(6)?,
        sentence_index: row.get(7)?,
        start_offset: row.get(8)?,
        end_offset: row.get(9)?,
        lookup_status: row.get(10)?,
        user_mark: row.get(11)?,
        meaning_note: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

fn row_to_relation(row: &rusqlite::Row<'_>) -> rusqlite::Result<ExerciseVocabRelation> {
    Ok(ExerciseVocabRelation {
        id: row.get(0)?,
        article_id: row.get(1)?,
        source_occurrence_id: row.get(2)?,
        target_occurrence_id: row.get(3)?,
        relation_type: row.get(4)?,
        relation_weight: row.get(5)?,
        evidence_json: row.get(6)?,
        created_at: row.get(7)?,
    })
}

fn collect_rows<T>(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>>,
    label: &str,
) -> Result<Vec<T>, StorageError> {
    let mut items = Vec::new();
    for row in rows {
        items
            .push(row.map_err(|e| StorageError::Database(format!("Failed to read {label}: {e}")))?);
    }
    Ok(items)
}

fn empty_default<'a>(value: &'a str, default: &'a str) -> &'a str {
    if value.trim().is_empty() {
        default
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::{
        get_exercise_attempt, list_exercise_annotations, list_exercise_vocab_occurrences,
        list_exercise_vocab_relations, load_exercise_word_mark_state,
        mark_exercise_vocab_entry_in_article, mark_exercise_vocab_occurrence,
        mark_exercise_vocab_word_in_article, refresh_same_article_relations,
        upsert_exercise_annotation, upsert_exercise_article, upsert_exercise_attempt,
        upsert_exercise_vocab_occurrence, upsert_exercise_vocab_relation,
    };
    use crate::models::{
        ExerciseAnnotationDraft, ExerciseArticleDraft, ExerciseAttemptDraft,
        ExerciseVocabOccurrenceDraft,
    };

    #[test]
    fn exercise_attempt_round_trip_preserves_resume_and_grading_evidence() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        crate::persistence::schema::apply_schema(&conn).expect("apply schema");

        upsert_exercise_attempt(
            &conn,
            &ExerciseAttemptDraft {
                attempt_id: "attempt-1".to_string(),
                paper_id: "cet4-2025-6-1".to_string(),
                section_id: "reading".to_string(),
                question_id: "q1".to_string(),
                selected_answer: Some("B".to_string()),
                is_correct: Some(false),
                answer_history_json: "[\"A\",\"B\"]".to_string(),
                status: "answered".to_string(),
            },
        )
        .expect("save attempt");

        let attempt = get_exercise_attempt(&conn, "attempt-1")
            .expect("load attempt")
            .expect("attempt exists");
        assert_eq!(attempt.selected_answer.as_deref(), Some("B"));
        assert_eq!(attempt.is_correct, Some(false));
        assert_eq!(attempt.answer_history_json, "[\"A\",\"B\"]");
        assert_eq!(attempt.status, "answered");
    }

    #[test]
    fn range_note_and_cross_article_word_state_round_trip() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        crate::persistence::schema::apply_schema(&conn).expect("apply schema");
        let current_article = upsert_exercise_article(
            &conn,
            &ExerciseArticleDraft {
                article_id: "paper-1:reading".to_string(),
                source_type: "builtin".to_string(),
                title: "Paper 1".to_string(),
                body: "A resilient society.".to_string(),
                language: "en".to_string(),
                metadata_json: "{}".to_string(),
            },
        )
        .expect("current article");
        let prior_article = upsert_exercise_article(
            &conn,
            &ExerciseArticleDraft {
                article_id: "paper-0:reading".to_string(),
                source_type: "builtin".to_string(),
                title: "Paper 0".to_string(),
                body: "Legacy systems.".to_string(),
                language: "en".to_string(),
                metadata_json: "{}".to_string(),
            },
        )
        .expect("prior article");
        for (article_id, word, meaning) in [
            (current_article, "resilient", "有韧性的"),
            (prior_article, "legacy", "遗留的"),
        ] {
            upsert_exercise_vocab_occurrence(
                &conn,
                &ExerciseVocabOccurrenceDraft {
                    article_id,
                    entry_id: None,
                    word_form: word.to_string(),
                    normalized_form: word.to_string(),
                    sentence_text: word.to_string(),
                    paragraph_index: 0,
                    sentence_index: 0,
                    start_offset: 0,
                    end_offset: word.len() as i64,
                    lookup_status: "matched".to_string(),
                    user_mark: "unknown".to_string(),
                    meaning_note: meaning.to_string(),
                },
            )
            .expect("marked occurrence");
        }
        upsert_exercise_annotation(
            &conn,
            &ExerciseAnnotationDraft {
                annotation_id: "note-1".to_string(),
                article_id: current_article,
                question_id: Some("q1".to_string()),
                scope: "stem".to_string(),
                start_offset: 2,
                end_offset: 11,
                selected_text: "resilient".to_string(),
                note_text: "因果转折".to_string(),
                color: "yellow".to_string(),
            },
        )
        .expect("save note");

        let notes = list_exercise_annotations(&conn, current_article).expect("list notes");
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].selected_text, "resilient");
        assert_eq!(notes[0].note_text, "因果转折");
        let marks = load_exercise_word_mark_state(
            &conn,
            "paper-1:reading",
            &["resilient".to_string(), "legacy".to_string()],
        )
        .expect("load mark state");
        assert!(marks.iter().any(|item| item.normalized_form == "resilient"
            && item.current_article
            && !item.prior_article));
        assert!(marks.iter().any(|item| item.normalized_form == "legacy"
            && !item.current_article
            && item.prior_article));
    }

    #[test]
    fn article_word_mark_updates_every_persisted_occurrence() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        crate::persistence::schema::apply_schema(&conn).expect("apply schema");
        let article_id = upsert_exercise_article(
            &conn,
            &ExerciseArticleDraft {
                article_id: "paper:section".to_string(),
                source_type: "builtin".to_string(),
                title: "Reading".to_string(),
                body: "Habit follows habit.".to_string(),
                language: "en".to_string(),
                metadata_json: "{}".to_string(),
            },
        )
        .expect("article");
        for (start, end, form) in [(0, 5, "Habit"), (14, 19, "habit")] {
            upsert_exercise_vocab_occurrence(
                &conn,
                &ExerciseVocabOccurrenceDraft {
                    article_id,
                    entry_id: None,
                    word_form: form.to_string(),
                    normalized_form: "habit".to_string(),
                    sentence_text: "Habit follows habit.".to_string(),
                    paragraph_index: 0,
                    sentence_index: 0,
                    start_offset: start,
                    end_offset: end,
                    lookup_status: "unmatched".to_string(),
                    user_mark: "none".to_string(),
                    meaning_note: "".to_string(),
                },
            )
            .expect("occurrence");
        }

        mark_exercise_vocab_word_in_article(
            &conn,
            article_id,
            "habit",
            "unknown",
            "unknown",
            Some("meaning"),
        )
        .expect("mark all");

        let occurrences = list_exercise_vocab_occurrences(&conn, article_id).unwrap();
        assert!(occurrences.iter().all(|item| item.user_mark == "unknown"));
        assert!(occurrences
            .iter()
            .all(|item| item.meaning_note == "meaning"));
    }

    #[test]
    fn article_entry_mark_updates_inflected_forms_as_one_word() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        crate::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute_batch(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'exam-family-test', 'ready');
             INSERT INTO entries (id, source_version_id, source_entry_key, word, lemma)
             VALUES (42, 1, 'patent', 'patent', 'patent');",
        )
        .expect("seed patent entry");
        let article_id = upsert_exercise_article(
            &conn,
            &ExerciseArticleDraft {
                article_id: "paper:reading".to_string(),
                source_type: "builtin".to_string(),
                title: "Reading".to_string(),
                body: "A patent protects patents.".to_string(),
                language: "en".to_string(),
                metadata_json: "{}".to_string(),
            },
        )
        .expect("article");
        for (start, end, form) in [(2, 8, "patent"), (18, 25, "patents")] {
            upsert_exercise_vocab_occurrence(
                &conn,
                &ExerciseVocabOccurrenceDraft {
                    article_id,
                    entry_id: Some(42),
                    word_form: form.to_string(),
                    normalized_form: form.to_string(),
                    sentence_text: "A patent protects patents.".to_string(),
                    paragraph_index: 0,
                    sentence_index: 0,
                    start_offset: start,
                    end_offset: end,
                    lookup_status: "matched".to_string(),
                    user_mark: "none".to_string(),
                    meaning_note: String::new(),
                },
            )
            .expect("occurrence");
        }

        mark_exercise_vocab_entry_in_article(
            &conn,
            article_id,
            42,
            "unknown",
            "familiar",
            Some("专利"),
        )
        .expect("mark family");

        let occurrences = list_exercise_vocab_occurrences(&conn, article_id).unwrap();
        assert!(occurrences.iter().all(|item| item.user_mark == "unknown"));
        let mark_level: String = conn
            .query_row(
                "SELECT mark_level FROM exercise_vocab_occurrences WHERE entry_id = 42 LIMIT 1",
                [],
                |row| row.get(0),
            )
            .expect("mark level");
        assert_eq!(mark_level, "familiar");
    }

    #[test]
    fn same_article_relation_refresh_is_mark_filtered_and_idempotent() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        crate::persistence::schema::apply_schema(&conn).expect("apply schema");
        let article_id = upsert_exercise_article(
            &conn,
            &ExerciseArticleDraft {
                article_id: "paper:section".to_string(),
                source_type: "builtin".to_string(),
                title: "Reading".to_string(),
                body: "Alpha beta gamma.".to_string(),
                language: "en".to_string(),
                metadata_json: "{}".to_string(),
            },
        )
        .expect("article");
        for (index, (word, mark)) in [("alpha", "unknown"), ("beta", "wrong"), ("gamma", "none")]
            .into_iter()
            .enumerate()
        {
            upsert_exercise_vocab_occurrence(
                &conn,
                &ExerciseVocabOccurrenceDraft {
                    article_id,
                    entry_id: None,
                    word_form: word.to_string(),
                    normalized_form: word.to_string(),
                    sentence_text: "Alpha beta gamma.".to_string(),
                    paragraph_index: 0,
                    sentence_index: 0,
                    start_offset: (index * 6) as i64,
                    end_offset: (index * 6 + word.len()) as i64,
                    lookup_status: "unmatched".to_string(),
                    user_mark: mark.to_string(),
                    meaning_note: String::new(),
                },
            )
            .expect("occurrence");
        }

        assert_eq!(
            refresh_same_article_relations(&conn, article_id).expect("refresh"),
            1
        );
        assert_eq!(
            refresh_same_article_relations(&conn, article_id).expect("refresh again"),
            1
        );
        let relations = list_exercise_vocab_relations(&conn, article_id).expect("relations");
        assert_eq!(relations.len(), 1);
        assert_eq!(relations[0].relation_type, "same_article");
    }

    #[test]
    fn exercise_article_occurrences_marks_and_relations_round_trip() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        crate::persistence::schema::apply_schema(&conn).expect("apply schema");

        let article_id = upsert_exercise_article(
            &conn,
            &ExerciseArticleDraft {
                article_id: "builtin:reading:001".to_string(),
                source_type: "builtin".to_string(),
                title: "A short debate".to_string(),
                body: "The debate shaped public opinion.".to_string(),
                language: "en".to_string(),
                metadata_json: "{\"level\":\"cet4\"}".to_string(),
            },
        )
        .expect("upsert article");

        let debate_id = upsert_exercise_vocab_occurrence(
            &conn,
            &ExerciseVocabOccurrenceDraft {
                article_id,
                entry_id: None,
                word_form: "debate".to_string(),
                normalized_form: "debate".to_string(),
                sentence_text: "The debate shaped public opinion.".to_string(),
                paragraph_index: 0,
                sentence_index: 0,
                start_offset: 4,
                end_offset: 10,
                lookup_status: "looked_up".to_string(),
                user_mark: "unknown".to_string(),
                meaning_note: "杈╄".to_string(),
            },
        )
        .expect("insert debate occurrence");
        let opinion_id = upsert_exercise_vocab_occurrence(
            &conn,
            &ExerciseVocabOccurrenceDraft {
                article_id,
                entry_id: None,
                word_form: "opinion".to_string(),
                normalized_form: "opinion".to_string(),
                sentence_text: "The debate shaped public opinion.".to_string(),
                paragraph_index: 0,
                sentence_index: 0,
                start_offset: 25,
                end_offset: 32,
                lookup_status: "unseen".to_string(),
                user_mark: "none".to_string(),
                meaning_note: String::new(),
            },
        )
        .expect("insert opinion occurrence");

        mark_exercise_vocab_occurrence(&conn, opinion_id, "wrong", Some("瑙傜偣"))
            .expect("mark opinion wrong");
        upsert_exercise_vocab_relation(
            &conn,
            article_id,
            debate_id,
            opinion_id,
            "same_sentence",
            1.0,
            "{\"sentenceIndex\":0}",
        )
        .expect("insert relation");

        let occurrences =
            list_exercise_vocab_occurrences(&conn, article_id).expect("list occurrences");
        assert_eq!(occurrences.len(), 2);
        assert_eq!(occurrences[0].word_form, "debate");
        assert_eq!(occurrences[1].user_mark, "wrong");
        assert_eq!(occurrences[1].lookup_status, "looked_up");

        let relations = list_exercise_vocab_relations(&conn, article_id).expect("list relations");
        assert_eq!(relations.len(), 1);
        assert_eq!(relations[0].relation_type, "same_sentence");
        assert_eq!(relations[0].source_occurrence_id, debate_id);
        assert_eq!(relations[0].target_occurrence_id, opinion_id);
    }
}
