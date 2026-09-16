/// Azure AD app (client) ID for Microsoft OAuth2 sign-in. Setup steps are in
/// README.md. Env var, then build-time value (see build.rs), then a placeholder.
pub fn ms_client_id() -> String {
    std::env::var("STRATA_MS_CLIENT_ID")
        .ok()
        .or_else(|| option_env!("BUILTIN_STRATA_MS_CLIENT_ID").map(str::to_string))
        .unwrap_or_else(|| "00000000-0000-0000-0000-000000000000".to_string())
}

/// CurseForge Core API key. Apply at
/// https://support.curseforge.com/support/solutions/articles/9000208346.
pub fn curseforge_api_key() -> Option<String> {
    std::env::var("STRATA_CURSEFORGE_API_KEY")
        .ok()
        .or_else(|| option_env!("BUILTIN_STRATA_CURSEFORGE_API_KEY").map(str::to_string))
        .filter(|s| !s.is_empty())
}

/// GitHub repo ("owner/repo") releases and repo-hosted data (the cape
/// catalog, the update manifest) are fetched from. Env var override is for
/// testing against a fork; not a secret, no build-time path needed.
pub fn update_repo() -> Option<String> {
    Some(std::env::var("STRATA_UPDATE_REPO").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| "nihalantiir/stratalauncher".to_string()))
}

/// Pastebin.com API key for the Logs page's upload option, from
/// https://pastebin.com/doc_api (My Account -> API Access).
pub fn pastebin_api_key() -> Option<String> {
    std::env::var("STRATA_PASTEBIN_API_KEY")
        .ok()
        .or_else(|| option_env!("BUILTIN_STRATA_PASTEBIN_API_KEY").map(str::to_string))
        .filter(|s| !s.is_empty())
}
