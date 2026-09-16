//! Per-instance log listing/reading (`logs/latest.log` + rotated `*.log.gz`)
//! and a shared log4j-style line parser for both Minecraft and launcher logs.

use crate::error::{AppError, AppResult};
use crate::timeutil::system_time_to_rfc3339;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogFileMeta {
    pub filename: String,
    pub label: String,
    pub size_bytes: u64,
    pub modified: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogLine {
    pub time: Option<String>,
    pub thread: Option<String>,
    pub level: String,
    pub logger: Option<String>,
    pub message: String,
}

fn logs_dir(instance_dir: &Path) -> PathBuf {
    instance_dir.join("logs")
}

/// `latest.log` -> "Latest session"; vanilla's rotated `2026-09-11-2.log.gz`
/// -> "2026-09-11, session 2"; anything else falls back to the raw filename.
fn label_for(filename: &str) -> String {
    if filename == "latest.log" {
        return "Latest session".to_string();
    }
    let stem = filename.trim_end_matches(".log.gz").trim_end_matches(".log");
    match stem.rsplit_once('-') {
        Some((date, n)) if date.len() == 10 && n.chars().all(|c| c.is_ascii_digit()) => {
            format!("{date}, session {n}")
        }
        _ => filename.to_string(),
    }
}

pub fn list_log_files(instance_dir: &Path) -> AppResult<Vec<LogFileMeta>> {
    let dir = logs_dir(instance_dir);
    std::fs::create_dir_all(&dir)?;

    let mut files = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let filename = entry.file_name().to_string_lossy().to_string();
        if !(filename.ends_with(".log") || filename.ends_with(".log.gz")) {
            continue;
        }
        let meta = entry.metadata()?;
        let modified = meta.modified().ok().and_then(system_time_to_rfc3339);
        files.push(LogFileMeta {
            filename: filename.clone(),
            label: label_for(&filename),
            size_bytes: meta.len(),
            modified,
        });
    }

    files.sort_by(|a, b| {
        let a_latest = a.filename == "latest.log";
        let b_latest = b.filename == "latest.log";
        b_latest.cmp(&a_latest).then_with(|| b.modified.cmp(&a.modified))
    });
    Ok(files)
}

const MAX_LOG_BYTES: usize = 2 * 1024 * 1024;

pub fn read_log_text(instance_dir: &Path, filename: &str) -> AppResult<String> {
    let path = logs_dir(instance_dir).join(filename);
    let bytes = if filename.ends_with(".gz") {
        let file = std::fs::File::open(&path)?;
        let mut decoder = flate2::read::GzDecoder::new(file);
        let mut buf = Vec::new();
        decoder.read_to_end(&mut buf)?;
        buf
    } else {
        std::fs::read(&path)?
    };

    let text = String::from_utf8_lossy(&bytes).into_owned();
    if text.len() > MAX_LOG_BYTES {
        Ok(text[text.len() - MAX_LOG_BYTES..].to_string())
    } else {
        Ok(text)
    }
}

/// Parses real log4j-pattern lines: `[HH:MM:SS] [Thread/LEVEL] (Logger): message`.
/// Lines that don't match are folded into the previous entry as continuation text.
pub fn parse_log_lines(text: &str) -> Vec<LogLine> {
    let mut lines: Vec<LogLine> = Vec::new();
    for raw in text.lines() {
        if let Some(parsed) = parse_one_line(raw) {
            lines.push(parsed);
        } else if let Some(last) = lines.last_mut() {
            last.message.push('\n');
            last.message.push_str(raw);
        } else if !raw.trim().is_empty() {
            lines.push(LogLine {
                time: None,
                thread: None,
                level: "UNKNOWN".into(),
                logger: None,
                message: raw.to_string(),
            });
        }
    }
    lines
}

