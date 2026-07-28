//! Question builder for study sessions.

use std::collections::HashSet;

use word_domain_models::{
    AnswerOutcome, ChoiceOption, EntryExample, MeaningZh, QuestionType, QuestionTypeWeight,
    SessionMode, StudyAnswer, StudyQuestion, StudyResult,
};

/// A word entry prepared for question generation.
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

/// Builds question sets from word entries.
pub struct QuestionBuilder;

#[derive(Debug, Clone)]
pub struct DomainModeRules {
    pub question_types: Vec<QuestionType>,
    pub loops_all_types_per_word: bool,
    pub batch_only: bool,
    pub from_wrong_pool: bool,
}

pub fn session_definition(mode: SessionMode) -> DomainModeRules {
    match mode {
        SessionMode::NewWord => DomainModeRules {
            question_types: QuestionType::all_four(),
            loops_all_types_per_word: true,
            batch_only: true,
            from_wrong_pool: false,
        },
        SessionMode::Review => DomainModeRules {
            question_types: QuestionType::all_four(),
            loops_all_types_per_word: false,
            batch_only: false,
            from_wrong_pool: false,
        },
        SessionMode::MixedTest => DomainModeRules {
            question_types: vec![
                QuestionType::EnToCnChoice,
                QuestionType::CnToEnChoice,
                QuestionType::EnToCnInput,
            ],
            loops_all_types_per_word: false,
            batch_only: false,
            from_wrong_pool: false,
        },
        SessionMode::WrongWordReinforcement => DomainModeRules {
            question_types: vec![
                QuestionType::EnToCnChoice,
                QuestionType::CnToEnChoice,
                QuestionType::EnToCnInput,
            ],
            loops_all_types_per_word: false,
            batch_only: false,
            from_wrong_pool: true,
        },
        SessionMode::RootAffix => DomainModeRules {
            question_types: vec![QuestionType::GlossToRootInput, QuestionType::RootToGlossInput],
            loops_all_types_per_word: false,
            batch_only: false,
            from_wrong_pool: false,
        },
    }
}

pub struct AnswerEvaluator;

