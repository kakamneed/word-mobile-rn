use serde::{Deserialize, Serialize};

/// A product-facing wordbook representing an exam or use-case vocabulary set.
///
/// Wordbooks are organized by user-meaningful categories (e.g. CET-4, CET-6,
/// 考研, 通用高频) rather than exposing the raw upstream source structure.
/// Each wordbook links to a source version so that its data provenance is
/// always traceable.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Wordbook {
    /// Primary key in SQLite. None when not yet persisted.
    pub id: Option<i64>,

    /// Stable product code, e.g. "CET4", "CET6", "KAOYAN".
    /// Unique across all wordbooks.
    pub code: String,

    /// Human-readable wordbook name, e.g. "CET-4 核心词汇".
    pub name: String,

    /// Category grouping, e.g. "exam", "frequency", "custom".
    pub category: String,

    /// Optional description for the wordbook.
    pub description: Option<String>,

    /// The upstream book identifier from `kajweb/dict`, e.g. "CET4_3".
    pub source_book_id: String,

    /// Foreign key to `source_versions.id`. Links this wordbook to a
    /// specific import snapshot. None when the wordbook is first derived
    /// but not yet persisted.
    pub source_version_id: Option<i64>,

    /// Total number of entries in this wordbook.
    pub total_entries: i64,

    /// Whether this wordbook is currently active for study.
    /// 0 = inactive, 1 = active.
    pub is_active: bool,
}
