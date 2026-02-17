use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use std::path::Path;

pub struct Database {
    pub pool: Pool<Sqlite>,
}

impl Database {
    pub async fn new(path: &Path) -> Result<Self, sqlx::Error> {
        let db_url = format!("sqlite:{}?mode=rwc", path.display());

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await?;

        let db = Self { pool };
        db.run_migrations().await?;
        db.ensure_default_settings().await?;

        Ok(db)
    }

    async fn run_migrations(&self) -> Result<(), sqlx::Error> {
        // Create clipboard_items table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS clipboard_items (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                content_preview TEXT NOT NULL,
                copied_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                source_app TEXT,
                is_pinned INTEGER NOT NULL DEFAULT 0,
                character_count INTEGER,
                word_count INTEGER
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create indexes for clipboard_items
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_clipboard_copied_at ON clipboard_items(copied_at DESC)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_clipboard_content ON clipboard_items(content)")
            .execute(&self.pool)
            .await?;

        // Check if glossary_entries table exists with old schema and migrate if needed
        // This must happen BEFORE creating the new table or indexes
        let table_exists: Option<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='glossary_entries'",
        )
        .fetch_optional(&self.pool)
        .await?;

        if table_exists.is_some() {
            // Table exists - check if it needs migration
            let _ = self.migrate_glossary_schema().await;
        }

        // Create glossary_entries table (keyword + description format for LLM context)
        // This will only create if it doesn't exist (after migration or on fresh install)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS glossary_entries (
                id TEXT PRIMARY KEY,
                keyword TEXT NOT NULL UNIQUE,
                description TEXT NOT NULL,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                usage_count INTEGER NOT NULL DEFAULT 0
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create index for glossary_entries keyword search
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_glossary_keyword ON glossary_entries(keyword)")
            .execute(&self.pool)
            .await?;