impl AnswerEvaluator {
    pub fn evaluate(
        question: &StudyQuestion,
        answer: &StudyAnswer,
        answered_at: &str,
    ) -> StudyResult {
        let (outcome, normalized_response) = if question.question_type.is_choice_type() {
            (Self::evaluate_choice(question, &answer.response), None)
        } else if question.question_type == QuestionType::WordSkeletonInput {
            let normalized = normalize_english_word(&answer.response);
            let outcome = if answer.response.trim().is_empty() {
                AnswerOutcome::Skipped
            } else if question
                .accepted_meanings
                .iter()
                .any(|word| normalize_english_word(word) == normalized)
            {
                AnswerOutcome::Correct
            } else {
                AnswerOutcome::Incorrect
            };
            (outcome, Some(normalized))
        } else {
            let (outcome, normalized) = evaluate_meaning_input(
                &answer.response,
                &question.accepted_meanings,
            );
            (outcome, Some(normalized))
        };
        let correct_answer = if question.question_type.is_choice_type() {
            question
                .choices
                .as_ref()
                .and_then(|choices| {
                    Self::resolved_choice_label(question).and_then(|label| {
                        choices
                            .iter()
                            .find(|choice| choice.label.trim() == label)
                            .map(|choice| choice.text.clone())
                    })
                })
                .or_else(|| question.accepted_meanings.first().cloned())
                .unwrap_or_default()
        } else {
            question.accepted_meanings.first().cloned().unwrap_or_default()
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

    fn evaluate_choice(question: &StudyQuestion, response: &str) -> AnswerOutcome {
        if response.trim().is_empty() {
            return AnswerOutcome::Skipped;
        }
        match Self::resolved_choice_label(question) {
            Some(label) if label == response.trim() => AnswerOutcome::Correct,
            _ => AnswerOutcome::Incorrect,
        }
    }

    fn resolved_choice_label(question: &StudyQuestion) -> Option<&str> {
        let choices = question.choices.as_ref()?;
        if question.question_type == QuestionType::CnToEnChoice {
            let word = normalize_english_word(&question.word);
            if let Some(choice) = choices
                .iter()
                .find(|choice| normalize_english_word(&choice.text) == word)
            {
                return Some(choice.label.trim());
            }
        } else {
            let meanings = question
                .accepted_meanings
                .iter()
                .map(|meaning| normalize_meaning_segment(meaning))
                .collect::<Vec<_>>();
            if let Some(choice) = choices.iter().find(|choice| {
                let text = normalize_meaning_segment(&choice.text);
                !text.is_empty() && meanings.iter().any(|meaning| meaning == &text)
            }) {
                return Some(choice.label.trim());
            }
        }
        question.correct_choice_label.as_deref().and_then(|label| {
            choices
                .iter()
                .any(|choice| choice.label.trim() == label.trim())
                .then_some(label.trim())
        })
    }
}

fn evaluate_meaning_input(response: &str, meanings: &[String]) -> (AnswerOutcome, String) {
    if response.trim().is_empty() {
        return (AnswerOutcome::Skipped, String::new());
    }
    let normalized = normalize_meaning_segment(response);
    for meaning in meanings {
        if normalized == normalize_meaning_segment(meaning) {
            return (AnswerOutcome::Correct, normalized);
        }
        let parts = split_meaning_segments(meaning);
        if parts.iter().any(|part| part == &normalized)
            || (!is_stopword(&normalized)
                && normalized.chars().count() >= 2
                && parts.iter().any(|part| {
                    part.contains(&normalized) || normalized.contains(part)
                }))
        {
            return (AnswerOutcome::FuzzyCorrect, normalized);
        }
    }
    (AnswerOutcome::Incorrect, normalized)
}

fn normalize_english_word(text: &str) -> String {
    text.chars()
        .filter(|ch| ch.is_ascii_alphabetic() || matches!(ch, '-' | '\''))
        .flat_map(char::to_lowercase)
        .collect()
}

fn split_meaning_segments(text: &str) -> Vec<String> {
    text.split([';', '\u{ff1b}', ',', '\u{ff0c}', '\u{3002}', '/'])
        .map(normalize_meaning_segment)
        .filter(|part| !part.is_empty())
        .collect()
}

fn normalize_meaning_segment(text: &str) -> String {
    let mut parenthesis_depth = 0u32;
    text.chars()
        .filter_map(|ch| match ch {
            '(' | '\u{ff08}' => {
                parenthesis_depth += 1;
                None
            }
            ')' | '\u{ff09}' => {
                parenthesis_depth = parenthesis_depth.saturating_sub(1);
                None
            }
            _ if parenthesis_depth > 0 || ch.is_whitespace() || ch.is_control() => None,
            ';' | '\u{ff1b}' | ',' | '\u{ff0c}' | '\u{3002}' | '/' | '.' | ':' | '\u{ff1a}' => None,
            _ => Some(ch),
        })
        .collect()
}

fn is_stopword(text: &str) -> bool {
    matches!(text, "" | "\u{7684}" | "\u{4e86}" | "\u{662f}" | "\u{5728}" | "\u{548c}")
}

impl QuestionBuilder {
    /// Generate the full question set for a list of words in a given session mode.
    pub fn build_session_questions(
        mode: &SessionMode,
        words: &[WordForQuestion],
        distractors: &[WordForQuestion],
        session_id: &str,
        question_type_weights: &[QuestionTypeWeight],
    ) -> Vec<StudyQuestion> {
        let distractors = if distractors.is_empty() { words } else { distractors };
        if matches!(mode, SessionMode::RootAffix) {
            Self::build_root_affix_questions(words, session_id)
        } else if matches!(mode, SessionMode::NewWord) {
            Self::build_loop_questions(words, distractors, session_id)
        } else if matches!(mode, SessionMode::WrongWordReinforcement) {
            Self::build_weighted_pool_questions(
                words,
                distractors,
                session_id,
                question_type_weights,
                Self::default_wrong_word_types(),
            )
        } else {
            let fallback_types = match mode {
                SessionMode::Review => QuestionType::all_four(),
                SessionMode::MixedTest => vec![
                    QuestionType::EnToCnChoice,
                    QuestionType::CnToEnChoice,
                    QuestionType::EnToCnInput,
                ],
                _ => QuestionType::all_four(),
            };
            Self::build_weighted_pool_questions(
                words,
                distractors,
                session_id,
                question_type_weights,
                fallback_types,
            )
        }
    }

    fn build_loop_questions(
        words: &[WordForQuestion],
        distractors: &[WordForQuestion],
        session_id: &str,
    ) -> Vec<StudyQuestion> {
        let types = QuestionType::all_four();
        let total_questions = (words.len() as u32) * (types.len() as u32);
        let mut questions = Vec::with_capacity(total_questions as usize);
        let mut question_index = 0u32;
        let mut used_distractors = HashSet::new();
        let mut previous_choice_label: Option<String> = None;

        for (type_round, qt) in types.iter().enumerate() {
            for word in Self::ordered_words_for_round(words, session_id, type_round) {
                let question = Self::build_single_question_with_used(
                    word,
                    qt,
                    distractors,
                    session_id,
                    question_index,
                    total_questions,
                    &mut used_distractors,
                    previous_choice_label.as_deref(),
                );
                if question.correct_choice_label.is_some() {
                    previous_choice_label = question.correct_choice_label.clone();
                }
                questions.push(question);
                question_index += 1;
            }
        }

        questions
    }

    fn build_root_affix_questions(
        words: &[WordForQuestion],
        session_id: &str,
    ) -> Vec<StudyQuestion> {
        let total_questions = words.len() as u32;
        let mut questions = Vec::with_capacity(total_questions as usize);
        let mut question_index = 0u32;

        for word in words {
            questions.push(Self::build_root_affix_question(
                word,
                QuestionType::RootToGlossInput,
                session_id,
                question_index,
                total_questions,
            ));
            question_index += 1;
        }

        questions
    }

    fn default_wrong_word_types() -> Vec<QuestionType> {
        vec![
            QuestionType::ExampleToCnChoice,
            QuestionType::EnToCnChoice,
            QuestionType::EnToCnInput,
            QuestionType::EnToCnChoice,
            QuestionType::EnToCnInput,
            QuestionType::EnToCnChoice,
            QuestionType::CnToEnChoice,
            QuestionType::EnToCnInput,
        ]
    }

    fn build_weighted_pool_questions(
        words: &[WordForQuestion],
        distractors: &[WordForQuestion],
        session_id: &str,
        question_type_weights: &[QuestionTypeWeight],
        fallback_types: Vec<QuestionType>,
    ) -> Vec<StudyQuestion> {
        let types = Self::question_type_sequence_for_count(
            words.len(),
            question_type_weights,
            fallback_types,
        );
        let total_questions = words.len() as u32;
        let mut questions = Vec::with_capacity(total_questions as usize);
        let mut question_index = 0u32;
        let mut used_distractors = HashSet::new();
        let mut previous_choice_label: Option<String> = None;

        for word in words {
            let qt = &types[(question_index as usize) % types.len()];
            let question = Self::build_single_question_with_used(
                word,
                qt,
                distractors,
                session_id,
                question_index,
                total_questions,
                &mut used_distractors,
                previous_choice_label.as_deref(),
            );
            if question.correct_choice_label.is_some() {
                previous_choice_label = question.correct_choice_label.clone();
            }
            questions.push(question);
            question_index += 1;
        }

        questions
    }

    fn question_type_sequence_for_count(
        count: usize,
        question_type_weights: &[QuestionTypeWeight],
        fallback_types: Vec<QuestionType>,
    ) -> Vec<QuestionType> {
        if count == 0 {
            return fallback_types;
        }
        let total_weight: u32 = question_type_weights.iter().map(|item| item.weight).sum();
        if total_weight == 0 {
            return fallback_types;
        }
        let mut sequence = Vec::with_capacity(count);
        let mut slots = question_type_weights
            .iter()
            .filter(|item| item.weight > 0)
            .map(|item| {
                let quota =
                    ((item.weight as f64 / total_weight as f64) * count as f64).round() as usize;
                (item.question_type.clone(), quota.max(1))
            })
            .collect::<Vec<_>>();
        while slots.iter().map(|(_, quota)| *quota).sum::<usize>() > count {
            if let Some((_, quota)) = slots.iter_mut().find(|(_, quota)| *quota > 1) {
                *quota -= 1;
            } else {
                break;
            }
        }
        while slots.iter().map(|(_, quota)| *quota).sum::<usize>() < count {
            if let Some((_, quota)) = slots.first_mut() {
                *quota += 1;
            }
        }
        while sequence.len() < count && slots.iter().any(|(_, quota)| *quota > 0) {
            for (question_type, quota) in slots.iter_mut() {
                if *quota == 0 {
                    continue;
                }
                sequence.push(question_type.clone());
                *quota -= 1;
                if sequence.len() >= count {
                    break;
                }
            }
        }
        if sequence.is_empty() {
            fallback_types
        } else {
            sequence
        }
    }

    #[cfg(test)]
    fn build_single_question(
        word: &WordForQuestion,
        question_type: &QuestionType,
        distractors: &[WordForQuestion],
        session_id: &str,
        question_index: u32,
        total_questions: u32,
    ) -> StudyQuestion {
        let mut used_distractors = HashSet::new();
        Self::build_single_question_with_used(
            word,
            question_type,
            distractors,
            session_id,
            question_index,
            total_questions,
            &mut used_distractors,
            None,
        )
    }

    fn build_single_question_with_used(
        word: &WordForQuestion,
        question_type: &QuestionType,
        distractors: &[WordForQuestion],
        session_id: &str,
        question_index: u32,
        total_questions: u32,
        used_distractors: &mut HashSet<String>,
        previous_choice_label: Option<&str>,
    ) -> StudyQuestion {
        let accepted_meanings: Vec<String> =
            word.meanings.iter().map(|m| m.meaning_cn.clone()).collect();

        let question_id = format!("{}_{}", session_id, question_index);

        match question_type {
            QuestionType::EnToCnChoice => {
                let primary_meaning = Self::primary_meaning_for_question(word, None);
                let accepted_meanings = vec![Self::sanitize_choice_text(&primary_meaning)];
                let excluded_meanings = Self::choice_meanings_for_word(word);
                let (choices, correct_label) = Self::build_cn_choices(
                    word,
                    &primary_meaning,
                    distractors,
                    &excluded_meanings,
                    word.part_of_speech.as_deref(),
                    session_id,
                    question_index,
                    used_distractors,
                    previous_choice_label,
                );
                StudyQuestion {
                    question_id,
                    question_type: QuestionType::EnToCnChoice,
                    entry_source_id: word.source_id.clone(),
                    word: word.word.clone(),
                    part_of_speech: word.part_of_speech.clone(),
                    phonetic_us: word.phonetic_us.clone(),
                    phonetic_uk: word.phonetic_uk.clone(),
                    prompt: word.word.clone(),
                    accepted_meanings,
                    example_sentence: None,
                    example_translation: None,
                    choices: Some(choices),
                    correct_choice_label: Some(correct_label),
                    question_index,
                    total_questions,
                }
            }
            QuestionType::ExampleToCnChoice | QuestionType::ExampleToCnChoiceNoTranslation => {
                let primary_meaning = Self::primary_meaning_for_question(word, None);
                let example = Self::example_matching_meaning(word, &primary_meaning);
                let prompt = example
                    .map(|e| e.sentence_en.clone())
                    .unwrap_or_else(|| word.word.clone());
                let contextual_meaning = example
                    .map(|example| {
                        Self::primary_meaning_for_question(word, Some(&example.sentence_cn))
                    })
                    .unwrap_or(primary_meaning);
                let example_sentence = example.map(|e| e.sentence_en.clone());
                let accepted_meanings = vec![Self::sanitize_choice_text(&contextual_meaning)];
                let excluded_meanings = Self::choice_meanings_for_word(word);
                let (choices, correct_label) = Self::build_cn_choices(
                    word,
                    &contextual_meaning,
                    distractors,
                    &excluded_meanings,
                    word.part_of_speech.as_deref(),
                    session_id,
                    question_index,
                    used_distractors,
                    previous_choice_label,
                );
                StudyQuestion {
                    question_id,
                    question_type: question_type.clone(),
                    entry_source_id: word.source_id.clone(),
                    word: word.word.clone(),
                    part_of_speech: word.part_of_speech.clone(),
                    phonetic_us: word.phonetic_us.clone(),
                    phonetic_uk: word.phonetic_uk.clone(),
                    prompt,
                    accepted_meanings,
                    example_sentence,
                    // Translations are feedback-only and must not leak before submission.
                    example_translation: None,
                    choices: Some(choices),
                    correct_choice_label: Some(correct_label),
                    question_index,
                    total_questions,
                }
            }
            QuestionType::CnToEnChoice => {
                let prompt = accepted_meanings
                    .first()
                    .map(|meaning| Self::sanitize_choice_text(meaning))
                    .unwrap_or_default();
                let (choices, correct_label) = Self::build_en_choices(
                    word,
                    distractors,
                    session_id,
                    question_index,
                    used_distractors,
                    previous_choice_label,
                );
                StudyQuestion {
                    question_id,
                    question_type: QuestionType::CnToEnChoice,
                    entry_source_id: word.source_id.clone(),
                    word: word.word.clone(),
                    part_of_speech: word.part_of_speech.clone(),
                    phonetic_us: word.phonetic_us.clone(),
                    phonetic_uk: word.phonetic_uk.clone(),
                    prompt,
                    accepted_meanings,
                    example_sentence: None,
                    example_translation: None,
                    choices: Some(choices),
                    correct_choice_label: Some(correct_label),
                    question_index,
                    total_questions,
                }
            }
            QuestionType::EnToCnInput => StudyQuestion {
                question_id,
                question_type: QuestionType::EnToCnInput,
                entry_source_id: word.source_id.clone(),
                word: word.word.clone(),
                part_of_speech: word.part_of_speech.clone(),
                phonetic_us: word.phonetic_us.clone(),
                phonetic_uk: word.phonetic_uk.clone(),
                prompt: word.word.clone(),
                accepted_meanings,
                example_sentence: None,
                example_translation: None,
                choices: None,
                correct_choice_label: None,
                question_index,
                total_questions,
            },
            QuestionType::WordSkeletonInput => StudyQuestion {
                question_id,
                question_type: QuestionType::WordSkeletonInput,
                entry_source_id: word.source_id.clone(),
                word: word.word.clone(),
                part_of_speech: word.part_of_speech.clone(),
                phonetic_us: word.phonetic_us.clone(),
                phonetic_uk: word.phonetic_uk.clone(),
                prompt: Self::word_skeleton_prompt(&word.word),
                accepted_meanings: vec![Self::word_skeleton_missing_text(&word.word)],
                example_sentence: None,
                example_translation: accepted_meanings.first().cloned(),
                choices: None,
                correct_choice_label: None,
                question_index,
                total_questions,
            },
            QuestionType::GlossToRootInput | QuestionType::RootToGlossInput => {
                unreachable!("root/affix questions are built by build_root_affix_question")
            }
        }
    }

    fn build_root_affix_question(
        word: &WordForQuestion,
        question_type: QuestionType,
        session_id: &str,
        question_index: u32,
        total_questions: u32,
    ) -> StudyQuestion {
        let question_id = format!("{}_{}", session_id, question_index);
        let example = word.examples.first();
        let example_sentence = example.map(|e| e.sentence_en.clone());
        let example_translation = example.map(|e| e.sentence_cn.clone());
        let meaning = word
            .meanings
            .first()
            .map(|m| m.meaning_cn.clone())
            .unwrap_or_default();

        match question_type {
            QuestionType::GlossToRootInput => StudyQuestion {
                question_id,
                question_type,
                entry_source_id: word.source_id.clone(),
                word: meaning.clone(),
                part_of_speech: None,
                phonetic_us: None,
                phonetic_uk: None,
                prompt: meaning,
                accepted_meanings: vec![
                    word.word.clone(),
                    word.word.replace('-', "").replace(' ', "").to_lowercase(),
                ],
                example_sentence,
                example_translation,
                choices: None,
                correct_choice_label: None,
                question_index,
                total_questions,
            },
            QuestionType::RootToGlossInput => StudyQuestion {
                question_id,
                question_type,
                entry_source_id: word.source_id.clone(),
                word: word.word.clone(),
                part_of_speech: None,
                phonetic_us: None,
                phonetic_uk: None,
                prompt: word.word.clone(),
                accepted_meanings: vec![meaning],
                example_sentence,
                example_translation,
                choices: None,
                correct_choice_label: None,
                question_index,
                total_questions,
            },
            _ => unreachable!("non-root question passed to build_root_affix_question"),
        }
    }

    fn build_cn_choices(
        word: &WordForQuestion,
        correct_text: &str,
        distractors: &[WordForQuestion],
        accepted_meanings: &[String],
        target_pos: Option<&str>,
        session_id: &str,
        question_index: u32,
        used_distractors: &mut HashSet<String>,
        previous_choice_label: Option<&str>,
    ) -> (Vec<ChoiceOption>, String) {
        let correct_text = Self::sanitize_choice_text(correct_text);
        let mut excluded_meanings: HashSet<String> = accepted_meanings
            .iter()
            .map(|meaning| Self::sanitize_choice_text(meaning))
            .collect();
        let mut distractor_texts = Self::collect_precomputed_distractor_texts(
            &word.cn_choice_distractors,
            question_index,
            &word.source_id,
            &correct_text,
            |text| !excluded_meanings.contains(text) && !used_distractors.contains(text),
        );
        if distractor_texts.len() < 3 && target_pos.is_some() {
            let mut seen: HashSet<String> = distractor_texts.iter().cloned().collect();
            for text in Self::collect_ranked_distractor_texts(
                distractors,
                question_index,
                &correct_text,
                &correct_text,
                |candidate| Self::choice_meanings_for_distractor(candidate, target_pos),
                |text| !excluded_meanings.contains(text) && !used_distractors.contains(text),
            ) {
                if seen.insert(text.clone()) {
                    distractor_texts.push(text);
                }
                if distractor_texts.len() >= 3 {
                    break;
                }
            }
        }

        for text in &distractor_texts {
            excluded_meanings.insert(text.clone());
        }

        if distractor_texts.len() < 3 {
            let fallback_texts = Self::collect_ranked_distractor_texts(
                distractors,
                question_index,
                &correct_text,
                &correct_text,
                Self::all_meanings_for_word,
                |text| !excluded_meanings.contains(text) && !used_distractors.contains(text),
            );
            for text in fallback_texts {
                if distractor_texts.len() >= 3 {
                    break;
                }
                excluded_meanings.insert(text.clone());
                distractor_texts.push(text);
            }
        }

        if distractor_texts.len() < 3 {
            let relaxed_texts = Self::collect_ranked_distractor_texts(
                distractors,
                question_index,
                &correct_text,
                &correct_text,
                Self::all_meanings_for_word,
                |text| !excluded_meanings.contains(text),
            );
            for text in relaxed_texts {
                if distractor_texts.len() >= 3 {
                    break;
                }
                if !distractor_texts.contains(&text) {
                    excluded_meanings.insert(text.clone());
                    distractor_texts.push(text);
                }
            }
        }
        for text in &distractor_texts {
            used_distractors.insert(text.clone());
        }

        let choice_count = distractor_texts.len() + 1;
        let labels = ["A", "B", "C", "D"];
        let mut options: Vec<ChoiceOption> = distractor_texts
            .into_iter()
            .map(|text| ChoiceOption {
                text,
                label: String::new(),
            })
            .collect();

        let correct_pos = Self::stable_choice_position(
            choice_count,
            &format!(
                "{session_id}:cn:{}:{question_index}:{}:{correct_text}",
                word.source_id, word.word
            ),
            previous_choice_label,
        );
        options.insert(
            correct_pos,
            ChoiceOption {
                text: correct_text,
                label: String::new(),
            },
        );

        for (i, opt) in options.iter_mut().enumerate() {
            opt.label = labels[i].to_string();
        }

        let correct_label = labels[correct_pos].to_string();
        (options, correct_label)
    }

    fn build_en_choices(
        word: &WordForQuestion,
        distractors: &[WordForQuestion],
        session_id: &str,
        question_index: u32,
        used_distractors: &mut HashSet<String>,
        previous_choice_label: Option<&str>,
    ) -> (Vec<ChoiceOption>, String) {
        let correct_text = word.word.clone();
        let labels = ["A", "B", "C", "D"];
        let correct_meaning = word
            .meanings
            .first()
            .map(|meaning| meaning.meaning_cn.as_str())
            .unwrap_or("");
        let mut distractor_texts = Self::collect_precomputed_distractor_texts(
            &word.en_choice_distractors,
            question_index,
            &word.source_id,
            correct_meaning,
            |text| text != &correct_text && !used_distractors.contains(text),
        );
        if distractor_texts.len() < 3 {
            let mut seen: HashSet<String> = distractor_texts.iter().cloned().collect();
            for text in Self::collect_ranked_distractor_texts(
                distractors,
                question_index,
                &word.source_id,
                correct_meaning,
                |candidate| {
                    if same_part_of_speech(
                        word.part_of_speech.as_deref(),
                        candidate.part_of_speech.as_deref(),
                    ) {
                        vec![candidate.word.clone()]
                    } else {
                        Vec::new()
                    }
                },
                |text| text != &correct_text && !used_distractors.contains(text),
            ) {
                if seen.insert(text.clone()) {
                    distractor_texts.push(text);
                }
                if distractor_texts.len() >= 3 {
                    break;
                }
            }
        }
        if distractor_texts.len() < 3 {
            let mut seen: HashSet<String> = distractor_texts.iter().cloned().collect();
            for text in Self::collect_ranked_distractor_texts(
                distractors,
                question_index,
                &word.source_id,
                correct_meaning,
                |candidate| vec![candidate.word.clone()],
                |text| text != &correct_text && !used_distractors.contains(text),
            ) {
                if seen.insert(text.clone()) {
                    distractor_texts.push(text);
                }
                if distractor_texts.len() >= 3 {
                    break;
                }
            }
        }
        if distractor_texts.len() < 3 {
            let mut seen: HashSet<String> = distractor_texts.iter().cloned().collect();
            for text in Self::collect_ranked_distractor_texts(
                distractors,
                question_index,
                &word.source_id,
                correct_meaning,
                |candidate| vec![candidate.word.clone()],
                |text| text != &correct_text,
            ) {
                if seen.insert(text.clone()) {
                    distractor_texts.push(text);
                }
                if distractor_texts.len() >= 3 {
                    break;
                }
            }
        }
        for text in &distractor_texts {
            used_distractors.insert(text.clone());
        }

        let choice_count = distractor_texts.len() + 1;
        let mut options: Vec<ChoiceOption> = distractor_texts
            .into_iter()
            .map(|text| ChoiceOption {
                text,
                label: String::new(),
            })
            .collect();

        let correct_pos = Self::stable_choice_position(
            choice_count,
            &format!(
                "{session_id}:en:{}:{question_index}:{}",
                word.source_id, correct_text
            ),
            previous_choice_label,
        );
        options.insert(
            correct_pos,
            ChoiceOption {
                text: correct_text.clone(),
                label: String::new(),
            },
        );

        for (i, opt) in options.iter_mut().enumerate() {
            opt.label = labels[i].to_string();
        }

        let correct_label = labels[correct_pos].to_string();
        (options, correct_label)
    }

    fn stable_choice_position(
        choice_count: usize,
        seed_input: &str,
        previous_choice_label: Option<&str>,
    ) -> usize {
        if choice_count <= 1 {
            return 0;
        }
        let labels = ["A", "B", "C", "D"];
        let seed = Self::stable_distractor_seed(seed_input);
        let mut position = (seed % choice_count as u64) as usize;
        if previous_choice_label == Some(labels[position]) {
            let alternate_offset =
                ((seed / choice_count as u64) % (choice_count - 1) as u64) as usize;
            position = (position + 1 + alternate_offset) % choice_count;
        }
        position
    }

    fn collect_ranked_distractor_texts<FMap, FFilter>(
        distractors: &[WordForQuestion],
        question_index: u32,
        source_id: &str,
        correct_hint: &str,
        values_for_word: FMap,
        should_keep: FFilter,
    ) -> Vec<String>
    where
        FMap: Fn(&WordForQuestion) -> Vec<String>,
        FFilter: Fn(&String) -> bool,
    {
        let mut candidates = Vec::<(usize, String)>::new();
        let mut seen = HashSet::new();
        let correct_norm = normalize_context_text(correct_hint);

        for candidate in distractors {
            for text in values_for_word(candidate) {
                if text.trim().is_empty() || !should_keep(&text) || !seen.insert(text.clone()) {
                    continue;
                }
                let text_norm = normalize_context_text(&text);
                let semantic_score = distractor_similarity_score(&correct_norm, &text_norm);
                candidates.push((semantic_score, text));
            }
        }

        candidates.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
        let quality_window = Self::distractor_quality_window(&candidates);
        let mut pool = candidates
            .into_iter()
            .take(quality_window)
            .collect::<Vec<_>>();
        pool.sort_by(|left, right| {
            let left_key = Self::stable_distractor_seed(&format!(
                "{source_id}:{question_index}:{correct_hint}:{}",
                left.1
            ));
            let right_key = Self::stable_distractor_seed(&format!(
                "{source_id}:{question_index}:{correct_hint}:{}",
                right.1
            ));
            left_key.cmp(&right_key).then_with(|| left.1.cmp(&right.1))
        });
        pool.into_iter().take(3).map(|(_, text)| text).collect()
    }

    fn collect_precomputed_distractor_texts<FFilter>(
        candidates: &[String],
        question_index: u32,
        source_id: &str,
        correct_hint: &str,
        should_keep: FFilter,
    ) -> Vec<String>
    where
        FFilter: Fn(&String) -> bool,
    {
        let mut ranked = Vec::<(usize, String)>::new();
        let mut seen = HashSet::new();
        let correct_norm = normalize_context_text(correct_hint);

        for candidate in candidates {
            let text = Self::sanitize_choice_text(candidate);
            if text.trim().is_empty() || !should_keep(&text) || !seen.insert(text.clone()) {
                continue;
            }
            let text_norm = normalize_context_text(&text);
            let semantic_score = distractor_similarity_score(&correct_norm, &text_norm);
            ranked.push((semantic_score, text));
        }

        ranked.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
        ranked.sort_by(|left, right| {
            let left_key = Self::stable_distractor_seed(&format!(
                "{source_id}:{question_index}:{correct_hint}:{}",
                left.1
            ));
            let right_key = Self::stable_distractor_seed(&format!(
                "{source_id}:{question_index}:{correct_hint}:{}",
                right.1
            ));
            left_key.cmp(&right_key).then_with(|| left.1.cmp(&right.1))
        });
        ranked.into_iter().take(3).map(|(_, text)| text).collect()
    }

    fn distractor_quality_window(candidates: &[(usize, String)]) -> usize {
        if candidates.is_empty() {
            return 0;
        }

        if candidates.len() <= 8 {
            let mut window = candidates.len().min(3);
            while window < candidates.len() {
                let previous_score = candidates[window - 1].0;
                let next_score = candidates[window].0;
                if previous_score != next_score {
                    break;
                }
                window += 1;
            }
            return window;
        }

        let minimum_window = candidates.len().min(16);
        let mut window = candidates.len().min(3);
        while window < candidates.len() && window < 64 {
            let previous_score = candidates[window - 1].0;
            let next_score = candidates[window].0;
            if window >= minimum_window && previous_score != next_score {
                break;
            }
            window += 1;
        }
        window
    }

    fn stable_seed(text: &str) -> u64 {
        text.bytes().fold(0u64, |acc, byte| {
            acc.wrapping_mul(33).wrapping_add(u64::from(byte))
        })
    }

    fn stable_distractor_seed(text: &str) -> u64 {
        text.bytes().fold(0xcbf29ce484222325u64, |acc, byte| {
            (acc ^ u64::from(byte)).wrapping_mul(0x100000001b3u64)
        })
    }

    fn choice_meanings_for_word(word: &WordForQuestion) -> Vec<String> {
        let preferred = Self::meanings_matching_target_pos(word, word.part_of_speech.as_deref());
        if preferred.is_empty() {
            word.meanings
                .iter()
                .map(|meaning| Self::sanitize_choice_text(&meaning.meaning_cn))
                .collect()
        } else {
            preferred
                .iter()
                .map(|meaning| Self::sanitize_choice_text(&meaning.meaning_cn))
                .collect()
        }
    }

    fn choice_meanings_for_distractor(
        word: &WordForQuestion,
        target_pos: Option<&str>,
    ) -> Vec<String> {
        Self::meanings_matching_target_pos(word, target_pos)
            .iter()
            .map(|meaning| Self::sanitize_choice_text(&meaning.meaning_cn))
            .collect()
    }

    fn all_meanings_for_word(word: &WordForQuestion) -> Vec<String> {
        word.meanings
            .iter()
            .map(|meaning| Self::sanitize_choice_text(&meaning.meaning_cn))
            .collect()
    }

    fn sanitize_choice_text(value: &str) -> String {
        let chars = value.trim().chars().collect::<Vec<_>>();
        let mut cleaned = String::new();
        let mut index = 0;
        while index < chars.len() {
            let ch = chars[index];
            if matches!(ch, 'A' | 'B' | 'C' | 'D') {
                let mut next = index + 1;
                while next < chars.len() && chars[next].is_whitespace() {
                    next += 1;
                }
                let has_option_separator = next < chars.len()
                    && matches!(chars[next], ';' | '；' | ':' | '：' | '.' | '．');
                let previous_is_boundary = cleaned
                    .chars()
                    .last()
                    .map(|last| last.is_whitespace() || matches!(last, ';' | '；' | ',' | '，'))
                    .unwrap_or(true);
                if has_option_separator && previous_is_boundary {
                    if !cleaned.ends_with('；') {
                        cleaned.push('；');
                    }
                    index = next + 1;
                    while index < chars.len() && chars[index].is_whitespace() {
                        index += 1;
                    }
                    continue;
                }
            }

            let normalized = match ch {
                '\u{fffd}' => None,
                '<' | '>' => None,
                '\u{00a0}' | '\t' | '\r' | '\n' => Some(' '),
                '；' | ';' => Some('；'),
                '，' | ',' => Some('，'),
                _ if ch.is_control() => None,
                _ => Some(ch),
            };
            if let Some(ch) = normalized {
                cleaned.push(ch);
            }
            index += 1;
        }
        let mut cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
        for separator in ['；', '，'] {
            cleaned = cleaned.replace(&format!(" {separator}"), &separator.to_string());
            cleaned = cleaned.replace(&format!("{separator} "), &separator.to_string());
        }
        if matches!(cleaned.trim(), "" | "/" | "\\" | "-" | "—" | "——") {
            return String::new();
        }
        cleaned
    }

    fn primary_meaning_for_question(
        word: &WordForQuestion,
        example_translation: Option<&str>,
    ) -> String {
        let preferred = Self::meanings_matching_target_pos(word, word.part_of_speech.as_deref());
        let candidate_meanings: Vec<String> = if preferred.is_empty() {
            word.meanings
                .iter()
                .map(|meaning| meaning.meaning_cn.clone())
                .collect()
        } else {
            preferred
                .iter()
                .map(|meaning| meaning.meaning_cn.clone())
                .collect()
        };
        if candidate_meanings.is_empty() {
            return String::new();
        }

        let Some(example_translation) = example_translation else {
            return candidate_meanings[0].clone();
        };

        let normalized_translation = normalize_context_text(example_translation);
        candidate_meanings
            .iter()
            .max_by_key(|meaning| {
                let normalized_meaning = normalize_context_text(meaning);
                context_overlap_score(&normalized_translation, &normalized_meaning)
            })
            .cloned()
            .unwrap_or_else(|| candidate_meanings[0].clone())
    }

    fn example_matching_meaning<'a>(
        word: &'a WordForQuestion,
        meaning: &str,
    ) -> Option<&'a EntryExample> {
        let normalized_meaning = normalize_context_text(meaning);
        word.examples
            .iter()
            .filter(|example| {
                context_overlap_score(
                    &normalize_context_text(&example.sentence_cn),
                    &normalized_meaning,
                ) > 0
            })
            .max_by_key(|example| {
                context_overlap_score(
                    &normalize_context_text(&example.sentence_cn),
                    &normalized_meaning,
                )
            })
    }

