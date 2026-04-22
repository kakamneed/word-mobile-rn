//! Question builder for study sessions.

use std::collections::HashSet;

use word_storage_core::models::{
    ChoiceOption, EntryExample, MeaningZh, QuestionType, SessionMode, StudyQuestion,
};

use crate::session_definition::SessionDefinition;

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
}

/// Builds question sets from word entries.
pub struct QuestionBuilder;

impl QuestionBuilder {
    /// Generate the full question set for a list of words in a given session mode.
    pub fn build_session_questions(
        mode: &SessionMode,
        words: &[WordForQuestion],
        distractors: &[WordForQuestion],
        session_id: &str,
    ) -> Vec<StudyQuestion> {
        let definition = SessionDefinition::for_mode(mode.clone());

        if matches!(mode, SessionMode::RootAffix) {
            Self::build_root_affix_questions(words, session_id)
        } else if definition.rules.loops_all_types_per_word {
            Self::build_loop_questions(words, distractors, session_id)
        } else if definition.rules.from_wrong_pool {
            Self::build_wrong_word_questions(words, distractors, session_id)
        } else {
            Self::build_mixed_test_questions(words, distractors, session_id)
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

        for (type_round, qt) in types.iter().enumerate() {
            for word in Self::ordered_words_for_round(words, session_id, type_round) {
                let question = Self::build_single_question(
                    word,
                    qt,
                    distractors,
                    session_id,
                    question_index,
                    total_questions,
                );
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

    fn build_wrong_word_questions(
        wrong_words: &[WordForQuestion],
        distractors: &[WordForQuestion],
        session_id: &str,
    ) -> Vec<StudyQuestion> {
        let types = vec![
            QuestionType::ExampleToCnChoice,
            QuestionType::EnToCnChoice,
            QuestionType::EnToCnInput,
            QuestionType::EnToCnChoice,
            QuestionType::EnToCnInput,
            QuestionType::EnToCnChoice,
            QuestionType::CnToEnChoice,
            QuestionType::EnToCnInput,
        ];
        let total_questions = wrong_words.len() as u32;
        let mut questions = Vec::with_capacity(total_questions as usize);
        let mut question_index = 0u32;

        for word in wrong_words {
            let qt = &types[(question_index as usize) % types.len()];
            let question = Self::build_single_question(
                word,
                qt,
                distractors,
                session_id,
                question_index,
                total_questions,
            );
            questions.push(question);
            question_index += 1;
        }

        questions
    }

    fn build_mixed_test_questions(
        words: &[WordForQuestion],
        distractors: &[WordForQuestion],
        session_id: &str,
    ) -> Vec<StudyQuestion> {
        let types = vec![
            QuestionType::ExampleToCnChoice,
            QuestionType::EnToCnChoice,
            QuestionType::EnToCnInput,
            QuestionType::EnToCnChoice,
            QuestionType::EnToCnInput,
            QuestionType::EnToCnChoice,
            QuestionType::CnToEnChoice,
            QuestionType::EnToCnInput,
        ];
        let total_questions = words.len() as u32;
        let mut questions = Vec::with_capacity(total_questions as usize);
        let mut question_index = 0u32;

        for word in words {
            let qt = &types[(question_index as usize) % types.len()];
            let question = Self::build_single_question(
                word,
                qt,
                distractors,
                session_id,
                question_index,
                total_questions,
            );
            questions.push(question);
            question_index += 1;
        }

        questions
    }

    fn build_single_question(
        word: &WordForQuestion,
        question_type: &QuestionType,
        distractors: &[WordForQuestion],
        session_id: &str,
        question_index: u32,
        total_questions: u32,
    ) -> StudyQuestion {
        let accepted_meanings: Vec<String> =
            word.meanings.iter().map(|m| m.meaning_cn.clone()).collect();

        let question_id = format!("{}_{}", session_id, question_index);

        let example = word.examples.first();
        let example_sentence = example.map(|e| e.sentence_en.clone());
        let example_translation = example.map(|e| e.sentence_cn.clone());

        match question_type {
            QuestionType::EnToCnChoice => {
                let accepted_meanings = Self::choice_meanings_for_word(word);
                let primary_meaning =
                    Self::primary_meaning_for_question(word, example_translation.as_deref());
                let (choices, correct_label) = Self::build_cn_choices(
                    &primary_meaning,
                    distractors,
                    &accepted_meanings,
                    word.part_of_speech.as_deref(),
                    question_index,
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
            QuestionType::ExampleToCnChoice => {
                let accepted_meanings = Self::choice_meanings_for_word(word);
                let prompt = example
                    .map(|e| e.sentence_en.clone())
                    .unwrap_or_else(|| word.word.clone());
                let contextual_meaning =
                    Self::primary_meaning_for_question(word, example_translation.as_deref());
                let (choices, correct_label) = Self::build_cn_choices(
                    &contextual_meaning,
                    distractors,
                    &accepted_meanings,
                    word.part_of_speech.as_deref(),
                    question_index,
                );
                StudyQuestion {
                    question_id,
                    question_type: QuestionType::ExampleToCnChoice,
                    entry_source_id: word.source_id.clone(),
                    word: word.word.clone(),
                    part_of_speech: word.part_of_speech.clone(),
                    phonetic_us: word.phonetic_us.clone(),
                    phonetic_uk: word.phonetic_uk.clone(),
                    prompt,
                    accepted_meanings,
                    example_sentence,
                    example_translation,
                    choices: Some(choices),
                    correct_choice_label: Some(correct_label),
                    question_index,
                    total_questions,
                }
            }
            QuestionType::CnToEnChoice => {
                let prompt = accepted_meanings.first().cloned().unwrap_or_default();
                let (choices, correct_label) =
                    Self::build_en_choices(word, distractors, question_index);
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
        correct_text: &str,
        distractors: &[WordForQuestion],
        accepted_meanings: &[String],
        target_pos: Option<&str>,
        question_index: u32,
    ) -> (Vec<ChoiceOption>, String) {
        let mut excluded_meanings: HashSet<String> = accepted_meanings.iter().cloned().collect();
        let mut distractor_texts = if target_pos.is_some() {
            Self::collect_distractor_texts(
                distractors,
                question_index,
                correct_text,
                |candidate| Self::choice_meanings_for_distractor(candidate, target_pos),
                |text| !excluded_meanings.contains(text),
            )
        } else {
            Vec::new()
        };

        for text in &distractor_texts {
            excluded_meanings.insert(text.clone());
        }

        if distractor_texts.len() < 3 {
            let fallback_texts = Self::collect_distractor_texts(
                distractors,
                question_index,
                correct_text,
                Self::all_meanings_for_word,
                |text| !excluded_meanings.contains(text),
            );
            for text in fallback_texts {
                if distractor_texts.len() >= 3 {
                    break;
                }
                excluded_meanings.insert(text.clone());
                distractor_texts.push(text);
            }
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

        let correct_pos = (question_index as usize) % choice_count.max(1);
        options.insert(
            correct_pos,
            ChoiceOption {
                text: correct_text.to_string(),
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
        question_index: u32,
    ) -> (Vec<ChoiceOption>, String) {
        let correct_text = word.word.clone();
        let labels = ["A", "B", "C", "D"];
        let distractor_texts = Self::collect_distractor_texts(
            distractors,
            question_index,
            &word.source_id,
            |candidate| vec![candidate.word.clone()],
            |text| text != &correct_text,
        );

        let choice_count = distractor_texts.len() + 1;
        let mut options: Vec<ChoiceOption> = distractor_texts
            .into_iter()
            .map(|text| ChoiceOption {
                text,
                label: String::new(),
            })
            .collect();

        let correct_pos = ((question_index + 1) as usize) % choice_count.max(1);
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

    fn collect_distractor_texts<FMap, FFilter>(
        distractors: &[WordForQuestion],
        question_index: u32,
        source_id: &str,
        values_for_word: FMap,
        should_keep: FFilter,
    ) -> Vec<String>
    where
        FMap: Fn(&WordForQuestion) -> Vec<String>,
        FFilter: Fn(&String) -> bool,
    {
        if distractors.is_empty() {
            return Vec::new();
        }

        let start_offset = (Self::stable_seed(source_id).wrapping_add(question_index as usize))
            % distractors.len();
        let mut collected = Vec::new();
        let mut seen = HashSet::new();

        for step in 0..distractors.len() {
            let index = (start_offset + step) % distractors.len();
            for text in values_for_word(&distractors[index]) {
                if !should_keep(&text) || !seen.insert(text.clone()) {
                    continue;
                }
                collected.push(text);
                if collected.len() >= 3 {
                    return collected;
                }
            }
        }

        collected
    }

    fn stable_seed(text: &str) -> usize {
        text.bytes().fold(0usize, |acc, byte| {
            acc.wrapping_mul(33).wrapping_add(byte as usize)
        })
    }

    fn choice_meanings_for_word(word: &WordForQuestion) -> Vec<String> {
        let preferred = Self::meanings_matching_target_pos(word, word.part_of_speech.as_deref());
        if preferred.is_empty() {
            word.meanings
                .iter()
                .map(|meaning| meaning.meaning_cn.clone())
                .collect()
        } else {
            preferred
                .iter()
                .map(|meaning| meaning.meaning_cn.clone())
                .collect()
        }
    }

    fn choice_meanings_for_distractor(
        word: &WordForQuestion,
        target_pos: Option<&str>,
    ) -> Vec<String> {
        Self::meanings_matching_target_pos(word, target_pos)
            .iter()
            .map(|meaning| meaning.meaning_cn.clone())
            .collect()
    }

    fn all_meanings_for_word(word: &WordForQuestion) -> Vec<String> {
        word.meanings
            .iter()
            .map(|meaning| meaning.meaning_cn.clone())
            .collect()
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

#[cfg(test)]
mod tests {
    use super::{QuestionBuilder, WordForQuestion};
    use word_storage_core::models::{EntryExample, MeaningZh, QuestionType, SessionMode};

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
        }
    }

    #[test]
    fn en_to_cn_choices_prefer_same_part_of_speech_distractors() {
        let target = build_word("target", "mutter", Some("v."), &[("v.", "咕哝；抱怨")], None);
        let distractors = vec![
            build_word("d1", "known", Some("adj."), &[("adj.", "认知的；认识的")], None),
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

        assert_eq!(question.accepted_meanings, vec!["集合支持；重新振作".to_string()]);
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
    fn mixed_test_prefers_en_to_cn_choice_and_input_question_types() {
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

        let questions =
            QuestionBuilder::build_session_questions(&SessionMode::MixedTest, &words, &words, "sess");

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

        assert!(en_to_cn_choice >= 3);
        assert!(en_to_cn_input >= 3);
        assert!(cn_to_en_choice <= 1);
    }
}
