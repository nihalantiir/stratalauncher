//! Refresh tokens live only in the OS keychain, never on disk in plaintext.

use crate::error::{AppError, AppResult};
use keyring::Entry;

const SERVICE: &str = "com.strata.launcher";

fn entry(account_id: &str) -> AppResult<Entry> {
    Entry::new(SERVICE, account_id).map_err(AppError::from)
}

pub fn store_refresh_token(account_id: &str, refresh_token: &str) -> AppResult<()> {
    entry(account_id)?.set_password(refresh_token)?;
    Ok(())
}

pub fn load_refresh_token(account_id: &str) -> AppResult<Option<String>> {
    match entry(account_id)?.get_password() {
        Ok(secret) => Ok(Some(secret)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AppError::from(e)),
    }
}

pub fn delete_refresh_token(account_id: &str) -> AppResult<()> {
    match entry(account_id)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AppError::from(e)),
    }
}