    fn meanings_matching_target_pos<'a>(
        word: &'a WordForQuestion,
        target_pos: Option<&str>,
    ) -> Vec<&'a MeaningZh> {
        let Some(target_pos) = target_pos.and_then(normalize_part_of_speech_tag) else {
            return Vec::new();
        };
        word.meanings
            .iter()
            .filter(|meaning| normalize_part_of_speech_tag(&meaning.pos) == Some(target_pos))
            .collect()
    }

    fn ordered_words_for_round<'a>(
        words: &'a [WordForQuestion],
        session_id: &str,
        round_index: usize,
    ) -> Vec<&'a WordForQuestion> {
        let mut ordered: Vec<&WordForQuestion> = words.iter().collect();
        ordered.sort_by(|left, right| {
            Self::stable_seed(&format!("{session_id}:base:{}", left.source_id))
                .cmp(&Self::stable_seed(&format!(
                    "{session_id}:base:{}",
                    right.source_id
                )))
                .then_with(|| left.source_id.cmp(&right.source_id))
        });
        if ordered.len() > 1 {
            let shift = round_index % ordered.len();
            ordered.rotate_left(shift);
            if round_index % 2 == 1 {
                ordered.reverse();
            }
        }
        ordered
    }

    fn word_skeleton_prompt(word: &str) -> String {
        Self::word_skeleton_parts(word).0
    }

    fn word_skeleton_missing_text(word: &str) -> String {
        Self::word_skeleton_parts(word).1
    }

    fn word_skeleton_parts(word: &str) -> (String, String) {
        let chars = word.chars().collect::<Vec<_>>();
        if chars.is_empty() {
            return (String::new(), String::new());
        }
        let hide_start = if chars.len() <= 2 { 0usize } else { 1usize };
        let hide_count = if chars.len() <= 4 {
            chars.len().saturating_sub(2).max(1)
        } else {
            ((chars.len() / 3).max(2)).min(chars.len().saturating_sub(2))
        };
        let hide_end = (hide_start + hide_count).min(chars.len());
        let mut skeleton = String::with_capacity(word.len());
        let mut missing = String::new();
        for (index, ch) in chars.iter().enumerate() {
            if index >= hide_start && index < hide_end && ch.is_ascii_alphabetic() {
                skeleton.push('_');
                missing.push(*ch);
            } else {
                skeleton.push(*ch);
            }
        }
        (skeleton, missing)
    }
}

