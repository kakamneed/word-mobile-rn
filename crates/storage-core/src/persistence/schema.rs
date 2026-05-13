use rusqlite::Connection;

/// Current schema version. Increment when structural changes are needed.
pub const SCHEMA_VERSION: i64 = 13;

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

    // Imported wrong words from AI/OCR-assisted external sources.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS imported_wrong_words (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            batch_id TEXT NOT NULL,
            candidate_id TEXT NOT NULL,
            source_type TEXT NOT NULL DEFAULT '',
            source_name TEXT NOT NULL DEFAULT '',
            entry_id INTEGER,
            word TEXT NOT NULL,
            meaning TEXT DEFAULT '',
            occurrence_count INTEGER NOT NULL DEFAULT 1,
            confidence REAL NOT NULL DEFAULT 0.0,
            evidence TEXT NOT NULL DEFAULT '',
            is_high_frequency INTEGER NOT NULL DEFAULT 0,
            imported_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(batch_id, candidate_id),
            FOREIGN KEY (entry_id) REFERENCES entries(id)
        );",
    )
    .map_err(|e| {
        crate::StorageError::Schema(format!("Failed to create imported_wrong_words: {e}"))
    })?;

    // User-authored memory hints for difficult words.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS word_hints (
            entry_id INTEGER NOT NULL PRIMARY KEY,
            hint_text TEXT NOT NULL DEFAULT '',
            source TEXT NOT NULL DEFAULT 'user',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (entry_id) REFERENCES entries(id) ON DELETE CASCADE
        );",
    )
    .map_err(|e| crate::StorageError::Schema(format!("Failed to create word_hints: {e}")))?;

    create_mastered_entry_tables(conn)?;

    // Sync outbox
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS sync_outbox (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            domain TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            idempotency_key TEXT NOT NULL UNIQUE,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            attempt_count INTEGER NOT NULL DEFAULT 0,
            last_attempt_at TEXT,
            status TEXT NOT NULL DEFAULT 'pending'
        );",
    )
    .map_err(|e| crate::StorageError::Schema(format!("Failed to create sync_outbox: {e}")))?;

    // Sync cursor state
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS sync_cursor_state (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id TEXT,
            device_id TEXT NOT NULL,
            last_pushed_at TEXT,
            last_pulled_cursor TEXT,
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )
    .map_err(|e| crate::StorageError::Schema(format!("Failed to create sync_cursor_state: {e}")))?;

    // Sync dead letter
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS sync_dead_letter (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            domain TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            idempotency_key TEXT NOT NULL,
            failure_code TEXT NOT NULL,
            failure_message TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            last_attempt_at TEXT
        );",
    )
    .map_err(|e| crate::StorageError::Schema(format!("Failed to create sync_dead_letter: {e}")))?;

    create_reward_image_tables(conn)?;
    create_local_leaderboard_tables(conn)?;

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
         CREATE INDEX IF NOT EXISTS idx_study_results_session ON study_results(session_id);
         CREATE INDEX IF NOT EXISTS idx_imported_wrong_words_entry ON imported_wrong_words(entry_id);
         CREATE INDEX IF NOT EXISTS idx_imported_wrong_words_word ON imported_wrong_words(word);
         CREATE INDEX IF NOT EXISTS idx_word_hints_updated ON word_hints(updated_at);
         CREATE INDEX IF NOT EXISTS idx_mastered_entries_source ON mastered_entries(source_entry_id);
         CREATE INDEX IF NOT EXISTS idx_mastered_entries_mastered_at ON mastered_entries(mastered_at);
         CREATE INDEX IF NOT EXISTS idx_sync_outbox_domain ON sync_outbox(domain);
         CREATE INDEX IF NOT EXISTS idx_sync_outbox_status ON sync_outbox(status);
         CREATE INDEX IF NOT EXISTS idx_sync_dead_letter_domain ON sync_dead_letter(domain);
         CREATE INDEX IF NOT EXISTS idx_sync_cursor_device ON sync_cursor_state(device_id);
         CREATE INDEX IF NOT EXISTS idx_reward_images_status ON reward_images(moderation_status, is_withdrawn, created_at);
         CREATE INDEX IF NOT EXISTS idx_reward_images_owner ON reward_images(owner_key, moderation_status, created_at);
         CREATE INDEX IF NOT EXISTS idx_reward_image_votes_week ON reward_image_votes(week_start, image_id);",
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
    if from_version < 9 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS imported_wrong_words (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                batch_id TEXT NOT NULL,
                candidate_id TEXT NOT NULL,
                source_type TEXT NOT NULL DEFAULT '',
                source_name TEXT NOT NULL DEFAULT '',
                entry_id INTEGER,
                word TEXT NOT NULL,
                meaning TEXT DEFAULT '',
                occurrence_count INTEGER NOT NULL DEFAULT 1,
                confidence REAL NOT NULL DEFAULT 0.0,
                evidence TEXT NOT NULL DEFAULT '',
                is_high_frequency INTEGER NOT NULL DEFAULT 0,
                imported_at TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(batch_id, candidate_id),
                FOREIGN KEY (entry_id) REFERENCES entries(id)
            );
            CREATE INDEX IF NOT EXISTS idx_imported_wrong_words_entry ON imported_wrong_words(entry_id);
            CREATE INDEX IF NOT EXISTS idx_imported_wrong_words_word ON imported_wrong_words(word);",
        )
        .map_err(|e| crate::StorageError::Schema(format!("Failed to migrate imported wrong words: {e}")))?;
    }
    if from_version < 10 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS word_hints (
                entry_id INTEGER NOT NULL PRIMARY KEY,
                hint_text TEXT NOT NULL DEFAULT '',
                source TEXT NOT NULL DEFAULT 'user',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (entry_id) REFERENCES entries(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_word_hints_updated ON word_hints(updated_at);",
        )
        .map_err(|e| crate::StorageError::Schema(format!("Failed to migrate word hints: {e}")))?;
    }
    if from_version < 11 {
        create_reward_image_tables(conn)?;
    }
    if from_version < 12 {
        create_local_leaderboard_tables(conn)?;
    }
    if from_version < 13 {
        create_mastered_entry_tables(conn)?;
    }
    Ok(())
}

