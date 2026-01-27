use crate::database::GlossaryEntryRow;
use crate::AppState;
use tauri::State;

use super::types::{
    DeleteResponse, ErrorDetail, ExportGlossaryResponse, GlossaryEntryResponse,
    GlossaryListResponse, ImportError, ImportGlossaryResponse,
};

#[tauri::command]
pub async fn get_glossary_entries(
    state: State<'_, AppState>,
    search_query: Option<String>,
    sort_by: Option<String>,
    sort_order: Option<String>,
) -> Result<GlossaryListResponse, String> {
    let db = state.db.lock().await;
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
    if keyword.is_empty() || keyword.len() > 100 {
        return Err("Keyword must be 1-100 characters".to_string());
    }

    if description.is_empty() || description.len() > 500 {
        return Err("Description must be 1-500 characters".to_string());
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

    let db = state.db.lock().await;
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
        if kw.is_empty() || kw.len() > 100 {
            return Err("Keyword must be 1-100 characters".to_string());
        }
    }
    if let Some(ref desc) = description {
        if desc.is_empty() || desc.len() > 500 {
            return Err("Description must be 1-500 characters".to_string());
        }
    }

    let db = state.db.lock().await;
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
    let db = state.db.lock().await;
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
    _state: State<'_, AppState>,
    _file_path: String,
    _format: String,
    _overwrite: bool,
) -> Result<ImportGlossaryResponse, String> {
    // TODO: Implement CSV/JSON import
    Ok(ImportGlossaryResponse {
        imported: 0,
        skipped: 0,
        errors: vec![ImportError {
            line: 0,
            message: "Import not yet implemented".to_string(),
        }],
    })
}

#[tauri::command]
pub async fn export_glossary(
    _state: State<'_, AppState>,
    _file_path: String,
    _format: String,
) -> Result<ExportGlossaryResponse, String> {
    // TODO: Implement CSV/JSON export
    Ok(ExportGlossaryResponse {
        success: false,
        exported_count: 0,
    })
}