fn normalize_part_of_speech_tag(value: &str) -> Option<&'static str> {
    let trimmed = value.trim().to_ascii_lowercase();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with('v') {
        return Some("v");
    }
    if trimmed.starts_with('n') {
        return Some("n");
    }
    if trimmed.starts_with("adj") {
        return Some("adj");
    }
    if trimmed.starts_with("adv") {
        return Some("adv");
    }
    None
}

fn same_part_of_speech(left: Option<&str>, right: Option<&str>) -> bool {
    match (
        left.and_then(normalize_part_of_speech_tag),
        right.and_then(normalize_part_of_speech_tag),
    ) {
        (Some(left), Some(right)) => left == right,
        _ => false,
    }
}

fn normalize_context_text(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != '，' && *ch != '。' && *ch != ',' && *ch != '.')
        .collect()
}

fn context_overlap_score(translation: &str, meaning: &str) -> usize {
    if meaning.is_empty() {
        return 0;
    }
    if translation.contains(meaning) {
        return meaning.chars().count() + 100;
    }
    meaning
        .chars()
        .filter(|ch| translation.contains(*ch))
        .count()
}

fn distractor_similarity_score(correct: &str, candidate: &str) -> usize {
    if correct.is_empty() || candidate.is_empty() {
        return 0;
    }
    let overlap = candidate
        .chars()
        .filter(|ch| correct.contains(*ch))
        .count()
        .saturating_mul(8);
    let len_gap = correct.chars().count().abs_diff(candidate.chars().count());
    overlap.saturating_sub(len_gap)
}

