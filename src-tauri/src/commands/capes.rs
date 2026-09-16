use crate::cape_catalog::{self, CapeEntry};
use crate::state::AppState;
use tauri::State;

/// `None` when there's nothing remote to offer yet; the frontend falls
/// back to its own bundled list (`src/lib/knownCapes.js`).
#[tauri::command]
pub async fn get_cape_catalog(state: State<'_, AppState>) -> Result<Option<Vec<CapeEntry>>, ()> {
    Ok(cape_catalog::get_catalog(&state.http).await)
}
