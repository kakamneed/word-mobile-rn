//! Placeholder for vocabulary status functionality.

/// Vocabulary status summary.
#[derive(Debug, Clone)]
pub struct VocabularyStatus {
    pub total_entries: usize,
    pub active_wordbooks: usize,
    pub has_bundled_snapshot: bool,
}