#[cfg(test)]
mod tests {
    use super::{QuestionBuilder, WordForQuestion};
    use std::collections::HashSet;
    use word_domain_models::{
        EntryExample, MeaningZh, QuestionType, QuestionTypeWeight, SessionMode,
    };

    #[test]
    fn stable_seed_has_fixed_width_cross_target_value() {
        assert_eq!(
            QuestionBuilder::stable_seed("sess-fixture-newword:base:entry-alpha"),
            9_252_296_553_957_568_985u64
        );
        assert_eq!(
            QuestionBuilder::stable_distractor_seed("sess-fixture-newword:0:alpha meaning"),
            7_815_142_371_858_687_363u64
        );
    }

    fn build_word(
        source_id: &str,
        word: &str,
        part_of_speech: Option<&str>,
        meanings: &[(&str, &str)],
        example_translation: Option<&str>,
    ) -> WordForQuestion {
        WordForQuestion {
            source_id: source_id.to_string(),
            word: word.to_string(),
            part_of_speech: part_of_speech.map(str::to_string),
            frequency: 1.0,
            phonetic_us: None,
            phonetic_uk: None,
            meanings: meanings
                .iter()
                .map(|(pos, meaning_cn)| MeaningZh {
                    pos: (*pos).to_string(),
                    meaning_cn: (*meaning_cn).to_string(),
                    meaning_en: None,
                })
                .collect(),
            examples: example_translation
                .map(|translation| EntryExample {
                    sentence_en: format!("example for {word}"),
                    sentence_cn: translation.to_string(),
                })
                .into_iter()
                .collect(),
            cn_choice_distractors: Vec::new(),
            en_choice_distractors: Vec::new(),
        }
    }

