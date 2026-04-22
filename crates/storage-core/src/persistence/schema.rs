use rusqlite::Connection;

/// Current schema version. Increment when structural changes are needed.
pub const SCHEMA_VERSION: i64 = 7;

/// Applies all idempotent schema migrations to the database.
///
/// This function is safe to call on every application start. It creates tables
/// only if they do not exist and inserts default metadata rows where missing.
pub fn apply_schema(conn: &Connection) -> Result<(), crate::StorageError> {
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
        .map_err(|e| crate::StorageError::Schema(format!("Failed to set pragmas: {e}")))?;

    // Schema version tracking table
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _schema_version (
            version INTEGER NOT NULL PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )
    .map_err(|e| crate::StorageError::Schema(format!("Failed to create _schema_version: {e}")))?;

    // Application metadata table
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS app_metadata (
            key TEXT NOT NULL PRIMARY KEY,
            value TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )
    .map_err(|e| crate::StorageError::Schema(format!("Failed to create app_metadata: {e}")))?;

    // Application settings table
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT NOT NULL PRIMARY KEY,
            value_json TEXT NOT NULL DEFAULT '{}',
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )
    .map_err(|e| crate::StorageError::Schema(format!("Failed to create app_settings: {e}")))?;

    // Source version metadata
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS source_versions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_name TEXT NOT NULL DEFAULT 'kajweb/dict',
            source_commit TEXT NOT NULL UNIQUE,
            imported_at TEXT NOT NULL DEFAULT (datetime('now')),
            snapshot_path TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'pending',
            notes TEXT
        );",
    )
    .map_err(|e| crate::StorageError::Schema(format!("Failed to create source_versions: {e}")))?;

    // Wordbooks
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS wordbooks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            code TEXT NOT NULL,
            name TEXT NOT NULL,
            category TEXT NOT NULL DEFAULT 'exam',
            description TEXT,
            source_book_id TEXT NOT NULL DEFAULT '',
            source_version_id INTEGER NOT NULL,
            total_entries INTEGER NOT NULL DEFAULT 0,
            is_active INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(code, source_version_id),
            FOREIGN KEY (source_version_id) REFERENCES source_versions(id)
        );",
    )
    .map_err(|e| crate::StorageError::Schema(format!("Failed to create wordbooks: {e}")))?;

    // Vocabulary entries
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS entries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_version_id INTEGER NOT NULL,
            source_entry_key TEXT NOT NULL,
            word TEXT NOT NULL,
            lemma TEXT NOT NULL DEFAULT '',
            phonetic_us TEXT DEFAULT '',
            phonetic_uk TEXT DEFAULT '',
            part_of_speech TEXT DEFAULT '',
            frequency REAL NOT NULL DEFAULT 0.0,
            difficulty TEXT DEFAULT '',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(source_version_id, source_entry_key),
            FOREIGN KEY (source_version_id) REFERENCES source_versions(id)
        );",
    )
    .map_err(|e| crate::StorageError::Schema(format!("Failed to create entries: {e}")))?;

    // Chinese meanings
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS entry_meanings (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            entry_id INTEGER NOT NULL,
            pos TEXT NOT NULL DEFAULT '',
            meaning_cn TEXT NOT NULL DEFAULT '',
            meaning_en TEXT DEFAULT '',
            sort_order INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (entry_id) REFERENCES entries(id) ON DELETE CASCADE
        );",
    )
    .map_err(|e| crate::StorageError::Schema(format!("Failed to create entry_meanings: {e}")))?;

    // Example sentences
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS entry_examples (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            entry_id INTEGER NOT NULL,
            sentence_en TEXT NOT NULL DEFAULT '',
            sentence_cn TEXT NOT NULL DEFAULT '',
            sort_order INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (entry_id) REFERENCES entries(id) ON DELETE CASCADE
        );",
    )
    .map_err(|e| crate::StorageError::Schema(format!("Failed to create entry_examples: {e}")))?;

    // Wordbook entry junction
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS wordbook_entries (
            wordbook_id INTEGER NOT NULL,
            entry_id INTEGER NOT NULL,
            rank_in_book INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (wordbook_id, entry_id),
            FOREIGN KEY (wordbook_id) REFERENCES wordbooks(id) ON DELETE CASCADE,
            FOREIGN KEY (entry_id) REFERENCES entries(id) ON DELETE CASCADE
        );",
    )
    .map_err(|e| crate::StorageError::Schema(format!("Failed to create wordbook_entries: {e}")))?;

    // Plan templates
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS plan_templates (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            active_wordbook_ids TEXT NOT NULL DEFAULT '[]',
            new_words_per_day INTEGER NOT NULL DEFAULT 20,
            review_words_per_day INTEGER NOT NULL DEFAULT 30,
            mixed_test_per_day INTEGER NOT NULL DEFAULT 10,
            wrong_word_test_per_day INTEGER NOT NULL DEFAULT 5,
            growth_interval_days INTEGER NOT NULL DEFAULT 7,
            growth_increment INTEGER NOT NULL DEFAULT 5,
            is_active INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )
    .map_err(|e| crate::StorageError::Schema(format!("Failed to create plan_templates: {e}")))?;

    // Study sessions
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS study_sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL UNIQUE,
            mode TEXT NOT NULL,
            total_words INTEGER NOT NULL,
            wordbook_id INTEGER,
            started_at TEXT NOT NULL,
            completed_at TEXT,
            FOREIGN KEY (wordbook_id) REFERENCES wordbooks(id)
        );",
    )
    .map_err(|e| crate::StorageError::Schema(format!("Failed to create study_sessions: {e}")))?;

    // Study results
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS study_results (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            question_id TEXT NOT NULL,
            entry_id INTEGER NOT NULL,
            question_type TEXT NOT NULL,
            user_response TEXT NOT NULL,
            normalized_response TEXT,
            correct_answer TEXT NOT NULL,
            outcome TEXT NOT NULL,
            response_time_ms INTEGER NOT NULL,
            answered_at TEXT NOT NULL,
            FOREIGN KEY (session_id) REFERENCES study_sessions(session_id),
            FOREIGN KEY (entry_id) REFERENCES entries(id)
        );",
    )
    .map_err(|e| crate::StorageError::Schema(format!("Failed to create study_results: {e}")))?;

    // Indexes
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_entries_source_version ON entries(source_version_id);
         CREATE INDEX IF NOT EXISTS idx_entries_word ON entries(word);
         CREATE INDEX IF NOT EXISTS idx_entries_source_key ON entries(source_entry_key);
         CREATE INDEX IF NOT EXISTS idx_entry_meanings_entry ON entry_meanings(entry_id);
         CREATE INDEX IF NOT EXISTS idx_entry_examples_entry ON entry_examples(entry_id);
         CREATE INDEX IF NOT EXISTS idx_wordbooks_source_version ON wordbooks(source_version_id);
         CREATE INDEX IF NOT EXISTS idx_wordbooks_code ON wordbooks(code);
         CREATE INDEX IF NOT EXISTS idx_wordbooks_is_active ON wordbooks(is_active);
         CREATE INDEX IF NOT EXISTS idx_wordbook_entries_wordbook ON wordbook_entries(wordbook_id);
         CREATE INDEX IF NOT EXISTS idx_wordbook_entries_entry ON wordbook_entries(entry_id);
         CREATE INDEX IF NOT EXISTS idx_source_versions_status ON source_versions(status);
         CREATE INDEX IF NOT EXISTS idx_source_versions_commit ON source_versions(source_commit);
         CREATE INDEX IF NOT EXISTS idx_study_sessions_session_id ON study_sessions(session_id);
         CREATE INDEX IF NOT EXISTS idx_study_results_session ON study_results(session_id);",
    )
    .map_err(|e| crate::StorageError::Schema(format!("Failed to create indexes: {e}")))?;

    // Record schema version
    record_schema_version(conn)?;

    Ok(())
}

fn record_schema_version(conn: &Connection) -> Result<(), crate::StorageError> {
    let current: Option<i64> = conn
        .query_row("SELECT MAX(version) FROM _schema_version", [], |row| {
            row.get(0)
        })
        .map_err(|e| crate::StorageError::Schema(format!("Failed to query version: {e}")))?;

    match current {
        None => {
            conn.execute(
                "INSERT INTO _schema_version (version) VALUES (?1)",
                [SCHEMA_VERSION],
            )
            .map_err(|e| crate::StorageError::Schema(format!("Failed to insert version: {e}")))?;
        }
        Some(v) if v < SCHEMA_VERSION => {
            run_migrations(conn, v)?;
            conn.execute(
                "INSERT INTO _schema_version (version) VALUES (?1)",
                [SCHEMA_VERSION],
            )
            .map_err(|e| crate::StorageError::Schema(format!("Failed to record version: {e}")))?;
        }
        Some(_) => {}
    }

    Ok(())
}

fn run_migrations(conn: &Connection, from_version: i64) -> Result<(), crate::StorageError> {
    // Add migrations here as needed
    // Example: if from_version < 2 { ... }
    let _ = conn;
    let _ = from_version;
    Ok(())
}
