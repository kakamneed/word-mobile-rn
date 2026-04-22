//! Answer evaluator for study answers.

use word_storage_core::models::{AnswerOutcome, StudyAnswer, StudyQuestion, StudyResult};

/// Authoritative grading service for study answers.
pub struct AnswerEvaluator;

impl AnswerEvaluator {
    /// Evaluate a user's answer against the question's accepted meanings.
    pub fn evaluate(
        question: &StudyQuestion,
        answer: &StudyAnswer,
        answered_at: &str,
    ) -> StudyResult {
        let (outcome, normalized_response) = if question.question_type.is_choice_type() {
            let choice_outcome = Self::evaluate_choice(question, &answer.response);
            (choice_outcome, None)
        } else {
            let (input_outcome, norm) =
                Self::evaluate_input(&answer.response, &question.accepted_meanings);
            (input_outcome, Some(norm))
        };

        let correct_answer = if question.question_type.is_choice_type() {
            question
                .choices
                .as_ref()
                .and_then(|choices| {
                    question
                        .correct_choice_label
                        .as_ref()
                        .and_then(|correct_label| {
                            choices
                                .iter()
                                .find(|choice| &choice.label == correct_label)
                                .map(|choice| choice.text.clone())
                        })
                })
                .unwrap_or_else(|| {
                    question
                        .accepted_meanings
                        .first()
                        .cloned()
                        .unwrap_or_default()
                })
        } else {
            question
                .accepted_meanings
                .first()
                .cloned()
                .unwrap_or_default()
        };

        StudyResult {
            question_id: question.question_id.clone(),
            entry_source_id: question.entry_source_id.clone(),
            question_type: question.question_type.clone(),
            user_response: answer.response.clone(),
            normalized_response,
            correct_answer,
            outcome,
            response_time_ms: answer.response_time_ms,
            answered_at: answered_at.to_string(),
        }
    }

    fn evaluate_choice(question: &StudyQuestion, selected_label: &str) -> AnswerOutcome {
        let correct_label = match &question.correct_choice_label {
            Some(label) => label,
            None => return AnswerOutcome::Incorrect,
        };

        if selected_label.trim().is_empty() {
            return AnswerOutcome::Skipped;
        }

        if selected_label.trim() == correct_label.trim() {
            AnswerOutcome::Correct
        } else {
            AnswerOutcome::Incorrect
        }
    }

    fn evaluate_input(response: &str, accepted_meanings: &[String]) -> (AnswerOutcome, String) {
        let trimmed = response.trim();

        if trimmed.is_empty() {
            return (AnswerOutcome::Skipped, String::new());
        }

        let normalized_input = normalize_meaning(trimmed);

        // Check each accepted meaning
        for meaning in accepted_meanings {
            let norm_meaning = normalize_meaning(meaning);

            if normalized_input == norm_meaning {
                return (AnswerOutcome::Correct, normalized_input);
            }

            if norm_meaning.contains(&normalized_input) || normalized_input.contains(&norm_meaning)
            {
                return (AnswerOutcome::FuzzyCorrect, normalized_input);
            }
        }

        (AnswerOutcome::Incorrect, normalized_input)
    }
}

/// Normalize a meaning string for comparison.
pub fn normalize_meaning(text: &str) -> String {
    let mut result = String::with_capacity(text.len());

    // Convert fullwidth parens to halfwidth
    let converted: String = text
        .chars()
        .map(|c| match c {
            '\u{FF08}' => '(',
            '\u{FF09}' => ')',
            _ => c,
        })
        .collect();

    let mut chars = converted.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '(' => {
                // Skip parenthesized content
                while let Some(&next) = chars.peek() {
                    if next == ')' {
                        chars.next();
                        break;
                    }
                    chars.next();
                }
            }
            ' ' | '\t' | '\u{3000}' => {
                // Collapse whitespace
                if !result.is_empty() && !result.ends_with(' ') {
                    result.push(' ');
                }
            }
            ';' | '；' => break, // Take only first meaning
            _ => result.push(c),
        }
    }

    result.trim().to_string()
}