    fn build_word_with_examples(
        source_id: &str,
        word: &str,
        part_of_speech: Option<&str>,
        meanings: &[(&str, &str)],
        examples: &[(&str, &str)],
    ) -> WordForQuestion {
        let mut word = build_word(source_id, word, part_of_speech, meanings, None);
        word.examples = examples
            .iter()
            .map(|(sentence_en, sentence_cn)| EntryExample {
                sentence_en: (*sentence_en).to_string(),
                sentence_cn: (*sentence_cn).to_string(),
            })
            .collect();
        word
    }

    #[test]
    fn en_to_cn_choices_prefer_same_part_of_speech_distractors() {
        let target = build_word(
            "target",
            "mutter",
            Some("v."),
            &[("v.", "咕哝；抱怨")],
            None,
        );
        let distractors = vec![
            build_word(
                "d1",
                "known",
                Some("adj."),
                &[("adj.", "认知的；认识的")],
                None,
            ),
            build_word("d2", "remark", Some("v."), &[("v.", "评论；说起")], None),
            build_word("d3", "grumble", Some("v."), &[("v.", "抱怨；发牢骚")], None),
            build_word("d4", "chant", Some("v."), &[("v.", "反复呼喊；吟唱")], None),
        ];

        let question = QuestionBuilder::build_single_question(
            &target,
            &QuestionType::EnToCnChoice,
            &distractors,
            "sess",
            0,
            1,
        );

        let texts: Vec<String> = question
            .choices
            .expect("choices should exist")
            .into_iter()
            .map(|choice| choice.text)
            .collect();
        assert!(texts.contains(&"咕哝；抱怨".to_string()));
        assert!(!texts.contains(&"认知的；认识的".to_string()));
        let allowed = ["评论；说起", "抱怨；发牢骚", "反复呼喊；吟唱"];
        for text in texts.iter().filter(|text| *text != "咕哝；抱怨") {
            assert!(allowed.contains(&text.as_str()));
        }
    }

    #[test]
    fn en_to_cn_choices_rank_same_pos_distractors_by_meaning_similarity() {
        let target = build_word(
            "foundation",
            "foundation",
            Some("n"),
            &[("n", "基础，根本；建立，创立；地基；基金，基金会")],
            None,
        );
        let distractors = vec![
            build_word(
                "scratch",
                "scratch",
                Some("n"),
                &[("n", "抓，搔；抓痕；起跑线")],
                None,
            ),
            build_word(
                "investment",
                "investment",
                Some("n"),
                &[("n", "投资，投资额；基金")],
                None,
            ),
            build_word(
                "basis",
                "basis",
                Some("n"),
                &[("n", "基础，基准；根据")],
                None,
            ),
            build_word(
                "establishment",
                "establishment",
                Some("n"),
                &[("n", "建立，创立；机构")],
                None,
            ),
            build_word(
                "narrative",
                "narrative",
                Some("n"),
                &[("n", "叙述；记叙文")],
                None,
            ),
        ];

        let question = QuestionBuilder::build_single_question(
            &target,
            &QuestionType::EnToCnChoice,
            &distractors,
            "sess",
            0,
            1,
        );

        let texts: Vec<String> = question
            .choices
            .expect("choices should exist")
            .into_iter()
            .map(|choice| choice.text)
            .collect();
        assert!(texts.contains(&"基础，基准；根据".to_string()));
        assert!(texts.contains(&"投资，投资额；基金".to_string()));
        assert!(texts.contains(&"建立，创立；机构".to_string()));
        assert!(!texts.contains(&"抓，搔；抓痕；起跑线".to_string()));
        assert!(!texts.contains(&"叙述；记叙文".to_string()));
    }

    #[test]
    fn session_choice_distractors_avoid_repeating_across_words() {
        let words = vec![
            build_word("w1", "basis", Some("n"), &[("n", "基础，基准")], None),
            build_word("w2", "fund", Some("n"), &[("n", "基金，资金")], None),
            build_word("w3", "building", Some("n"), &[("n", "建筑物，楼房")], None),
        ];
        let distractors = vec![
            build_word("d1", "foundation", Some("n"), &[("n", "基础，地基")], None),
            build_word("d2", "capital", Some("n"), &[("n", "资金，资本")], None),
            build_word("d3", "structure", Some("n"), &[("n", "建筑物，结构")], None),
            build_word("d4", "support", Some("n"), &[("n", "支撑，支持")], None),
            build_word("d5", "grant", Some("n"), &[("n", "补助金，拨款")], None),
            build_word("d6", "house", Some("n"), &[("n", "房屋，住宅")], None),
            build_word("d7", "principle", Some("n"), &[("n", "原则，根本")], None),
            build_word("d8", "budget", Some("n"), &[("n", "预算，经费")], None),
            build_word("d9", "tower", Some("n"), &[("n", "塔，塔楼")], None),
        ];

        let questions = QuestionBuilder::build_session_questions(
            &SessionMode::MixedTest,
            &words,
            &distractors,
            "sess_no_repeat",
            &[],
        );

        let mut seen = HashSet::new();
        for question in questions
            .iter()
            .filter(|question| question.question_type == QuestionType::EnToCnChoice)
        {
            let correct_label = question.correct_choice_label.as_deref().unwrap_or("");
            for choice in question.choices.as_ref().expect("choices should exist") {
                if choice.label != correct_label {
                    assert!(
                        seen.insert(choice.text.clone()),
                        "distractor repeated in one session: {}",
                        choice.text
                    );
                }
            }
        }
    }

    #[test]
    fn session_choice_distractors_vary_across_large_candidate_pool() {
        let words: Vec<WordForQuestion> = (0..32)
            .map(|index| {
                build_word(
                    &format!("w{index}"),
                    &format!("target{index}"),
                    Some("n."),
                    &[("n.", "core shared meaning")],
                    None,
                )
            })
            .collect();
        let distractors: Vec<WordForQuestion> = (0..96)
            .map(|index| {
                build_word(
                    &format!("d{index}"),
                    &format!("distractor{index}"),
                    Some("n."),
                    &[("n.", &format!("core shared nearby meaning {index}"))],
                    None,
                )
            })
            .collect();

        let questions = QuestionBuilder::build_session_questions(
            &SessionMode::MixedTest,
            &words,
            &distractors,
            "sess_variety",
            &[
                QuestionTypeWeight {
                    question_type: QuestionType::EnToCnChoice,
                    weight: 50,
                },
                QuestionTypeWeight {
                    question_type: QuestionType::ExampleToCnChoice,
                    weight: 50,
                },
            ],
        );

        let mut choice_sets = HashSet::new();
        for question in questions.iter().filter(|question| {
            matches!(
                question.question_type,
                QuestionType::EnToCnChoice | QuestionType::ExampleToCnChoice
            )
        }) {
            let correct_label = question.correct_choice_label.as_deref().unwrap_or("");
            let mut distractor_texts = question
                .choices
                .as_ref()
                .expect("choices should exist")
                .iter()
                .filter(|choice| choice.label != correct_label)
                .map(|choice| choice.text.clone())
                .collect::<Vec<_>>();
            distractor_texts.sort();
            choice_sets.insert(distractor_texts.join("|"));
        }

        assert!(
            choice_sets.len() >= 20,
            "choice distractors should vary across a large candidate pool, got {} sets",
            choice_sets.len()
        );
    }

    #[test]
    fn en_to_cn_choices_use_precomputed_wordbook_distractors_without_global_pool() {
        let mut target = build_word(
            "target",
            "assimilate",
            Some("v."),
            &[("v.", "经消化而吸收，同化")],
            None,
        );
        target.cn_choice_distractors = vec![
            "吸收，接纳".to_string(),
            "适应，改编".to_string(),
            "结合，融合".to_string(),
            "吞并，合并".to_string(),
            "吸引，引起".to_string(),
            "消化，领会".to_string(),
            "转化，改变".to_string(),
        ];

        let question = QuestionBuilder::build_single_question(
            &target,
            &QuestionType::EnToCnChoice,
            &[],
            "sess_precomputed_cn",
            0,
            1,
        );

        let correct_label = question
            .correct_choice_label
            .as_deref()
            .expect("correct label should exist");
        let choices = question.choices.expect("choices should exist");
        assert_eq!(choices.len(), 4);
        assert!(choices.iter().any(|choice| {
            choice.label == correct_label && choice.text == "经消化而吸收，同化"
        }));
        let distractor_count = choices
            .iter()
            .filter(|choice| choice.label != correct_label)
            .filter(|choice| target.cn_choice_distractors.contains(&choice.text))
            .count();
        assert_eq!(distractor_count, 3);
    }