        // Create translations table (cache)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS translations (
                id TEXT PRIMARY KEY,
                source_text TEXT NOT NULL,
                translated_text TEXT NOT NULL,
                source_language TEXT NOT NULL CHECK(source_language IN ('ko', 'en')),
                target_language TEXT NOT NULL CHECK(target_language IN ('ko', 'en')),
                model TEXT NOT NULL,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                glossary_used TEXT,
                input_tokens INTEGER,
                output_tokens INTEGER
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create index for translations lookup
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_translation_lookup ON translations(source_text, source_language, target_language)",
        )
        .execute(&self.pool)
        .await?;

        // Create user_settings table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS user_settings (
                id TEXT PRIMARY KEY DEFAULT 'default',
                max_history_count INTEGER NOT NULL DEFAULT 50,
                preferred_model TEXT NOT NULL DEFAULT 'claude-haiku-4-5-20251001',
                auto_detect_language INTEGER NOT NULL DEFAULT 1,
                double_press_interval INTEGER NOT NULL DEFAULT 500,
                translation_cache_days INTEGER NOT NULL DEFAULT 7,
                show_source_app INTEGER NOT NULL DEFAULT 1,
                popup_position TEXT NOT NULL DEFAULT 'cursor',
                launch_at_login INTEGER NOT NULL DEFAULT 0,
                paste_delay_ms INTEGER NOT NULL DEFAULT 150,
                api_key TEXT,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Migration: add api_key column if it doesn't exist (for existing databases)
        let _ = sqlx::query("ALTER TABLE user_settings ADD COLUMN api_key TEXT")
            .execute(&self.pool)
            .await;

        // Migration: add paste_delay_ms column if it doesn't exist
        let _ = sqlx::query(
            "ALTER TABLE user_settings ADD COLUMN paste_delay_ms INTEGER NOT NULL DEFAULT 150",
        )
        .execute(&self.pool)
        .await;

        // Migration: add updated_at column to clipboard_items if it doesn't exist
        let _ = sqlx::query("ALTER TABLE clipboard_items ADD COLUMN updated_at DATETIME")
            .execute(&self.pool)
            .await;

        // Create monitor_window_sizes table for per-monitor adaptive window sizing
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS monitor_window_sizes (
                monitor_key TEXT PRIMARY KEY,
                window_width INTEGER NOT NULL,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn ensure_default_settings(&self) -> Result<(), sqlx::Error> {
        // Insert default settings if not exists
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO user_settings (id) VALUES ('default')
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ============================================
    // Clipboard Operations
    // ============================================

    pub async fn get_clipboard_history(
        &self,
        limit: i32,
        offset: i32,
        search_query: Option<&str>,
    ) -> Result<(Vec<ClipboardItemRow>, i64), sqlx::Error> {
        let items = if let Some(query) = search_query {
            let search_pattern = format!("%{}%", query);
            sqlx::query_as::<_, ClipboardItemRow>(
                r#"
                SELECT id, content, content_preview, copied_at, source_app, is_pinned, character_count, word_count, updated_at
                FROM clipboard_items
                WHERE content LIKE ?
                ORDER BY is_pinned DESC, copied_at DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(&search_pattern)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, ClipboardItemRow>(
                r#"
                SELECT id, content, content_preview, copied_at, source_app, is_pinned, character_count, word_count, updated_at
                FROM clipboard_items
                ORDER BY is_pinned DESC, copied_at DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        };

        let total: (i64,) = if let Some(query) = search_query {
            let search_pattern = format!("%{}%", query);
            sqlx::query_as("SELECT COUNT(*) FROM clipboard_items WHERE content LIKE ?")
                .bind(&search_pattern)
                .fetch_one(&self.pool)
                .await?
        } else {
            sqlx::query_as("SELECT COUNT(*) FROM clipboard_items")
                .fetch_one(&self.pool)
                .await?
        };

        Ok((items, total.0))
    }

    pub async fn insert_clipboard_item(&self, item: &ClipboardItemRow) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO clipboard_items (id, content, content_preview, copied_at, source_app, is_pinned, character_count, word_count)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&item.id)
        .bind(&item.content)
        .bind(&item.content_preview)
        .bind(&item.copied_at)
        .bind(&item.source_app)
        .bind(item.is_pinned)
        .bind(item.character_count)
        .bind(item.word_count)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_clipboard_item(&self, id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM clipboard_items WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete all clipboard items (clear history)
    pub async fn clear_all_clipboard_items(&self) -> Result<i64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM clipboard_items")
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() as i64)
    }

    pub async fn toggle_pin_clipboard_item(&self, id: &str) -> Result<Option<bool>, sqlx::Error> {
        let row: Option<(i32,)> =
            sqlx::query_as("SELECT is_pinned FROM clipboard_items WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

        if let Some((current_pinned,)) = row {
            let new_pinned = if current_pinned == 0 { 1 } else { 0 };
            sqlx::query("UPDATE clipboard_items SET is_pinned = ? WHERE id = ?")
                .bind(new_pinned)
                .bind(id)
                .execute(&self.pool)
                .await?;
            Ok(Some(new_pinned == 1))
        } else {
            Ok(None)
        }
    }

    pub async fn cleanup_old_clipboard_items(&self, max_count: i32) -> Result<(), sqlx::Error> {
        // Get the count of pinned items (these are always preserved)
        let pinned_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM clipboard_items WHERE is_pinned = 1")
                .fetch_one(&self.pool)
                .await?;

        // Calculate how many non-pinned items we can keep
        // Total slots = max_count, pinned items take priority
        let non_pinned_slots = std::cmp::max(0, max_count as i64 - pinned_count.0) as i32;

        // Delete oldest non-pinned items if count exceeds available slots
        sqlx::query(
            r#"
            DELETE FROM clipboard_items
            WHERE id IN (
                SELECT id FROM clipboard_items
                WHERE is_pinned = 0
                ORDER BY copied_at DESC
                LIMIT -1 OFFSET ?
            )
            "#,
        )
        .bind(non_pinned_slots)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Find a clipboard item by its content (for deduplication)
    pub async fn find_clipboard_item_by_content(
        &self,
        content: &str,
    ) -> Result<Option<String>, sqlx::Error> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT id FROM clipboard_items WHERE content = ? LIMIT 1")
                .bind(content)
                .fetch_optional(&self.pool)
                .await?;

        Ok(row.map(|(id,)| id))
    }

    /// Update the timestamp of an existing clipboard item (move to top)
    pub async fn update_clipboard_item_timestamp(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE clipboard_items SET copied_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Create a new clipboard item (for manual creation)
    pub async fn create_clipboard_item(
        &self,
        id: &str,
        content: &str,
        content_preview: &str,
        character_count: i32,
        word_count: i32,
    ) -> Result<ClipboardItemRow, sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO clipboard_items (id, content, content_preview, copied_at, source_app, is_pinned, character_count, word_count, updated_at)
            VALUES (?, ?, ?, ?, NULL, 0, ?, ?, NULL)
            "#,
        )
        .bind(id)
        .bind(content)
        .bind(content_preview)
        .bind(&now)
        .bind(character_count)
        .bind(word_count)
        .execute(&self.pool)
        .await?;

        // Return the created item
        self.get_clipboard_item_by_id(id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)
    }

    /// Update the content of an existing clipboard item
    pub async fn update_clipboard_item_content(
        &self,
        id: &str,
        content: &str,
        content_preview: &str,
        character_count: i32,
        word_count: i32,
    ) -> Result<Option<ClipboardItemRow>, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE clipboard_items
            SET content = ?, content_preview = ?, character_count = ?, word_count = ?, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            "#,
        )
        .bind(content)
        .bind(content_preview)
        .bind(character_count)
        .bind(word_count)
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() > 0 {
            self.get_clipboard_item_by_id(id).await
        } else {
            Ok(None)
        }
    }

    /// Get a single clipboard item by ID
    pub async fn get_clipboard_item_by_id(
        &self,
        id: &str,
    ) -> Result<Option<ClipboardItemRow>, sqlx::Error> {
        sqlx::query_as::<_, ClipboardItemRow>(
            r#"
            SELECT id, content, content_preview, copied_at, source_app, is_pinned, character_count, word_count, updated_at
            FROM clipboard_items
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    // ============================================
    // Glossary Operations
    // ============================================

    /// Migrate from old glossary schema (sourceText/targetText/languages) to new (keyword/description)
    async fn migrate_glossary_schema(&self) -> Result<(), sqlx::Error> {
        // Check if old columns exist
        let has_old_schema: Option<(String,)> = sqlx::query_as(
            "SELECT name FROM pragma_table_info('glossary_entries') WHERE name = 'source_text'",
        )
        .fetch_optional(&self.pool)
        .await?;

        if has_old_schema.is_some() {
            // Old schema exists, need to migrate
            // Create new table with new schema
            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS glossary_entries_new (
                    id TEXT PRIMARY KEY,
                    keyword TEXT NOT NULL UNIQUE,
                    description TEXT NOT NULL,
                    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    usage_count INTEGER NOT NULL DEFAULT 0
                )
                "#,
            )
            .execute(&self.pool)
            .await?;

            // Migrate data: combine source_text as keyword, and create description from target_text + note
            sqlx::query(
                r#"
                INSERT OR IGNORE INTO glossary_entries_new (id, keyword, description, created_at, updated_at, usage_count)
                SELECT 
                    id, 
                    source_text, 
                    CASE 
                        WHEN note IS NOT NULL AND note != '' 
                        THEN target_text || ' - ' || note
                        ELSE target_text
                    END,
                    created_at, 
                    updated_at, 
                    usage_count
                FROM glossary_entries
                "#,
            )
            .execute(&self.pool)
            .await?;

            // Drop old table and rename new
            sqlx::query("DROP TABLE glossary_entries")
                .execute(&self.pool)
                .await?;

            sqlx::query("ALTER TABLE glossary_entries_new RENAME TO glossary_entries")
                .execute(&self.pool)
                .await?;

            // Recreate index
            sqlx::query(
                "CREATE INDEX IF NOT EXISTS idx_glossary_keyword ON glossary_entries(keyword)",
            )
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    pub async fn get_glossary_entries(
        &self,
        search_query: Option<&str>,
        sort_by: &str,
        sort_order: &str,
    ) -> Result<Vec<GlossaryEntryRow>, sqlx::Error> {
        // Whitelist-validated ORDER BY to prevent SQL injection
        let column = match sort_by {
            "usageCount" => "usage_count",
            "createdAt" => "created_at",
            "keyword" => "keyword",
            other => {
                log::warn!("Invalid sort_by value '{}', defaulting to 'keyword'", other);
                "keyword"
            }
        };
        let direction = match sort_order {
            "desc" | "DESC" => "DESC",
            _ => "ASC",
        };
        let order_clause = format!("{} {}", column, direction);

        let entries = if let Some(query) = search_query {
            let search_pattern = format!("%{}%", query);
            sqlx::query_as::<_, GlossaryEntryRow>(&format!(
                r#"
                SELECT id, keyword, description, created_at, updated_at, usage_count
                FROM glossary_entries
                WHERE keyword LIKE ? OR description LIKE ?
                ORDER BY {}
                "#,
                order_clause
            ))
            .bind(&search_pattern)
            .bind(&search_pattern)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, GlossaryEntryRow>(&format!(
                r#"
                SELECT id, keyword, description, created_at, updated_at, usage_count
                FROM glossary_entries
                ORDER BY {}
                "#,
                order_clause
            ))
            .fetch_all(&self.pool)
            .await?
        };

        Ok(entries)
    }

    pub async fn insert_glossary_entry(&self, entry: &GlossaryEntryRow) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO glossary_entries (id, keyword, description, created_at, updated_at, usage_count)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&entry.id)
        .bind(&entry.keyword)
        .bind(&entry.description)
        .bind(&entry.created_at)
        .bind(&entry.updated_at)
        .bind(entry.usage_count)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn upsert_glossary_entry(
        &self,
        keyword: &str,
        description: &str,
    ) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        let id = uuid::Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO glossary_entries (id, keyword, description, created_at, updated_at, usage_count)
            VALUES (?, ?, ?, ?, ?, 0)
            ON CONFLICT(keyword) DO UPDATE SET
                description = excluded.description,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(&id)
        .bind(keyword)
        .bind(description)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_glossary_entry(
        &self,
        id: &str,
        keyword: Option<&str>,
        description: Option<&str>,
    ) -> Result<Option<GlossaryEntryRow>, sqlx::Error> {
        if let Some(kw) = keyword {
            sqlx::query("UPDATE glossary_entries SET keyword = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                .bind(kw)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(desc) = description {
            sqlx::query(
                "UPDATE glossary_entries SET description = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            )
            .bind(desc)
            .bind(id)
            .execute(&self.pool)
            .await?;
        }

        let entry = sqlx::query_as::<_, GlossaryEntryRow>(
            "SELECT id, keyword, description, created_at, updated_at, usage_count FROM glossary_entries WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(entry)
    }

    pub async fn delete_glossary_entry(&self, id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM glossary_entries WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Find glossary entries whose keywords appear in the given text
    /// This is language-agnostic - LLM will use the descriptions to translate appropriately
    pub async fn find_glossary_matches(
        &self,
        text: &str,
    ) -> Result<Vec<GlossaryEntryRow>, sqlx::Error> {
        // Find all glossary entries where keyword appears in the source text (case-insensitive)
        let text_lower = text.to_lowercase();
        sqlx::query_as::<_, GlossaryEntryRow>(
            r#"
            SELECT id, keyword, description, created_at, updated_at, usage_count
            FROM glossary_entries
            WHERE LOWER(?) LIKE '%' || LOWER(keyword) || '%'
            "#,
        )
        .bind(&text_lower)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn increment_glossary_usage(&self, ids: &[String]) -> Result<(), sqlx::Error> {
        for id in ids {
            sqlx::query("UPDATE glossary_entries SET usage_count = usage_count + 1 WHERE id = ?")
                .bind(id)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    // ============================================
    // Translation Cache Operations
    // ============================================

    pub async fn find_cached_translation(
        &self,
        source_text: &str,
        source_language: &str,
        target_language: &str,
        cache_days: i32,
    ) -> Result<Option<TranslationRow>, sqlx::Error> {
        sqlx::query_as::<_, TranslationRow>(
            r#"
            SELECT id, source_text, translated_text, source_language, target_language, model, created_at, glossary_used, input_tokens, output_tokens
            FROM translations
            WHERE source_text = ? AND source_language = ? AND target_language = ?
              AND created_at > datetime('now', ? || ' days')
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(source_text)
        .bind(source_language)
        .bind(target_language)
        .bind(-cache_days)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn insert_translation(
        &self,
        translation: &TranslationRow,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO translations (id, source_text, translated_text, source_language, target_language, model, created_at, glossary_used, input_tokens, output_tokens)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&translation.id)
        .bind(&translation.source_text)
        .bind(&translation.translated_text)
        .bind(&translation.source_language)
        .bind(&translation.target_language)
        .bind(&translation.model)
        .bind(&translation.created_at)
        .bind(&translation.glossary_used)
        .bind(translation.input_tokens)
        .bind(translation.output_tokens)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn cleanup_expired_translations(&self, cache_days: i32) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM translations WHERE created_at < datetime('now', ? || ' days')",
        )
        .bind(-cache_days)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    // ============================================
    // Settings Operations
    // ============================================

    pub async fn get_settings(&self) -> Result<UserSettingsRow, sqlx::Error> {
        sqlx::query_as::<_, UserSettingsRow>(
            r#"
            SELECT id, max_history_count, preferred_model, auto_detect_language, double_press_interval,
                   translation_cache_days, show_source_app, popup_position, launch_at_login, paste_delay_ms, api_key, updated_at
            FROM user_settings
            WHERE id = 'default'
            "#,
        )
        .fetch_one(&self.pool)
        .await
    }

    pub async fn update_settings(&self, settings: &UserSettingsRow) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE user_settings SET
                max_history_count = ?,
                preferred_model = ?,
                auto_detect_language = ?,
                double_press_interval = ?,
                translation_cache_days = ?,
                show_source_app = ?,
                popup_position = ?,
                launch_at_login = ?,
                paste_delay_ms = ?,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = 'default'
            "#,
        )
        .bind(settings.max_history_count)
        .bind(&settings.preferred_model)
        .bind(settings.auto_detect_language)
        .bind(settings.double_press_interval)
        .bind(settings.translation_cache_days)
        .bind(settings.show_source_app)
        .bind(&settings.popup_position)
        .bind(settings.launch_at_login)
        .bind(settings.paste_delay_ms)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ============================================
    // API Key Operations
    // ============================================

    pub async fn get_api_key(&self) -> Result<Option<String>, sqlx::Error> {
        let row: (Option<String>,) =
            sqlx::query_as("SELECT api_key FROM user_settings WHERE id = 'default'")
                .fetch_one(&self.pool)
                .await?;

        Ok(row.0)
    }

    pub async fn set_api_key(&self, api_key: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE user_settings SET api_key = ?, updated_at = CURRENT_TIMESTAMP WHERE id = 'default'"
        )
        .bind(api_key)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_api_key(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE user_settings SET api_key = NULL, updated_at = CURRENT_TIMESTAMP WHERE id = 'default'"
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub fn validate_api_key_format(api_key: &str) -> bool {
        // Claude API keys typically start with "sk-ant-" and have a specific length
        api_key.starts_with("sk-ant-") && api_key.len() > 20
    }

    // ============================================
    // Monitor Window Size Operations
    // ============================================

    /// Get saved window width for a specific monitor
    pub async fn get_monitor_window_width(
        &self,
        monitor_key: &str,
    ) -> Result<Option<i32>, sqlx::Error> {
        let row: Option<(i32,)> =
            sqlx::query_as("SELECT window_width FROM monitor_window_sizes WHERE monitor_key = ?")
                .bind(monitor_key)
                .fetch_optional(&self.pool)
                .await?;

        Ok(row.map(|(width,)| width))
    }

    /// Save window width for a specific monitor
    pub async fn save_monitor_window_width(
        &self,
        monitor_key: &str,
        width: i32,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO monitor_window_sizes (monitor_key, window_width, updated_at)
            VALUES (?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(monitor_key) DO UPDATE SET
                window_width = excluded.window_width,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(monitor_key)
        .bind(width)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

// ============================================
// Row Types
// ============================================

#[derive(Debug, sqlx::FromRow)]
pub struct ClipboardItemRow {
    pub id: String,
    pub content: String,
    pub content_preview: String,
    pub copied_at: String,
    pub source_app: Option<String>,
    pub is_pinned: i32,
    pub character_count: Option<i32>,
    pub word_count: Option<i32>,
    pub updated_at: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct GlossaryEntryRow {
    pub id: String,
    pub keyword: String,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
    pub usage_count: i32,
}

#[derive(Debug, sqlx::FromRow)]
pub struct TranslationRow {
    pub id: String,
    pub source_text: String,
    pub translated_text: String,
    pub source_language: String,
    pub target_language: String,
    pub model: String,
    pub created_at: String,
    pub glossary_used: Option<String>,
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct UserSettingsRow {
    pub id: String,
    pub max_history_count: i32,
    pub preferred_model: String,
    pub auto_detect_language: i32,
    pub double_press_interval: i32,
    pub translation_cache_days: i32,
    pub show_source_app: i32,
    pub popup_position: String,
    pub launch_at_login: i32,
    pub paste_delay_ms: i32,
    pub api_key: Option<String>,
    pub updated_at: String,
}

#[cfg(test)]
mod tests {
    use super::{ClipboardItemRow, Database};
    use chrono::Utc;

    fn test_db_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("transclip-test-{}.db", uuid::Uuid::new_v4()))
    }

    #[tokio::test]
    async fn cleanup_old_clipboard_items_preserves_pinned_items() {
        let path = test_db_path();
        let db = Database::new(&path).await.expect("db should initialize");

        let pinned = ClipboardItemRow {
            id: "pinned".to_string(),
            content: "pinned".to_string(),
            content_preview: "pinned".to_string(),
            copied_at: Utc::now().to_rfc3339(),
            source_app: None,
            is_pinned: 1,
            character_count: Some(6),
            word_count: Some(1),
            updated_at: None,
        };
        db.insert_clipboard_item(&pinned)
            .await
            .expect("should insert pinned item");

        for idx in 0..5 {
            let content = format!("unpinned-{idx}");
            let item = ClipboardItemRow {
                id: format!("unpinned-{idx}"),
                content: content.clone(),
                content_preview: content,
                copied_at: Utc::now().to_rfc3339(),
                source_app: None,
                is_pinned: 0,
                character_count: Some(10),
                word_count: Some(1),
                updated_at: None,
            };
            db.insert_clipboard_item(&item)
                .await
                .expect("should insert unpinned item");
        }

        db.cleanup_old_clipboard_items(3)
            .await
            .expect("cleanup should succeed");

        let (items, total) = db
            .get_clipboard_history(50, 0, None)
            .await
            .expect("should fetch history");

        assert_eq!(total, 3);
        assert_eq!(items.len(), 3);
        assert!(items
            .iter()
            .any(|item| item.id == "pinned" && item.is_pinned == 1));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn validate_api_key_format_requires_expected_prefix_and_length() {
        assert!(Database::validate_api_key_format(
            "sk-ant-this-is-a-valid-looking-key"
        ));
        assert!(!Database::validate_api_key_format("sk-test-short"));
        assert!(!Database::validate_api_key_format("invalid-prefix-value"));
    }
}
