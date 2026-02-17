use crate::database::GlossaryEntryRow;
use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

use super::types::{
    DeleteResponse, ErrorDetail, ExportGlossaryResponse, GlossaryEntryResponse,
    GlossaryListResponse, ImportError, ImportGlossaryResponse,
};

#[derive(Debug, Deserialize, Serialize)]
struct GlossaryFileEntry {
    keyword: String,
    description: String,
}

#[derive(Debug, Deserialize)]
struct GlossaryJsonEnvelope {
    entries: Vec<GlossaryFileEntry>,
}

#[derive(Debug)]
enum ImportRow {
    Entry(GlossaryFileEntry),
    ParseError(String),
}

fn is_keyword_unique_violation(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(db_err) => db_err
            .message()
            .contains("UNIQUE constraint failed: glossary_entries.keyword"),
        _ => false,
    }
}

fn validate_keyword(keyword: &str) -> Result<(), String> {
    if keyword.is_empty() || keyword.len() > 100 {
        return Err("Keyword must be 1-100 characters".to_string());
    }
    Ok(())
}

fn validate_description(description: &str) -> Result<(), String> {
    if description.is_empty() || description.len() > 500 {
        return Err("Description must be 1-500 characters".to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn get_glossary_entries(
    state: State<'_, AppState>,
    search_query: Option<String>,
    sort_by: Option<String>,
    sort_order: Option<String>,
) -> Result<GlossaryListResponse, String> {
    let db = &state.db;
    let sort_by = sort_by.unwrap_or_else(|| "keyword".to_string());
    let sort_order = sort_order.unwrap_or_else(|| "asc".to_string());

    let entries = db
        .get_glossary_entries(search_query.as_deref(), &sort_by, &sort_order)
        .await
        .map_err(|e| e.to_string())?;

    let total = entries.len() as i64;

    Ok(GlossaryListResponse {
        entries: entries
            .into_iter()
            .map(GlossaryEntryResponse::from)
            .collect(),
        total,
    })
}

#[tauri::command]
pub async fn add_glossary_entry(
    state: State<'_, AppState>,
    keyword: String,
    description: String,
) -> Result<GlossaryEntryResponse, String> {
    // Validation
    validate_keyword(&keyword)?;
    validate_description(&description)?;

    let now = chrono::Utc::now().to_rfc3339();
    let entry = GlossaryEntryRow {
        id: uuid::Uuid::new_v4().to_string(),
        keyword,
        description,
        created_at: now.clone(),
        updated_at: now,
        usage_count: 0,
    };

    let db = &state.db;
    db.insert_glossary_entry(&entry)
        .await
        .map_err(|e| e.to_string())?;

    Ok(GlossaryEntryResponse::from(entry))
}

#[tauri::command]
pub async fn update_glossary_entry(
    state: State<'_, AppState>,
    id: String,
    keyword: Option<String>,
    description: Option<String>,
) -> Result<GlossaryEntryResponse, String> {
    // Validation
    if let Some(ref kw) = keyword {
        validate_keyword(kw)?;
    }
    if let Some(ref desc) = description {
        validate_description(desc)?;
    }

    let db = &state.db;
    let entry = db
        .update_glossary_entry(&id, keyword.as_deref(), description.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    entry
        .map(GlossaryEntryResponse::from)
        .ok_or_else(|| "Glossary entry not found".to_string())
}

#[tauri::command]
pub async fn delete_glossary_entry(
    state: State<'_, AppState>,
    id: String,
) -> Result<DeleteResponse, String> {
    let db = &state.db;
    let deleted = db
        .delete_glossary_entry(&id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(DeleteResponse {
        success: deleted,
        error: if deleted {
            None
        } else {
            Some(ErrorDetail {
                code: "NOT_FOUND".to_string(),
                message: "Glossary entry not found".to_string(),
            })
        },
    })
}

#[tauri::command]
pub async fn import_glossary(
    state: State<'_, AppState>,
    file_path: String,
    format: String,
    overwrite: bool,
) -> Result<ImportGlossaryResponse, String> {
    let format = format.to_lowercase();
    let mut rows: Vec<(i32, ImportRow)> = Vec::new();

    match format.as_str() {
        "csv" => {
            let mut reader = csv::ReaderBuilder::new()
                .flexible(true)
                .from_path(&file_path)
                .map_err(|e| format!("Failed to read CSV: {}", e))?;

            for (idx, record) in reader.deserialize::<GlossaryFileEntry>().enumerate() {
                match record {
                    Ok(entry) => rows.push((idx as i32 + 2, ImportRow::Entry(entry))),
                    Err(err) => {
                        rows.push((idx as i32 + 2, ImportRow::ParseError(err.to_string())));
                    }
                }
            }
        }
        "json" => {
            let content = std::fs::read_to_string(&file_path)
                .map_err(|e| format!("Failed to read JSON: {}", e))?;

            let parsed_entries = serde_json::from_str::<Vec<GlossaryFileEntry>>(&content)
                .or_else(|_| {
                    serde_json::from_str::<GlossaryJsonEnvelope>(&content)
                        .map(|envelope| envelope.entries)
                })
                .map_err(|e| format!("Failed to parse JSON: {}", e))?;

            for (idx, entry) in parsed_entries.into_iter().enumerate() {
                rows.push((idx as i32 + 1, ImportRow::Entry(entry)));
            }
        }
        _ => {
            return Err("Unsupported format. Use 'csv' or 'json'.".to_string());
        }
    }

    let db = &state.db;
    let mut imported = 0;
    let mut skipped = 0;
    let mut errors: Vec<ImportError> = Vec::new();

    for (line, row) in rows {
        let row = match row {
            ImportRow::Entry(entry) => entry,
            ImportRow::ParseError(message) => {
                errors.push(ImportError { line, message });
                continue;
            }
        };

        let keyword = row.keyword.trim().to_string();
        let description = row.description.trim().to_string();

        if let Err(message) =
            validate_keyword(&keyword).and_then(|_| validate_description(&description))
        {
            errors.push(ImportError { line, message });
            continue;
        }

        if overwrite {
            if let Err(err) = db.upsert_glossary_entry(&keyword, &description).await {
                errors.push(ImportError {
                    line,
                    message: format!("Database error: {}", err),
                });
            } else {
                imported += 1;
            }
            continue;
        }

        let now = chrono::Utc::now().to_rfc3339();
        let entry = GlossaryEntryRow {
            id: uuid::Uuid::new_v4().to_string(),
            keyword,
            description,
            created_at: now.clone(),
            updated_at: now,
            usage_count: 0,
        };

        match db.insert_glossary_entry(&entry).await {
            Ok(_) => imported += 1,
            Err(err) if is_keyword_unique_violation(&err) => skipped += 1,
            Err(err) => errors.push(ImportError {
                line,
                message: format!("Database error: {}", err),
            }),
        }
    }

    Ok(ImportGlossaryResponse {
        imported,
        skipped,
        errors,
    })
}

#[tauri::command]
pub async fn export_glossary(
    state: State<'_, AppState>,
    file_path: String,
    format: String,
) -> Result<ExportGlossaryResponse, String> {
    let format = format.to_lowercase();
    let db = &state.db;
    let entries = db
        .get_glossary_entries(None, "keyword", "asc")
        .await
        .map_err(|e| e.to_string())?;
    let exported_count = entries.len() as i32;

    match format.as_str() {
        "csv" => {
            let mut writer = csv::WriterBuilder::new()
                .from_path(&file_path)
                .map_err(|e| format!("Failed to create CSV: {}", e))?;

            writer
                .write_record(["keyword", "description"])
                .map_err(|e| format!("Failed to write CSV header: {}", e))?;

            for entry in entries {
                writer
                    .serialize(GlossaryFileEntry {
                        keyword: entry.keyword,
                        description: entry.description,
                    })
                    .map_err(|e| format!("Failed to write CSV row: {}", e))?;
            }

            writer
                .flush()
                .map_err(|e| format!("Failed to flush CSV: {}", e))?;
        }
        "json" => {
            let payload: Vec<GlossaryFileEntry> = entries
                .into_iter()
                .map(|entry| GlossaryFileEntry {
                    keyword: entry.keyword,
                    description: entry.description,
                })
                .collect();

            let content = serde_json::to_string_pretty(&payload)
                .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
            std::fs::write(&file_path, content)
                .map_err(|e| format!("Failed to write JSON: {}", e))?;
        }
        _ => return Err("Unsupported format. Use 'csv' or 'json'.".to_string()),
    }

    Ok(ExportGlossaryResponse {
        success: true,
        exported_count,
    })
}
