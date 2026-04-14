use serde::{Deserialize, Serialize};

/// Application-owned standardized vocabulary entry.
///
/// This is the official intermediate representation between upstream raw
/// source data (e.g. `kajweb/dict` JSON) and the SQLite import layer.
/// No runtime code should depend directly on the upstream structure; all
/// consumption flows through this model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StandardizedEntry {
    /// Stable upstream identifier, e.g. "CET4_3_1" from kajweb/dict wordId.
    pub source_id: String,

    /// The headword as it appears in the source (exact spelling/capitalization).
    pub word: String,

    /// Base lemma form. Often the same as `word` but may differ for
    /// inflected forms.
    pub lemma: String,

    /// US English phonetic transcription, e.g. "'kænsl".
    pub phonetic_us: Option<String>,

    /// UK English phonetic transcription.
    pub phonetic_uk: Option<String>,

    /// Primary part of speech, e.g. "vt", "n", "adj".
    /// Derived from the first meaning in the upstream `trans` array.
    pub part_of_speech: Option<String>,

    /// Chinese meanings grouped by part of speech.
    pub meanings_zh: Vec<MeaningZh>,

    /// Example sentences with Chinese translations.
    pub examples: Vec<EntryExample>,

    /// Book-level tags from the upstream source, e.g. ["四级"].
    pub tags: Vec<String>,

    /// Product-assigned difficulty category (e.g. "CET4", "CET6", "考研").
    /// Populated from upstream book tags during normalization.
    pub difficulty: Option<String>,

    /// Derived frequency score. Since `kajweb/dict` has no explicit
    /// frequency field, this is computed from `wordRank` using the
    /// formula: `10000.0 / rank`. Rank 1 (the most prominent word in
    /// the book) receives the highest frequency value.
    pub frequency: f64,
}

/// A single Chinese meaning entry for a word.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeaningZh {
    /// Part of speech tag, e.g. "vt", "n".
    pub pos: String,

    /// Chinese translation string.
    pub meaning_cn: String,

    /// Optional English paraphrase or alternative translation.
    pub meaning_en: Option<String>,
}

/// An example sentence with translation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryExample {
    /// The example sentence in English.
    pub sentence_en: String,

    /// The Chinese translation of the example sentence.
    pub sentence_cn: String,
}
