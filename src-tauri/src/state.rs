use crate::auth::CachedSession;
use crate::db::Db;
use crate::error::AppResult;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

pub struct AppState {
    pub db: Db,
    pub http: reqwest::Client,
    pub cancel_login: AtomicBool,
    /// instanceId -> the running game process's OS pid. Purely in-memory;
    /// doesn't need to survive a Strata restart.
    pub running: Mutex<HashMap<String, u32>>,
    /// accountId -> last-obtained live session, reused instead of
    /// re-running the full MSA/Xbox/XSTS/Minecraft chain. In-memory only.
    pub session_cache: Mutex<HashMap<String, CachedSession>>,
}

impl AppState {
    pub fn new() -> AppResult<Self> {
        Ok(Self {
            db: Db::open()?,
            http: reqwest::Client::builder()
                .user_agent(concat!("StrataLauncher/", env!("CARGO_PKG_VERSION")))
                .build()
                .expect("failed to build reqwest client"),
            cancel_login: AtomicBool::new(false),
            running: Mutex::new(HashMap::new()),
            session_cache: Mutex::new(HashMap::new()),
        })
    }
}