    #[test]
    fn cn_to_en_choices_use_precomputed_wordbook_distractors_without_global_pool() {
        let mut target = build_word(
            "target",
            "explosive",
            Some("adj."),
            &[("adj.", "爆炸的，爆发的")],
            None,
        );
        target.en_choice_distractors = vec![
            "violent".to_string(),
            "intense".to_string(),
            "sudden".to_string(),
            "rapid".to_string(),
            "fierce".to_string(),
            "dramatic".to_string(),
            "unstable".to_string(),
        ];

        let question = QuestionBuilder::build_single_question(
            &target,
            &QuestionType::CnToEnChoice,
            &[],
            "sess_precomputed_en",
            0,
            1,
        );

        let correct_label = question
            .correct_choice_label
            .as_deref()
            .expect("correct label should exist");
        let choices = question.choices.expect("choices should exist");
        assert_eq!(choices.len(), 4);
        assert!(choices
            .iter()
            .any(|choice| choice.label == correct_label && choice.text == "explosive"));
        let distractor_count = choices
            .iter()
            .filter(|choice| choice.label != correct_label)
            .filter(|choice| target.en_choice_distractors.contains(&choice.text))
            .count();
        assert_eq!(distractor_count, 3);
    }
    #[test]
    fn cn_to_en_choices_prefer_same_part_of_speech_distractors() {
        let target = build_word("target", "adapt", Some("v."), &[("v.", "adapt")], None);
        let distractors = vec![
            build_word("n1", "method", Some("n."), &[("n.", "method")], None),
            build_word("v1", "clarify", Some("v."), &[("v.", "clarify")], None),
            build_word("v2", "retain", Some("v."), &[("v.", "retain")], None),
            build_word("v3", "modify", Some("v."), &[("v.", "modify")], None),
        ];

        let question = QuestionBuilder::build_single_question(
            &target,
            &QuestionType::CnToEnChoice,
            &distractors,
            "sess",
            0,
            1,
        );

        let texts: Vec<String> = question
            .choices
            .expect("choices should exist")
            .into_iter()
            .map(|choice| choice.text)
            .collect();
        assert!(texts.contains(&"adapt".to_string()));
        assert!(!texts.contains(&"method".to_string()));
        for text in texts.iter().filter(|text| *text != "adapt") {
            assert!(["clarify", "retain", "modify"].contains(&text.as_str()));
        }
    }

    #[test]
    fn cn_choice_text_strips_embedded_option_labels_from_source_meanings() {
        let target = build_word(
            "target",
            "abrupt",
            Some("adj."),
            &[("adj.", "sudden; unexpected")],
            None,
        );
        let distractors = vec![
            build_word(
                "d1",
                "circular",
                Some("adj."),
                &[("adj.", "round; circular A; cyclic C；looping <\u{fffd}")],
                None,
            ),
            build_word(
                "d2",
                "capable",
                Some("adj."),
                &[("adj.", "capable; competent")],
                None,
            ),
            build_word(
                "d3",
                "narrative",
                Some("adj."),
                &[("adj.", "narrative; story-like")],
                None,
            ),
        ];

        let question = QuestionBuilder::build_single_question(
            &target,
            &QuestionType::EnToCnChoice,
            &distractors,
            "sess",
            0,
            1,
        );

        let texts: Vec<String> = question
            .choices
            .expect("choices should exist")
            .into_iter()
            .map(|choice| choice.text)
            .collect();
        assert!(!texts.iter().any(|text| text.contains(" A;")));
        assert!(!texts.iter().any(|text| text.contains(" C；")));
        assert!(!texts.iter().any(|text| text.contains('<')));
        assert!(!texts.iter().any(|text| text.contains('\u{fffd}')));
        assert!(texts
            .iter()
            .any(|text| text == "round；circular；cyclic；looping"));
    }

    #[test]
    fn cn_choice_distractors_skip_placeholder_slash_meanings() {
        let target = build_word(
            "target",
            "politician",
            Some("n."),
            &[("n.", "politician meaning")],
            None,
        );
        let distractors = vec![
            build_word("d1", "slash", Some("n."), &[("n.", "/")], None),
            build_word("d2", "staff", Some("n."), &[("n.", "staff meaning")], None),
            build_word(
                "d3",
                "invite",
                Some("n."),
                &[("n.", "invite meaning")],
                None,
            ),
            build_word(
                "d4",
                "public",
                Some("n."),
                &[("n.", "public meaning")],
                None,
            ),
        ];

        let question = QuestionBuilder::build_single_question(
            &target,
            &QuestionType::ExampleToCnChoice,
            &distractors,
            "sess",
            0,
            1,
        );

        let texts = question
            .choices
            .expect("choices should exist")
            .into_iter()
            .map(|choice| choice.text)
            .collect::<Vec<_>>();
        assert!(!texts.iter().any(|text| text.trim() == "/"));
        assert!(!texts.iter().any(|text| text.trim().is_empty()));
    }

    #[test]
    fn cn_to_en_prompt_strips_embedded_option_labels() {
        let target = build_word(
            "target",
            "surgeon",
            Some("n."),
            &[("n.", "doctor B; surgeon C：operator")],
            None,
        );

        let question = QuestionBuilder::build_single_question(
            &target,
            &QuestionType::CnToEnChoice,
            &[],
            "sess",
            0,
            1,
        );

        assert_eq!(question.prompt, "doctor；surgeon；operator");
    }

    #[test]
    fn example_no_translation_choice_hides_translation_payload() {
        let target = build_word(
            "target",
            "adapt",
            Some("v."),
            &[("v.", "adapt meaning")],
            Some("adapt translation"),
        );

        let question = QuestionBuilder::build_single_question(
            &target,
            &QuestionType::ExampleToCnChoiceNoTranslation,
            &[],
            "sess",
            0,
            1,
        );

        assert_eq!(
            question.question_type,
            QuestionType::ExampleToCnChoiceNoTranslation
        );
        assert_eq!(
            question.example_sentence.as_deref(),
            Some("example for adapt")
        );
        assert_eq!(question.example_translation, None);
        assert!(question.choices.is_some());
    }

    #[test]
    fn example_choice_does_not_use_example_from_different_meaning() {
        let target = build_word_with_examples(
            "target",
            "pop",
            Some("adj"),
            &[("adj", "流行的，通俗的"), ("v", "突然出现；冒出")],
            &[(
                "All at once an idea popped into her head.",
                "她脑子里突然冒出一个念头。",
            )],
        );
        let distractors = vec![
            build_word(
                "d1",
                "artistic",
                Some("adj"),
                &[("adj", "艺术的，美术的")],
                None,
            ),
            build_word(
                "d2",
                "notable",
                Some("adj"),
                &[("adj", "值得注意的，显著的")],
                None,
            ),
            build_word(
                "d3",
                "splendid",
                Some("adj"),
                &[("adj", "华丽的，极好的")],
                None,
            ),
        ];

        let question = QuestionBuilder::build_single_question(
            &target,
            &QuestionType::ExampleToCnChoice,
            &distractors,
            "sess",
            0,
            1,
        );

        assert_eq!(question.prompt, "pop");
        assert_eq!(question.example_sentence, None);
        assert_eq!(question.example_translation, None);
        assert_eq!(
            question.accepted_meanings,
            vec!["流行的，通俗的".to_string()]
        );
    }

    #[test]
    fn word_skeleton_input_prompts_with_hidden_middle_letters() {
        let target = build_word(
            "target",
            "function",
            Some("n."),
            &[("n.", "function meaning")],
            None,
        );

        let question = QuestionBuilder::build_single_question(
            &target,
            &QuestionType::WordSkeletonInput,
            &[],
            "sess",
            0,
            1,
        );

        assert_eq!(question.question_type, QuestionType::WordSkeletonInput);
        assert_eq!(question.prompt, "f__ction");
        assert_eq!(question.accepted_meanings, vec!["un".to_string()]);
        assert_eq!(
            question.example_translation.as_deref(),
            Some("function meaning")
        );
        assert!(question.choices.is_none());
    }

    #[test]
    fn word_skeleton_input_accepts_only_missing_letters() {
        let target = build_word(
            "target",
            "fridge",
            Some("n."),
            &[("n.", "fridge meaning")],
            None,
        );

        let question = QuestionBuilder::build_single_question(
            &target,
            &QuestionType::WordSkeletonInput,
            &[],
            "sess",
            0,
            1,
        );

        assert_eq!(question.prompt, "f__dge");
        assert_eq!(question.accepted_meanings, vec!["ri".to_string()]);
    }

