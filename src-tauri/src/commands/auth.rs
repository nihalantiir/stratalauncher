use crate::auth::{self, keychain, offline};
use crate::db::models::{Account, AccountKind};
use crate::db::AccountsRepo;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn start_microsoft_login(app: AppHandle, state: State<'_, AppState>) -> AppResult<Account> {
    let (account, refresh_token) =
        auth::sign_in_with_device_code(&app, &state.http, &state.cancel_login).await?;

    keychain::store_refresh_token(&account.id, &refresh_token)?;

    let conn = state.db.0.lock().unwrap();
    AccountsRepo::upsert(&conn, &account)?;
    AccountsRepo::set_active(&conn, &account.id)?;
    Ok(account)
}

#[tauri::command]
pub fn cancel_microsoft_login(state: State<'_, AppState>) -> AppResult<()> {
    state.cancel_login.store(true, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
pub fn create_offline_profile(username: String, state: State<'_, AppState>) -> AppResult<Account> {
    if !offline::is_valid_username(&username) {
        return Err(AppError::Auth(
            "Usernames must be 3-16 characters: letters, numbers, and underscores only.".into(),
        ));
    }

    let account = Account {
        id: offline::offline_uuid(&username),
        kind: AccountKind::Offline,
        username,
        mc_uuid: None,
        skin_url: None,
        skin_variant: None,
        is_active: true,
        created_at: chrono::Utc::now().to_rfc3339(),
        last_used_at: Some(chrono::Utc::now().to_rfc3339()),
    };

    let conn = state.db.0.lock().unwrap();
    AccountsRepo::upsert(&conn, &account)?;
    AccountsRepo::set_active(&conn, &account.id)?;
    Ok(account)
}

#[tauri::command]
pub fn list_accounts(state: State<'_, AppState>) -> AppResult<Vec<Account>> {
    let conn = state.db.0.lock().unwrap();
    AccountsRepo::list(&conn)
}

#[tauri::command]
pub fn get_active_account(state: State<'_, AppState>) -> AppResult<Option<Account>> {
    let conn = state.db.0.lock().unwrap();
    AccountsRepo::active(&conn)
}

#[tauri::command]
pub fn set_active_account(id: String, state: State<'_, AppState>) -> AppResult<()> {
    let conn = state.db.0.lock().unwrap();
    AccountsRepo::set_active(&conn, &id)
}

#[tauri::command]
pub fn remove_account(id: String, state: State<'_, AppState>) -> AppResult<()> {
    let conn = state.db.0.lock().unwrap();
    let account = AccountsRepo::get(&conn, &id)?;
    if let Some(account) = account {
        if account.kind == AccountKind::Microsoft {
            keychain::delete_refresh_token(&id)?;
        }
    }
    AccountsRepo::remove(&conn, &id)
}