fn create_mastered_entry_tables(conn: &Connection) -> Result<(), crate::StorageError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS mastered_entries (
            source_entry_id TEXT NOT NULL PRIMARY KEY,
            entry_id INTEGER,
            reason TEXT NOT NULL DEFAULT 'mastered',
            mastered_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (entry_id) REFERENCES entries(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_mastered_entries_entry ON mastered_entries(entry_id);
        CREATE INDEX IF NOT EXISTS idx_mastered_entries_source ON mastered_entries(source_entry_id);
        CREATE INDEX IF NOT EXISTS idx_mastered_entries_mastered_at ON mastered_entries(mastered_at);",
    )
    .map_err(|e| {
        crate::StorageError::Schema(format!("Failed to create mastered entry tables: {e}"))
    })?;
    Ok(())
}

fn create_reward_image_tables(conn: &Connection) -> Result<(), crate::StorageError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS reward_image_upload_entitlements (
            owner_key TEXT NOT NULL PRIMARY KEY DEFAULT 'local',
            available_uploads INTEGER NOT NULL DEFAULT 0 CHECK (available_uploads >= 0),
            last_granted_streak_milestone INTEGER NOT NULL DEFAULT 0 CHECK (last_granted_streak_milestone >= 0),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS reward_images (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            owner_key TEXT NOT NULL DEFAULT 'local',
            local_path TEXT NOT NULL,
            mime_type TEXT NOT NULL DEFAULT 'image/jpeg',
            original_filename TEXT NOT NULL DEFAULT '',
            moderation_status TEXT NOT NULL DEFAULT 'pending',
            moderation_reason TEXT NOT NULL DEFAULT '',
            moderation_checked_at TEXT,
            is_withdrawn INTEGER NOT NULL DEFAULT 0,
            draw_pool_eligible INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            CHECK (length(trim(local_path)) > 0),
            CHECK (mime_type IN ('image/jpeg', 'image/png', 'image/webp')),
            CHECK (moderation_status IN ('pending', 'approved', 'rejected'))
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_reward_images_local_path
            ON reward_images(local_path);

        CREATE TABLE IF NOT EXISTS leaderboard_image_tags (
            owner_key TEXT NOT NULL PRIMARY KEY DEFAULT 'local',
            image_id INTEGER NOT NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (image_id) REFERENCES reward_images(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS reward_image_votes (
            image_id INTEGER NOT NULL,
            voter_key TEXT NOT NULL DEFAULT 'local',
            week_start TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (image_id, voter_key, week_start),
            FOREIGN KEY (image_id) REFERENCES reward_images(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS reward_image_weekly_winners (
            week_start TEXT NOT NULL,
            rank INTEGER NOT NULL CHECK (rank BETWEEN 1 AND 3),
            image_id INTEGER NOT NULL,
            vote_count INTEGER NOT NULL DEFAULT 0 CHECK (vote_count >= 0),
            promoted_at TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (week_start, rank),
            UNIQUE (week_start, image_id),
            FOREIGN KEY (image_id) REFERENCES reward_images(id) ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS reward_draw_pool_images (
            image_id INTEGER NOT NULL PRIMARY KEY,
            source_week_start TEXT,
            source_rank INTEGER CHECK (source_rank IS NULL OR source_rank BETWEEN 1 AND 3),
            is_active INTEGER NOT NULL DEFAULT 1,
            added_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (image_id) REFERENCES reward_images(id) ON DELETE RESTRICT
        );

        CREATE INDEX IF NOT EXISTS idx_reward_images_status
            ON reward_images(moderation_status, is_withdrawn, created_at);
        CREATE INDEX IF NOT EXISTS idx_reward_images_owner
            ON reward_images(owner_key, moderation_status, created_at);
        CREATE INDEX IF NOT EXISTS idx_reward_image_votes_week
            ON reward_image_votes(week_start, image_id);",
    )
    .map_err(|e| {
        crate::StorageError::Schema(format!("Failed to create reward image tables: {e}"))
    })?;
    Ok(())
}

fn create_local_leaderboard_tables(conn: &Connection) -> Result<(), crate::StorageError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS local_leaderboard_summaries (
            user_key TEXT NOT NULL,
            display_name TEXT NOT NULL DEFAULT '',
            total_questions INTEGER NOT NULL DEFAULT 0 CHECK (total_questions >= 0),
            correct_count INTEGER NOT NULL DEFAULT 0 CHECK (correct_count >= 0),
            mixed_test_total_questions INTEGER NOT NULL DEFAULT 0 CHECK (mixed_test_total_questions >= 0),
            mixed_test_correct_count INTEGER NOT NULL DEFAULT 0 CHECK (mixed_test_correct_count >= 0),
            current_streak_days INTEGER NOT NULL DEFAULT 0 CHECK (current_streak_days >= 0),
            period TEXT NOT NULL DEFAULT 'all_time',
            period_start TEXT NOT NULL DEFAULT '1970-01-01',
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (user_key, period, period_start),
            CHECK (correct_count <= total_questions),
            CHECK (mixed_test_correct_count <= mixed_test_total_questions),
            CHECK (period IN ('weekly', 'monthly', 'all_time'))
        );

        CREATE INDEX IF NOT EXISTS idx_local_leaderboard_period
            ON local_leaderboard_summaries(period, period_start, total_questions DESC);
        CREATE INDEX IF NOT EXISTS idx_local_leaderboard_streak
            ON local_leaderboard_summaries(current_streak_days DESC);",
    )
    .map_err(|e| {
        crate::StorageError::Schema(format!("Failed to create local leaderboard tables: {e}"))
    })?;
    Ok(())
}