    #[test]
    fn word_skeleton_input_masks_short_words() {
        let target = build_word(
            "target",
            "ruby",
            Some("n."),
            &[("n.", "ruby meaning")],
            None,
        );

        let question = QuestionBuilder::build_single_question(
            &target,
            &QuestionType::WordSkeletonInput,
            &[],
            "sess",
            0,
            1,
        );

        assert_eq!(question.prompt, "r__y");
        assert_eq!(question.accepted_meanings, vec!["ub".to_string()]);
        assert_ne!(question.prompt, question.word);
    }

    #[test]
    fn example_question_hides_synthetic_meaning_explanation_translation() {
        let target = build_word_with_examples(
            "target",
            "zoom",
            Some("vi"),
            &[("vi", "\u{6025}\u{901f}\u{79fb}\u{52a8}\u{ff1b}\u{6025}\u{5347}\u{ff0c}\u{731b}\u{6da8}")],
            &[(
                "They need to zoom the idea in a clear way.",
                "zoom \u{5728}\u{6b64}\u{5904}\u{8868}\u{793a}\u{ff1a}\u{6025}\u{901f}\u{79fb}\u{52a8}\u{ff1b}\u{6025}\u{5347}\u{ff0c}\u{731b}\u{6da8}",
            )],
        );

        let question = QuestionBuilder::build_single_question(
            &target,
            &QuestionType::ExampleToCnChoice,
            &[],
            "sess",
            0,
            1,
        );

        assert_eq!(
            question.example_sentence.as_deref(),
            Some("They need to zoom the idea in a clear way.")
        );
        assert_eq!(question.example_translation, None);
    }

    #[test]
    fn new_word_ignores_personalized_weights_and_keeps_fixed_type_rounds() {
        let words = vec![
            build_word("w1", "function", Some("n."), &[("n.", "meaning 1")], None),
            build_word("w2", "balance", Some("n."), &[("n.", "meaning 2")], None),
            build_word("w3", "conduct", Some("v."), &[("v.", "meaning 3")], None),
        ];
        let weights = vec![QuestionTypeWeight {
            question_type: QuestionType::WordSkeletonInput,
            weight: 100,
        }];

        let questions = QuestionBuilder::build_session_questions(
            &SessionMode::NewWord,
            &words,
            &words,
            "sess_personalized_new",
            &weights,
        );

        assert_eq!(questions.len(), words.len() * 4);
        assert!(questions[0..words.len()]
            .iter()
            .all(|question| question.question_type == QuestionType::ExampleToCnChoice));
        assert!(questions[words.len()..words.len() * 2]
            .iter()
            .all(|question| question.question_type == QuestionType::EnToCnChoice));
        assert!(questions[words.len() * 2..words.len() * 3]
            .iter()
            .all(|question| question.question_type == QuestionType::CnToEnChoice));
        assert!(questions[words.len() * 3..words.len() * 4]
            .iter()
            .all(|question| question.question_type == QuestionType::EnToCnInput));
    }

    #[test]
    fn new_word_choice_labels_are_owned_by_generated_choice_position() {
        let target = build_word(
            "target",
            "target",
            Some("n."),
            &[("n.", "target meaning")],
            Some("target meaning example"),
        );
        let distractors = vec![
            build_word("d1", "alpha", Some("n."), &[("n.", "alpha meaning")], None),
            build_word("d2", "bravo", Some("n."), &[("n.", "bravo meaning")], None),
            build_word(
                "d3",
                "charlie",
                Some("n."),
                &[("n.", "charlie meaning")],
                None,
            ),
            build_word("d4", "delta", Some("n."), &[("n.", "delta meaning")], None),
            build_word("d5", "echo", Some("n."), &[("n.", "echo meaning")], None),
            build_word(
                "d6",
                "foxtrot",
                Some("n."),
                &[("n.", "foxtrot meaning")],
                None,
            ),
        ];

        let questions = QuestionBuilder::build_session_questions(
            &SessionMode::NewWord,
            &[target],
            &distractors,
            "sess_choice_labels",
            &[],
        );
        let choice_labels = questions
            .iter()
            .filter(|question| question.question_type.is_choice_type())
            .map(|question| {
                let label = question
                    .correct_choice_label
                    .as_deref()
                    .expect("choice question has correct label");
                let choices = question.choices.as_ref().expect("choices");
                assert!(
                    choices.iter().any(|choice| choice.label == label),
                    "correct label {label} must point at one rendered choice"
                );
                label.to_string()
            })
            .collect::<Vec<_>>();

        assert_eq!(choice_labels.len(), 3);
        assert!(choice_labels
            .windows(2)
            .all(|labels| labels[0] != labels[1]));
    }

    #[test]
    fn mixed_test_choice_labels_do_not_repeat_across_interleaved_inputs() {
        let words = (0..9)
            .map(|index| {
                build_word(
                    &format!("w{index}"),
                    &format!("target{index}"),
                    Some("n."),
                    &[("n.", &format!("target meaning {index}"))],
                    None,
                )
            })
            .collect::<Vec<_>>();
        let distractors = (0..24)
            .map(|index| {
                build_word(
                    &format!("d{index}"),
                    &format!("distractor{index}"),
                    Some("n."),
                    &[("n.", &format!("distractor meaning {index}"))],
                    None,
                )
            })
            .collect::<Vec<_>>();

        let questions = QuestionBuilder::build_session_questions(
            &SessionMode::MixedTest,
            &words,
            &distractors,
            "sess_choice_position_variety",
            &[],
        );

        let choice_labels = questions
            .iter()
            .filter_map(|question| question.correct_choice_label.as_deref())
            .collect::<Vec<_>>();

        assert!(
            choice_labels.len() >= 2,
            "fixture should include multiple choice questions"
        );
        assert!(
            choice_labels
                .windows(2)
                .all(|labels| labels[0] != labels[1]),
            "consecutive choice questions reused correct labels: {choice_labels:?}"
        );
    }

    #[test]
    fn root_affix_ignores_personalized_weights() {
        let words = vec![build_word(
            "root",
            "re",
            Some("root"),
            &[("root", "again")],
            None,
        )];
        let weights = vec![QuestionTypeWeight {
            question_type: QuestionType::WordSkeletonInput,
            weight: 100,
        }];

        let questions = QuestionBuilder::build_session_questions(
            &SessionMode::RootAffix,
            &words,
            &words,
            "sess_root_fixed",
            &weights,
        );

        assert_eq!(questions.len(), 1);
        assert_eq!(questions[0].question_type, QuestionType::RootToGlossInput);
    }

    #[test]
    fn example_questions_prefer_meanings_matching_displayed_part_of_speech() {
        let rally = build_word(
            "rally",
            "rally",
            Some("v."),
            &[("n.", "集会；公路汽车赛"), ("v.", "集合支持；重新振作")],
            Some("为政党争取支持的努力"),
        );

        let question = QuestionBuilder::build_single_question(
            &rally,
            &QuestionType::ExampleToCnChoice,
            &[],
            "sess",
            0,
            1,
        );

        assert_eq!(
            question.accepted_meanings,
            vec!["集合支持；重新振作".to_string()]
        );
        let correct_label = question
            .correct_choice_label
            .expect("correct choice label should exist");
        let correct_text = question
            .choices
            .expect("choices should exist")
            .into_iter()
            .find(|choice| choice.label == correct_label)
            .map(|choice| choice.text)
            .expect("correct choice should exist");
        assert_eq!(correct_text, "集合支持；重新振作");
    }

    #[test]
    fn mixed_test_uses_default_choice_and_input_question_types() {
        let words: Vec<WordForQuestion> = (0..8)
            .map(|index| {
                build_word(
                    &format!("w{index}"),
                    &format!("word{index}"),
                    Some("v."),
                    &[("v.", &format!("释义{index}"))],
                    None,
                )
            })
            .collect();

        let questions = QuestionBuilder::build_session_questions(
            &SessionMode::MixedTest,
            &words,
            &words,
            "sess",
            &[],
        );

        let en_to_cn_choice = questions
            .iter()
            .filter(|question| question.question_type == QuestionType::EnToCnChoice)
            .count();
        let en_to_cn_input = questions
            .iter()
            .filter(|question| question.question_type == QuestionType::EnToCnInput)
            .count();
        let cn_to_en_choice = questions
            .iter()
            .filter(|question| question.question_type == QuestionType::CnToEnChoice)
            .count();

        assert!(en_to_cn_choice > 0);
        assert!(en_to_cn_input > 0);
        assert!(cn_to_en_choice > 0);
        assert_eq!(
            en_to_cn_choice + en_to_cn_input + cn_to_en_choice,
            questions.len()
        );
    }
}
