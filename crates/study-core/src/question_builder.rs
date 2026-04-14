//! Question builder for study sessions.

use word_storage_core::models::{ChoiceOption, EntryExample, MeaningZh, QuestionType, SessionMode, StudyQuestion};

use crate::session_definition::SessionDefinition;

/// A word entry prepared for question generation.
#[derive(Debug, Clone)]
pub struct WordForQuestion {
    pub source_id: String,
    pub word: String,
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

        if definition.rules.loops_all_types_per_word {
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

        for word in words {
            for qt in &types {
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

    fn build_wrong_word_questions(
        wrong_words: &[WordForQuestion],
        distractors: &[WordForQuestion],
        session_id: &str,
    ) -> Vec<StudyQuestion> {
        let types = vec![
            QuestionType::EnToCnChoice,
            QuestionType::CnToEnChoice,
            QuestionType::EnToCnInput,
        ];
        let total_questions = (wrong_words.len() as u32) * (types.len() as u32);
        let mut questions = Vec::with_capacity(total_questions as usize);
        let mut question_index = 0u32;

        for word in wrong_words {
            for qt in &types {
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

    fn build_mixed_test_questions(
        words: &[WordForQuestion],
        distractors: &[WordForQuestion],
        session_id: &str,
    ) -> Vec<StudyQuestion> {
        let types = vec![
            QuestionType::EnToCnChoice,
            QuestionType::CnToEnChoice,
            QuestionType::EnToCnInput,
        ];
        let total_questions = (words.len() as u32) * (types.len() as u32);
        let mut questions = Vec::with_capacity(total_questions as usize);
        let mut question_index = 0u32;

        for word in words {
            for qt in &types {
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

    fn build_single_question(
        word: &WordForQuestion,
        question_type: &QuestionType,
        distractors: &[WordForQuestion],
        session_id: &str,
        question_index: u32,
        total_questions: u32,
    ) -> StudyQuestion {
        let accepted_meanings: Vec<String> = word
            .meanings
            .iter()
            .map(|m| m.meaning_cn.clone())
            .collect();

        let question_id = format!("{}_{}", session_id, question_index);

        let example = word.examples.first();
        let example_sentence = example.map(|e| e.sentence_en.clone());
        let example_translation = example.map(|e| e.sentence_cn.clone());

        match question_type {
            QuestionType::EnToCnChoice => {
                let (choices, correct_label) =
                    Self::build_cn_choices(word, distractors, &accepted_meanings, question_index);
                StudyQuestion {
                    question_id,
                    question_type: QuestionType::EnToCnChoice,
                    entry_source_id: word.source_id.clone(),
                    word: word.word.clone(),
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
                let prompt = example
                    .map(|e| e.sentence_en.clone())
                    .unwrap_or_else(|| word.word.clone());
                let (choices, correct_label) =
                    Self::build_cn_choices(word, distractors, &accepted_meanings, question_index);
                StudyQuestion {
                    question_id,
                    question_type: QuestionType::ExampleToCnChoice,
                    entry_source_id: word.source_id.clone(),
                    word: word.word.clone(),
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
                prompt: word.word.clone(),
                accepted_meanings,
                example_sentence: None,
                example_translation: None,
                choices: None,
                correct_choice_label: None,
                question_index,
                total_questions,
            },
        }
    }

    fn build_cn_choices(
        word: &WordForQuestion,
        distractors: &[WordForQuestion],
        accepted_meanings: &[String],
        question_index: u32,
    ) -> (Vec<ChoiceOption>, String) {
        let correct_text = accepted_meanings.first().cloned().unwrap_or_default();

        let mut distractor_texts: Vec<String> = distractors
            .iter()
            .flat_map(|d| d.meanings.iter().map(|m| m.meaning_cn.clone()))
            .filter(|m| m != &correct_text)
            .take(3)
            .collect();

        while distractor_texts.len() < 3 {
            distractor_texts.push(String::from("---"));
        }

        let labels = ["A", "B", "C", "D"];
        let mut options: Vec<ChoiceOption> = distractor_texts
            .into_iter()
            .map(|text| ChoiceOption {
                text,
                label: String::new(),
            })
            .collect();

        let correct_pos = (question_index as usize) % 4;
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

    fn build_en_choices(
        word: &WordForQuestion,
        distractors: &[WordForQuestion],
        question_index: u32,
    ) -> (Vec<ChoiceOption>, String) {
        let correct_text = word.word.clone();
        let labels = ["A", "B", "C", "D"];

        let mut distractor_texts: Vec<String> = distractors
            .iter()
            .map(|d| d.word.clone())
            .filter(|w| w != &correct_text)
            .take(3)
            .collect();

        while distractor_texts.len() < 3 {
            distractor_texts.push(String::from("---"));
        }

        let mut options: Vec<ChoiceOption> = distractor_texts
            .into_iter()
            .map(|text| ChoiceOption {
                text,
                label: String::new(),
            })
            .collect();

        let correct_pos = ((question_index + 1) as usize) % 4;
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
}
