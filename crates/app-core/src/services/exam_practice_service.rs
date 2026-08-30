use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamQuestionCapabilities {
    pub browsable: bool,
    pub answerable: bool,
    pub auto_gradable: bool,
    pub causal_analyzable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExamWordToken {
    pub text: String,
    pub normalized: String,
    pub start_offset: usize,
    pub end_offset: usize,
}

pub fn tokenize_english(text: &str) -> Vec<ExamWordToken> {
    let mut utf16_offset = 0;
    let chars = text
        .char_indices()
        .map(|(byte_offset, character)| {
            let current_utf16_offset = utf16_offset;
            utf16_offset += character.len_utf16();
            (byte_offset, current_utf16_offset, character)
        })
        .collect::<Vec<_>>();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        if !chars[index].2.is_ascii_alphabetic() {
            index += 1;
            continue;
        }
        let start_byte = chars[index].0;
        let start_utf16 = chars[index].1;
        let mut end_byte = start_byte + chars[index].2.len_utf8();
        let mut end_utf16 = start_utf16 + chars[index].2.len_utf16();
        index += 1;
        while index < chars.len() {
            let character = chars[index].2;
            let joins_word = character.is_ascii_alphabetic()
                || (matches!(character, '\'' | '-' | '\u{2019}')
                    && index + 1 < chars.len()
                    && chars[index + 1].2.is_ascii_alphabetic());
            if !joins_word {
                break;
            }
            end_byte = chars[index].0 + character.len_utf8();
            end_utf16 = chars[index].1 + character.len_utf16();
            index += 1;
        }
        let value = &text[start_byte..end_byte];
        tokens.push(ExamWordToken {
            text: value.to_string(),
            normalized: value.to_ascii_lowercase(),
            start_offset: start_utf16,
            end_offset: end_utf16,
        });
    }
    tokens
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamChoice {
    pub label: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamQuestion {
    pub id: String,
    pub number: i64,
    #[serde(default = "default_question_kind")]
    pub kind: String,
    #[serde(default)]
    pub stem: String,
    #[serde(default)]
    pub choices: Vec<ExamChoice>,
    pub answer: Option<String>,
    #[serde(default)]
    pub explanation: String,
    #[serde(default)]
    pub source: Value,
    #[serde(default)]
    pub answer_source: Option<String>,
    #[serde(skip_deserializing, default = "default_capabilities")]
    pub capabilities: ExamQuestionCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamSection {
    pub id: String,
    #[serde(rename = "type", default)]
    pub section_type: String,
    pub title: String,
    #[serde(default)]
    pub instructions: String,
    #[serde(default)]
    pub passage: String,
    #[serde(default)]
    pub paragraph_translations: Vec<String>,
    #[serde(default)]
    pub questions: Vec<ExamQuestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamPaper {
    pub schema_version: i64,
    pub id: String,
    pub exam: String,
    pub title: String,
    pub year: i64,
    pub month: Option<i64>,
    pub set: Option<i64>,
    #[serde(default)]
    pub source: Value,
    pub sections: Vec<ExamSection>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamSectionSummary {
    pub id: String,
    pub section_type: String,
    pub title: String,
    pub question_count: usize,
    pub auto_gradable_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamPaperSummary {
    pub id: String,
    pub title: String,
    pub year: i64,
    pub month: Option<i64>,
    pub set: Option<i64>,
    pub question_count: usize,
    pub answer_bearing_count: usize,
    pub auto_gradable_count: usize,
    pub origin: String,
    pub sections: Vec<ExamSectionSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamCatalogExam {
    pub exam: String,
    pub papers: Vec<ExamPaperSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamCatalog {
    pub schema_version: i64,
    pub exams: Vec<ExamCatalogExam>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExamVocabularyFrequency {
    pub word: String,
    pub occurrence_count: usize,
    pub article_count: usize,
    pub paper_count: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExamManifest {
    schema_version: i64,
    files: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExamDocument {
    schema_version: i64,
    exam: String,
    papers: Vec<ExamPaper>,
}

pub fn load_exam_catalog(asset_dir: &Path) -> Result<ExamCatalog, String> {
    let (manifest, documents) = load_documents(asset_dir)?;
    build_exam_catalog(&manifest, &documents)
}

pub fn load_exam_paper(
    asset_dir: &Path,
    exam: &str,
    paper_id: &str,
) -> Result<Option<ExamPaper>, String> {
    let manifest_path = asset_dir.join("manifest.json");
    let manifest_text = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("Failed to read {}: {error}", manifest_path.display()))?;
    let manifest: ExamManifest = serde_json::from_str(&manifest_text)
        .map_err(|error| format!("Invalid {}: {error}", manifest_path.display()))?;
    let relative = manifest
        .files
        .into_iter()
        .find(|relative| {
            Path::new(relative)
                .file_stem()
                .is_some_and(|stem| stem.to_string_lossy() == exam)
        })
        .ok_or_else(|| format!("Exam asset not found: {exam}"))?;
    let file_name = Path::new(&relative)
        .file_name()
        .ok_or_else(|| format!("Invalid exam asset path: {relative}"))?;
    let file_path = asset_dir.join(file_name);
    let text = fs::read_to_string(&file_path)
        .map_err(|error| format!("Failed to read {}: {error}", file_path.display()))?;
    let value = serde_json::from_str(&text)
        .map_err(|error| format!("Invalid {}: {error}", file_path.display()))?;
    let document = parse_document(&value)?;
    if document.exam != exam {
        return Err(format!(
            "Exam asset mismatch: expected {exam}, found {}",
            document.exam
        ));
    }
    Ok(document
        .papers
        .into_iter()
        .find(|paper| paper.id == paper_id))
}

pub fn load_all_exam_papers(asset_dir: &Path) -> Result<Vec<ExamPaper>, String> {
    let (_, documents) = load_documents(asset_dir)?;
    let mut papers = Vec::new();
    for document in documents.values() {
        papers.extend(parse_document(document)?.papers);
    }
    Ok(papers)
}

pub fn rank_exam_vocabulary(papers: &[ExamPaper], limit: usize) -> Vec<ExamVocabularyFrequency> {
    #[derive(Default)]
    struct Counts {
        occurrences: usize,
        articles: std::collections::BTreeSet<String>,
        papers: std::collections::BTreeSet<String>,
    }
    let stop_words = [
        "the", "and", "that", "this", "with", "from", "have", "has", "was", "were", "are", "for",
        "but", "not", "you", "your", "they", "their", "its", "into", "about", "what", "when",
        "where", "which", "who", "why", "how", "can", "could", "would", "should",
    ];
    let mut counts = BTreeMap::<String, Counts>::new();
    for paper in papers {
        for section in &paper.sections {
            let article_id = format!("{}:{}", paper.id, section.id);
            let question_text = section
                .questions
                .iter()
                .flat_map(|question| {
                    std::iter::once(question.stem.as_str())
                        .chain(question.choices.iter().map(|choice| choice.text.as_str()))
                })
                .collect::<Vec<_>>()
                .join(" ");
            let text = format!("{} {}", section.passage, question_text);
            for token in tokenize_english(&text) {
                if token.normalized.len() < 3 || stop_words.contains(&token.normalized.as_str()) {
                    continue;
                }
                let entry = counts
                    .entry(normalize_exam_word_family(&token.normalized))
                    .or_default();
                entry.occurrences += 1;
                entry.articles.insert(article_id.clone());
                entry.papers.insert(paper.id.clone());
            }
        }
    }
    let mut ranked = counts
        .into_iter()
        .map(|(word, counts)| ExamVocabularyFrequency {
            word,
            occurrence_count: counts.occurrences,
            article_count: counts.articles.len(),
            paper_count: counts.papers.len(),
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .paper_count
            .cmp(&left.paper_count)
            .then_with(|| right.article_count.cmp(&left.article_count))
            .then_with(|| right.occurrence_count.cmp(&left.occurrence_count))
            .then_with(|| left.word.cmp(&right.word))
    });
    ranked.truncate(limit);
    ranked
}

fn normalize_exam_word_family(word: &str) -> String {
    let normalized = word.trim().to_ascii_lowercase();
    let irregular = match normalized.as_str() {
        "children" => Some("child"),
        "people" => Some("person"),
        "men" => Some("man"),
        "women" => Some("woman"),
        "mice" => Some("mouse"),
        "feet" => Some("foot"),
        "teeth" => Some("tooth"),
        "geese" => Some("goose"),
        "went" | "gone" => Some("go"),
        "saw" | "seen" => Some("see"),
        "made" => Some("make"),
        "took" | "taken" => Some("take"),
        "gave" | "given" => Some("give"),
        "found" => Some("find"),
        "thought" => Some("think"),
        "bought" => Some("buy"),
        "brought" => Some("bring"),
        "wrote" | "written" => Some("write"),
        _ => None,
    };
    if let Some(irregular) = irregular {
        return irregular.to_string();
    }
    if normalized.ends_with("ies") && normalized.len() > 3 {
        return format!("{}y", &normalized[..normalized.len() - 3]);
    }
    if normalized.ends_with("ing") && normalized.len() > 4 {
        let stem = &normalized[..normalized.len() - 3];
        if stem.ends_with("at") || stem.ends_with("iz") || stem.ends_with("bl") {
            return format!("{stem}e");
        }
        if stem.len() > 2 && stem.as_bytes()[stem.len() - 1] == stem.as_bytes()[stem.len() - 2] {
            return stem[..stem.len() - 1].to_string();
        }
        return stem.to_string();
    }
    if normalized.ends_with("ed") && normalized.len() > 3 {
        let stem = &normalized[..normalized.len() - 2];
        if stem.ends_with("at") || stem.ends_with("iz") || stem.ends_with("or") {
            return format!("{stem}e");
        }
        if stem.len() > 2 && stem.as_bytes()[stem.len() - 1] == stem.as_bytes()[stem.len() - 2] {
            return stem[..stem.len() - 1].to_string();
        }
        return stem.to_string();
    }
    if normalized.ends_with("es") && normalized.len() > 3 {
        let stem = &normalized[..normalized.len() - 2];
        if stem.ends_with("ss")
            || stem.ends_with('x')
            || stem.ends_with('z')
            || stem.ends_with("ch")
            || stem.ends_with("sh")
        {
            return stem.to_string();
        }
    }
    if normalized.ends_with('s')
        && normalized.len() > 2
        && !normalized.ends_with("ss")
        && !["news", "series", "species", "means", "analysis"].contains(&normalized.as_str())
    {
        return normalized[..normalized.len() - 1].to_string();
    }
    normalized
}

fn load_documents(asset_dir: &Path) -> Result<(Value, BTreeMap<String, Value>), String> {
    let manifest_path = asset_dir.join("manifest.json");
    let manifest_text = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("Failed to read {}: {error}", manifest_path.display()))?;
    let manifest: Value = serde_json::from_str(&manifest_text)
        .map_err(|error| format!("Invalid {}: {error}", manifest_path.display()))?;
    let typed: ExamManifest = serde_json::from_value(manifest.clone())
        .map_err(|error| format!("Invalid exam manifest: {error}"))?;
    if typed.schema_version != 1 || typed.files.is_empty() {
        return Err("Unsupported or empty exam manifest".to_string());
    }
    let mut documents = BTreeMap::new();
    for relative in typed.files {
        let source_path = PathBuf::from(&relative);
        let file_name = source_path
            .file_name()
            .ok_or_else(|| format!("Invalid exam asset path: {relative}"))?;
        let file_path = asset_dir.join(file_name);
        let text = fs::read_to_string(&file_path)
            .map_err(|error| format!("Failed to read {}: {error}", file_path.display()))?;
        let document = serde_json::from_str(&text)
            .map_err(|error| format!("Invalid {}: {error}", file_path.display()))?;
        documents.insert(file_name.to_string_lossy().to_string(), document);
    }
    Ok((manifest, documents))
}

pub fn build_exam_catalog(
    manifest: &Value,
    documents: &BTreeMap<String, Value>,
) -> Result<ExamCatalog, String> {
    let manifest: ExamManifest = serde_json::from_value(manifest.clone())
        .map_err(|error| format!("Invalid exam manifest: {error}"))?;
    if manifest.schema_version != 1 {
        return Err(format!(
            "Unsupported exam manifest schema {}",
            manifest.schema_version
        ));
    }
    let mut exams = Vec::new();
    for relative in manifest.files {
        let file_name = Path::new(&relative)
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .ok_or_else(|| format!("Invalid exam asset path: {relative}"))?;
        let value = documents
            .get(&file_name)
            .ok_or_else(|| format!("Missing exam document: {file_name}"))?;
        let document = parse_document(value)?;
        let papers = document.papers.iter().map(summarize_paper).collect();
        exams.push(ExamCatalogExam {
            exam: document.exam,
            papers,
        });
    }
    Ok(ExamCatalog {
        schema_version: 1,
        exams,
    })
}

pub fn build_exam_paper(
    exam: &str,
    paper_id: &str,
    documents: &BTreeMap<String, Value>,
) -> Result<Option<ExamPaper>, String> {
    for value in documents.values() {
        let document = parse_document(value)?;
        if document.exam != exam {
            continue;
        }
        if let Some(paper) = document
            .papers
            .into_iter()
            .find(|paper| paper.id == paper_id)
        {
            return Ok(Some(paper));
        }
    }
    Ok(None)
}

pub fn normalize_user_paper(value: Value) -> Result<ExamPaper, String> {
    let mut paper: ExamPaper = serde_json::from_value(value)
        .map_err(|error| format!("Invalid user exam paper: {error}"))?;
    if paper.schema_version != 1 {
        return Err(format!(
            "Unsupported user paper schema {}",
            paper.schema_version
        ));
    }
    if paper.id.trim().is_empty() || paper.exam.trim().is_empty() || paper.sections.is_empty() {
        return Err("User paper requires id, exam, and at least one section".to_string());
    }
    for section in &mut paper.sections {
        for question in &mut section.questions {
            question.capabilities = derive_capabilities(question, &section.passage);
        }
    }
    Ok(paper)
}

pub fn merge_user_papers(catalog: &mut ExamCatalog, papers: &[ExamPaper]) {
    for paper in papers {
        let summary = summarize_paper_with_origin(paper, "user");
        if let Some(exam) = catalog
            .exams
            .iter_mut()
            .find(|item| item.exam == paper.exam)
        {
            exam.papers.retain(|item| item.id != paper.id);
            exam.papers.push(summary);
            exam.papers.sort_by(|left, right| {
                right
                    .year
                    .cmp(&left.year)
                    .then_with(|| left.title.cmp(&right.title))
            });
        } else {
            catalog.exams.push(ExamCatalogExam {
                exam: paper.exam.clone(),
                papers: vec![summary],
            });
        }
    }
}

fn parse_document(value: &Value) -> Result<ExamDocument, String> {
    let mut document: ExamDocument = serde_json::from_value(value.clone())
        .map_err(|error| format!("Invalid exam document: {error}"))?;
    if document.schema_version != 1 {
        return Err(format!(
            "Unsupported exam document schema {}",
            document.schema_version
        ));
    }
    for paper in &mut document.papers {
        for section in &mut paper.sections {
            for question in &mut section.questions {
                question.capabilities = derive_capabilities(question, section.passage.as_str());
            }
        }
    }
    Ok(document)
}

fn summarize_paper(paper: &ExamPaper) -> ExamPaperSummary {
    summarize_paper_with_origin(paper, "bundled")
}

fn summarize_paper_with_origin(paper: &ExamPaper, origin: &str) -> ExamPaperSummary {
    let sections = paper
        .sections
        .iter()
        .map(|section| ExamSectionSummary {
            id: section.id.clone(),
            section_type: section.section_type.clone(),
            title: section.title.clone(),
            question_count: section.questions.len(),
            auto_gradable_count: section
                .questions
                .iter()
                .filter(|question| question.capabilities.auto_gradable)
                .count(),
        })
        .collect::<Vec<_>>();
    let questions = paper.sections.iter().flat_map(|section| &section.questions);
    let question_count = questions.clone().count();
    let answer_bearing_count = questions
        .clone()
        .filter(|question| {
            question
                .answer
                .as_deref()
                .is_some_and(|answer| !answer.trim().is_empty())
        })
        .count();
    let auto_gradable_count = questions
        .filter(|question| question.capabilities.auto_gradable)
        .count();
    ExamPaperSummary {
        id: paper.id.clone(),
        title: paper.title.clone(),
        year: paper.year,
        month: paper.month,
        set: paper.set,
        question_count,
        answer_bearing_count,
        auto_gradable_count,
        origin: origin.to_string(),
        sections,
    }
}

fn derive_capabilities(question: &ExamQuestion, passage: &str) -> ExamQuestionCapabilities {
    let answer = question
        .answer
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_uppercase();
    let auto_gradable = question.kind == "objective"
        && !question.choices.is_empty()
        && !answer.is_empty()
        && question.choices.iter().any(|choice| choice.label == answer);
    ExamQuestionCapabilities {
        browsable: true,
        answerable: !question.choices.is_empty()
            || matches!(question.kind.as_str(), "translation" | "writing")
            || !question.stem.trim().is_empty(),
        auto_gradable,
        causal_analyzable: auto_gradable
            && (!passage.trim().is_empty() || !question.stem.trim().is_empty()),
    }
}

fn default_question_kind() -> String {
    "subjective".to_string()
}

fn default_capabilities() -> ExamQuestionCapabilities {
    ExamQuestionCapabilities {
        browsable: false,
        answerable: false,
        auto_gradable: false,
        causal_analyzable: false,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;

    use super::{
        build_exam_catalog, build_exam_paper, normalize_user_paper, rank_exam_vocabulary,
        tokenize_english,
    };

    fn fixture() -> (serde_json::Value, BTreeMap<String, serde_json::Value>) {
        let manifest = json!({
            "schemaVersion": 1,
            "files": ["cet4.json"],
            "totalPapers": 1
        });
        let documents = BTreeMap::from([(
            "cet4.json".to_string(),
            json!({
                "schemaVersion": 1,
                "exam": "cet4",
                "papers": [{
                    "schemaVersion": 1,
                    "id": "cet4-2025-6-1",
                    "exam": "cet4",
                    "title": "CET4 2025-06 Set 1",
                    "year": 2025,
                    "month": 6,
                    "set": 1,
                    "source": {"repo": "fixture", "path": "fixture.json"},
                    "sections": [{
                        "id": "reading",
                        "type": "reading",
                        "title": "Reading",
                        "passage": "A short passage.",
                        "questions": [{
                            "id": "q1",
                            "number": 1,
                            "kind": "objective",
                            "stem": "Choose one.",
                            "choices": [
                                {"label": "A", "text": "First"},
                                {"label": "B", "text": "Second"}
                            ],
                            "answer": "A",
                            "explanation": "Because A is correct."
                        }]
                    }]
                }]
            }),
        )]);
        (manifest, documents)
    }

    #[test]
    fn catalog_summarizes_papers_without_exposing_full_questions() {
        let (manifest, documents) = fixture();

        let catalog = build_exam_catalog(&manifest, &documents).expect("build catalog");

        assert_eq!(catalog.exams.len(), 1);
        assert_eq!(catalog.exams[0].exam, "cet4");
        assert_eq!(catalog.exams[0].papers[0].question_count, 1);
        assert_eq!(catalog.exams[0].papers[0].auto_gradable_count, 1);
    }

    #[test]
    fn paper_adds_explicit_question_capabilities() {
        let (_, documents) = fixture();

        let paper = build_exam_paper("cet4", "cet4-2025-6-1", &documents)
            .expect("build paper")
            .expect("paper exists");

        let capabilities = &paper.sections[0].questions[0].capabilities;
        assert!(capabilities.browsable);
        assert!(capabilities.answerable);
        assert!(capabilities.auto_gradable);
        assert!(capabilities.causal_analyzable);
    }

    #[test]
    fn workspace_assets_load_through_the_mobile_bundle_directory_contract() {
        let asset_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../apps/flutter_mobile/assets/exam-papers");

        let catalog = super::load_exam_catalog(&asset_dir).expect("load workspace catalog");

        assert_eq!(catalog.exams.len(), 4);
        assert_eq!(
            catalog
                .exams
                .iter()
                .map(|exam| exam.papers.len())
                .sum::<usize>(),
            149
        );
        assert_eq!(
            catalog
                .exams
                .iter()
                .flat_map(|exam| &exam.papers)
                .map(|paper| paper.auto_gradable_count)
                .sum::<usize>(),
            5266
        );
    }

    #[test]
    fn paper_loading_does_not_parse_unrelated_exam_documents() {
        let root = std::env::temp_dir().join(format!(
            "word-mobile-exam-paper-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("create fixture dir");
        std::fs::write(
            root.join("manifest.json"),
            r#"{"schemaVersion":1,"files":["cet4.json","cet6.json"]}"#,
        )
        .expect("write manifest");
        std::fs::write(
            root.join("cet4.json"),
            r#"{"schemaVersion":1,"exam":"cet4","papers":[{"schemaVersion":1,"id":"paper-1","exam":"cet4","title":"Paper","year":2025,"source":{},"sections":[]}]}"#,
        )
        .expect("write target exam");
        std::fs::write(root.join("cet6.json"), "not valid json")
            .expect("write unrelated invalid exam");

        let paper = super::load_exam_paper(&root, "cet4", "paper-1")
            .expect("load target only")
            .expect("paper exists");

        assert_eq!(paper.id, "paper-1");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn english_tokenizer_preserves_repeated_word_offsets() {
        let tokens = tokenize_english("Well-known words aren't always known.");

        assert_eq!(
            tokens
                .iter()
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>(),
            ["Well-known", "words", "aren't", "always", "known",]
        );
        assert_eq!((tokens[0].start_offset, tokens[0].end_offset), (0, 10));
        assert_eq!(
            &"Well-known words aren't always known."[tokens[4].start_offset..tokens[4].end_offset],
            "known"
        );
    }

    #[test]
    fn english_tokenizer_reports_utf16_offsets_after_unicode_text() {
        let tokens = tokenize_english("汉😀 available");

        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].text, "available");
        assert_eq!((tokens[0].start_offset, tokens[0].end_offset), (4, 13));
    }

    #[test]
    fn vocabulary_ranking_distinguishes_occurrences_articles_and_papers() {
        fn paper(id: &str, passage: &str) -> super::ExamPaper {
            normalize_user_paper(json!({
                "schemaVersion": 1,
                "id": id,
                "exam": "custom",
                "title": id,
                "year": 2026,
                "source": {},
                "sections": [{
                    "id": "reading",
                    "type": "reading",
                    "title": "Reading",
                    "passage": passage,
                    "questions": []
                }]
            }))
            .expect("paper fixture")
        }

        let ranked = rank_exam_vocabulary(
            &[
                paper("paper-1", "resilient resilient isolated"),
                paper("paper-2", "resilient recurring"),
            ],
            10,
        );
        let resilient = ranked
            .iter()
            .find(|item| item.word == "resilient")
            .expect("resilient rank");
        assert_eq!(resilient.occurrence_count, 3);
        assert_eq!(resilient.article_count, 2);
        assert_eq!(resilient.paper_count, 2);
        assert_eq!(
            ranked.first().map(|item| item.word.as_str()),
            Some("resilient")
        );
    }

    #[test]
    fn vocabulary_ranking_collapses_inflected_forms() {
        fn paper(passage: &str) -> super::ExamPaper {
            normalize_user_paper(json!({
                "schemaVersion": 1,
                "id": "paper-forms",
                "exam": "custom",
                "title": "forms",
                "year": 2026,
                "source": {},
                "sections": [{
                    "id": "reading",
                    "type": "reading",
                    "title": "Reading",
                    "passage": passage,
                    "questions": []
                }]
            }))
            .expect("paper fixture")
        }

        let ranked = rank_exam_vocabulary(&[paper("patent patents authorized authorize")], 10);
        assert_eq!(
            ranked
                .iter()
                .find(|item| item.word == "patent")
                .unwrap()
                .occurrence_count,
            2
        );
        assert_eq!(
            ranked
                .iter()
                .find(|item| item.word == "authorize")
                .unwrap()
                .occurrence_count,
            2
        );
    }
}