fn parse_one_line(line: &str) -> Option<LogLine> {
    let rest = line.strip_prefix('[')?;
    let (time, rest) = rest.split_once(']')?;
    let rest = rest.trim_start().strip_prefix('[')?;
    let (thread_level, rest) = rest.split_once(']')?;
    let (thread, level) = thread_level.rsplit_once('/')?;
    let mut rest = rest.trim_start();

    // A `(Logger):` prefix is only real when `)` is immediately followed by
    // `:`, otherwise it's an ordinary message starting with a parenthesis.
    let mut logger = None;
    if let Some(after_paren) = rest.strip_prefix('(') {
        if let Some((candidate, after)) = after_paren.split_once(')') {
            if after.starts_with(':') {
                logger = Some(candidate.to_string());
                rest = after;
            }
        }
    }

    let message = rest.strip_prefix(':').unwrap_or(rest).trim_start().to_string();

    Some(LogLine {
        time: Some(time.to_string()),
        thread: Some(thread.to_string()),
        level: level.to_string(),
        logger,
        message,
    })
}

/// Where a log's text can be pasted for sharing. mclo.gs needs no
/// credentials; Pastebin needs one (see `config.rs`) and is offered once configured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogUploadTarget {
    Mclogs,
    Pastebin,
}

/// The upload targets actually usable right now.
pub fn available_upload_targets() -> Vec<LogUploadTarget> {
    let mut targets = vec![LogUploadTarget::Mclogs];
    if crate::config::pastebin_api_key().is_some() {
        targets.push(LogUploadTarget::Pastebin);
    }
    targets
}

pub async fn upload_log(http: &reqwest::Client, content: &str, target: LogUploadTarget) -> AppResult<String> {
    match target {
        LogUploadTarget::Mclogs => upload_log_mclogs(http, content).await,
        LogUploadTarget::Pastebin => upload_log_pastebin(http, content).await,
    }
}

/// The same public, no-auth log host most third-party launchers use for
/// support requests.
async fn upload_log_mclogs(http: &reqwest::Client, content: &str) -> AppResult<String> {
    #[derive(serde::Deserialize)]
    struct MclogsResponse {
        success: bool,
        url: Option<String>,
        error: Option<String>,
    }

    let resp: MclogsResponse = http
        .post("https://api.mclo.gs/1/log")
        .form(&[("content", content)])
        .send()
        .await?
        .json()
        .await?;

    if resp.success {
        resp.url.ok_or_else(|| AppError::Other("mclo.gs did not return a URL".into()))
    } else {
        Err(AppError::Other(resp.error.unwrap_or_else(|| "log upload failed".into())))
    }
}

/// Pastebin's API returns the paste URL as plain text on success, or a
/// plain-text `Bad API request, ...` message on failure; never JSON.
async fn upload_log_pastebin(http: &reqwest::Client, content: &str) -> AppResult<String> {
    let api_key = crate::config::pastebin_api_key()
        .ok_or_else(|| AppError::Other("Pastebin isn't configured.".into()))?;

    let body = http
        .post("https://pastebin.com/api/api_post.php")
        .form(&[
            ("api_dev_key", api_key.as_str()),
            ("api_option", "paste"),
            ("api_paste_code", content),
            ("api_paste_name", "Strata Launcher log"),
            ("api_paste_expire_date", "1M"),
        ])
        .send()
        .await?
        .text()
        .await?;

    if body.starts_with("http") {
        Ok(body)
    } else {
        Err(AppError::Other(body))
    }
}

/// mclo.gs's advantage over a plain text host: it parses the log and
/// reports detected problems. Shapes match `GET /1/insights/{id}` exactly.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InsightSolution {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InsightProblem {
    pub message: String,
    #[serde(default)]
    pub solutions: Vec<InsightSolution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InsightInfo {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogInsights {
    pub title: String,
    pub problems: Vec<InsightProblem>,
    pub information: Vec<InsightInfo>,
}

#[derive(Debug, Deserialize)]
struct RawAnalysis {
    problems: Vec<InsightProblem>,
    information: Vec<InsightInfo>,
}

#[derive(Debug, Deserialize)]
struct RawInsightsResponse {
    title: String,
    analysis: RawAnalysis,
}

pub async fn fetch_mclogs_insights(http: &reqwest::Client, id: &str) -> AppResult<LogInsights> {
    let raw: RawInsightsResponse = http
        .get(format!("https://api.mclo.gs/1/insights/{id}"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(LogInsights {
        title: raw.title,
        problems: raw.analysis.problems,
        information: raw.analysis.information,
    })
}
