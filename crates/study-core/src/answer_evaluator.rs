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

        let selected_label = selected_label.trim();
        if selected_label.is_empty() {
            return AnswerOutcome::Skipped;
        }

        if selected_label == correct_label.trim() {
            return AnswerOutcome::Correct;
        }

        if Self::selected_choice_matches_accepted_meaning(question, selected_label) {
            return AnswerOutcome::Correct;
        }

        AnswerOutcome::Incorrect
    }

    fn selected_choice_matches_accepted_meaning(
        question: &StudyQuestion,
        selected_label: &str,
    ) -> bool {
        let Some(selected_text) = question.choices.as_ref().and_then(|choices| {
            choices
                .iter()
                .find(|choice| choice.label.trim() == selected_label)
                .map(|choice| choice.text.as_str())
        }) else {
            return false;
        };

        meaning_text_matches_any_accepted(selected_text, &question.accepted_meanings)
    }

    fn evaluate_input(response: &str, accepted_meanings: &[String]) -> (AnswerOutcome, String) {
        let trimmed = response.trim();

        if trimmed.is_empty() {
            return (AnswerOutcome::Skipped, String::new());
        }

        let normalized_input = normalize_answer_text(trimmed);

        // Check each accepted meaning
        for meaning in accepted_meanings {
            let accepted_parts = normalized_meaning_parts(meaning);

            if accepted_parts.iter().any(|part| &normalized_input == part) {
                return (AnswerOutcome::Correct, normalized_input);
            }

            if is_fuzzy_meaning_match(&normalized_input, &accepted_parts) {
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

fn normalize_answer_text(text: &str) -> String {
    normalize_meaning_segment(text)
}

fn normalized_meaning_parts(text: &str) -> Vec<String> {
    split_meaning_segments(text)
        .into_iter()
        .map(|segment| normalize_meaning_segment(&segment))
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn split_meaning_segments(text: &str) -> Vec<String> {
    let converted: String = text
        .chars()
        .map(|c| match c {
            '\u{FF08}' => '(',
            '\u{FF09}' => ')',
            _ => c,
        })
        .collect();
    let mut result = Vec::new();
    let mut current = String::new();
    let mut paren_depth = 0u32;
    for ch in converted.chars() {
        match ch {
            '(' => {
                paren_depth += 1;
            }
            ')' => {
                paren_depth = paren_depth.saturating_sub(1);
            }
            ';' | '；' | ',' | '，' | '、' | '/' if paren_depth == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    result.push(trimmed.to_string());
                }
                current.clear();
            }
            _ if paren_depth == 0 => current.push(ch),
            _ => {}
        }
    }
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        result.push(trimmed.to_string());
    }
    result
}

fn normalize_meaning_segment(text: &str) -> String {
    text.chars()
        .filter_map(|ch| match ch {
            '\u{FF08}' | '\u{FF09}' | '(' | ')' => None,
            ' ' | '\t' | '\u{3000}' | '\r' | '\n' => None,
            ';' | '；' | ',' | '，' | '、' | '/' | '.' | '。' | ':' | '：' => None,
            _ if ch.is_control() => None,
            _ => Some(ch),
        })
        .collect::<String>()
        .trim()
        .to_string()
}

fn is_fuzzy_meaning_match(input: &str, accepted_parts: &[String]) -> bool {
    let input_tokens = meaningful_tokens(input);
    if input_tokens.is_empty() {
        return false;
    }

    accepted_parts.iter().any(|part| {
        if part.is_empty() {
            return false;
        }
        if input.chars().count() >= 2 && (part.contains(input) || input.contains(part)) {
            return true;
        }
        let accepted_tokens = meaningful_tokens(part);
        !accepted_tokens.is_empty()
            && input_tokens
                .iter()
                .any(|token| accepted_tokens.iter().any(|accepted| accepted == token))
    })
}

fn meaning_text_matches_any_accepted(candidate: &str, accepted_meanings: &[String]) -> bool {
    let candidate_parts = normalized_meaning_parts(candidate);
    if candidate_parts.is_empty() {
        return false;
    }
    let candidate_full = normalize_answer_text(candidate);

    accepted_meanings.iter().any(|accepted| {
        let accepted_parts = normalized_meaning_parts(accepted);
        if accepted_parts.is_empty() {
            return false;
        }
        let accepted_full = normalize_answer_text(accepted);
        if !candidate_full.is_empty() && candidate_full == accepted_full {
            return true;
        }
        candidate_parts.iter().any(|candidate_part| {
            candidate_part.chars().count() >= 2
                && !is_stopword(candidate_part)
                && accepted_parts.iter().any(|accepted_part| {
                    accepted_part.chars().count() >= 2
                        && !is_stopword(accepted_part)
                        && candidate_part == accepted_part
                })
        })
    })
}

fn meaningful_tokens(text: &str) -> Vec<String> {
    let chars = text.chars().collect::<Vec<_>>();
    let mut tokens = Vec::new();

    if chars.len() >= 2 && !is_stopword(text) {
        tokens.push(text.to_string());
    }

    for window_size in [4usize, 3, 2] {
        if chars.len() < window_size {
            continue;
        }
        for window in chars.windows(window_size) {
            let token = window.iter().collect::<String>();
            if !is_stopword(&token) {
                tokens.push(token);
            }
        }
    }

    tokens.sort();
    tokens.dedup();
    tokens
}

fn is_stopword(text: &str) -> bool {
    matches!(
        text,
        "" | "的"
            | "了"
            | "是"
            | "在"
            | "和"
            | "或"
            | "与"
            | "及"
            | "而"
            | "把"
            | "被"
            | "使"
            | "为"
            | "对"
            | "中"
            | "上"
            | "下"
            | "等"
            | "个"
            | "一种"
            | "某种"
    )
}

#[cfg(test)]
mod choice_tests {
    use super::AnswerEvaluator;
    use word_storage_core::models::{
        AnswerOutcome, ChoiceOption, QuestionType, StudyAnswer, StudyQuestion,
    };

    fn answer(response: &str) -> StudyAnswer {
        StudyAnswer {
            question_id: "q1".to_string(),
            response: response.to_string(),
            response_time_ms: 100,
        }
    }

    fn choice_question(accepted_meanings: Vec<&str>, correct_label: &str) -> StudyQuestion {
        StudyQuestion {
            question_id: "q1".to_string(),
            question_type: QuestionType::EnToCnChoice,
            entry_source_id: "entry".to_string(),
            word: "word".to_string(),
            part_of_speech: None,
            phonetic_us: None,
            phonetic_uk: None,
            prompt: "word".to_string(),
            accepted_meanings: accepted_meanings.into_iter().map(str::to_string).collect(),
            example_sentence: None,
            example_translation: None,
            choices: Some(vec![
                ChoiceOption {
                    label: "A".to_string(),
                    text: "intense; explosive; eruptive".to_string(),
                },
                ChoiceOption {
                    label: "B".to_string(),
                    text: "explosive; eruptive; impulsive".to_string(),
                },
                ChoiceOption {
                    label: "C".to_string(),
                    text: "calm; quiet".to_string(),
                },
                ChoiceOption {
                    label: "D".to_string(),
                    text: "portable; movable".to_string(),
                },
            ]),
            correct_choice_label: Some(correct_label.to_string()),
            question_index: 0,
            total_questions: 1,
        }
    }

    #[test]
    fn choice_accepts_alternate_correct_meaning_text() {
        let question = choice_question(vec!["explosive; eruptive; impulsive"], "B");

        let result = AnswerEvaluator::evaluate(&question, &answer("A"), "2026-05-01T00:00:00Z");

        assert_eq!(result.outcome, AnswerOutcome::Correct);
    }

    #[test]
    fn choice_rejects_option_with_only_partial_word_overlap() {
        let mut question = choice_question(vec!["explosive; eruptive; impulsive"], "B");
        question.choices.as_mut().unwrap()[0].text = "explosive device; tool".to_string();

        let result = AnswerEvaluator::evaluate(&question, &answer("A"), "2026-05-01T00:00:00Z");

        assert_eq!(result.outcome, AnswerOutcome::Incorrect);
    }
}

#[cfg(test)]
mod tests {
    use super::AnswerEvaluator;
    use word_storage_core::models::{AnswerOutcome, QuestionType, StudyAnswer, StudyQuestion};

    fn input_question(accepted_meanings: Vec<&str>) -> StudyQuestion {
        StudyQuestion {
            question_id: "q1".to_string(),
            question_type: QuestionType::EnToCnInput,
            entry_source_id: "entry".to_string(),
            word: "word".to_string(),
            part_of_speech: None,
            phonetic_us: None,
            phonetic_uk: None,
            prompt: "word".to_string(),
            accepted_meanings: accepted_meanings.into_iter().map(str::to_string).collect(),
            example_sentence: None,
            example_translation: None,
            choices: None,
            correct_choice_label: None,
            question_index: 0,
            total_questions: 1,
        }
    }

    fn answer(response: &str) -> StudyAnswer {
        StudyAnswer {
            question_id: "q1".to_string(),
            response: response.to_string(),
            response_time_ms: 100,
        }
    }

    #[test]
    fn input_accepts_later_meaning_segment_as_fuzzy_correct() {
        let question = input_question(vec!["动物脂，油脂；润滑油"]);

        let result =
            AnswerEvaluator::evaluate(&question, &answer("润滑油"), "2026-04-30T00:00:00Z");

        assert_eq!(result.outcome, AnswerOutcome::Correct);
    }

    #[test]
    fn input_rejects_single_stopword_substring_match() {
        let question = input_question(vec!["取代...的；替代的"]);

        let result = AnswerEvaluator::evaluate(&question, &answer("的"), "2026-04-30T00:00:00Z");

        assert_eq!(result.outcome, AnswerOutcome::Incorrect);
    }

    #[test]
    fn input_allows_meaningful_partial_match() {
        let question = input_question(vec!["取代...的；替代的"]);

        let result = AnswerEvaluator::evaluate(&question, &answer("取代"), "2026-04-30T00:00:00Z");

        assert_eq!(result.outcome, AnswerOutcome::FuzzyCorrect);
    }
}
