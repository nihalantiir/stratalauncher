#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE};

struct Args {
    pid: u32,
    new_exe: PathBuf,
    target_exe: PathBuf,
}

fn parse_args() -> Result<Args, String> {
    let mut pid: Option<u32> = None;
    let mut new_exe: Option<PathBuf> = None;
    let mut target_exe: Option<PathBuf> = None;

    let mut argv = std::env::args().skip(1);
    while let Some(flag) = argv.next() {
        match flag.as_str() {
            "--pid" => {
                let value = argv.next().ok_or("--pid requires a value")?;
                pid = Some(value.parse::<u32>().map_err(|e| format!("invalid --pid value '{value}': {e}"))?);
            }
            "--new-exe" => {
                let value = argv.next().ok_or("--new-exe requires a value")?;
                new_exe = Some(PathBuf::from(value));
            }
            "--target-exe" => {
                let value = argv.next().ok_or("--target-exe requires a value")?;
                target_exe = Some(PathBuf::from(value));
            }
            other => {
                return Err(format!("unrecognized argument '{other}'"));
            }
        }
    }

    let pid = pid.ok_or("missing required --pid <u32>")?;
    let new_exe = new_exe.ok_or("missing required --new-exe <path>")?;
    let target_exe = target_exe.ok_or("missing required --target-exe <path>")?;

    Ok(Args { pid, new_exe, target_exe })
}

fn print_usage() {
    eprintln!(
        "usage: strata-updater --pid <u32> --new-exe <path> --target-exe <path>"
    );
}

fn log_failure(message: &str) {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let line = format!("[{timestamp}] {message}");

    eprintln!("{line}");

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(dir) = current_exe.parent() {
            let log_path = dir.join("updater.log");
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
                let _ = writeln!(file, "{line}");
            }
        }
    }
}

fn wait_for_caller_exit(pid: u32) {
    let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid) };
    let handle = match handle {
        Ok(h) => h,
        // Caller process is already gone (or inaccessible); nothing to wait on.
        Err(_) => return,
    };

    unsafe {
        WaitForSingleObject(handle, 30_000);
    }

    let _ = unsafe { CloseHandle(handle) };
}

fn old_path_for(target_exe: &Path) -> PathBuf {
    let mut name = target_exe.as_os_str().to_owned();
    name.push(".old");
    PathBuf::from(name)
}

/// Renames target_exe aside, moves new_exe into place, retrying since a
/// fresh file lock is transient; rolls back if the second rename fails.
fn swap_files(new_exe: &Path, target_exe: &Path) -> Result<(), String> {
    let old_exe = old_path_for(target_exe);
    const MAX_ATTEMPTS: u32 = 10;
    const RETRY_DELAY: Duration = Duration::from_millis(300);

    let mut last_error = String::new();

    for attempt in 1..=MAX_ATTEMPTS {
        if let Err(e) = std::fs::rename(target_exe, &old_exe) {
            last_error = format!("attempt {attempt}: failed to rename target exe aside: {e}");
            std::thread::sleep(RETRY_DELAY);
            continue;
        }

        if let Err(e) = std::fs::rename(new_exe, target_exe) {
            last_error = format!("attempt {attempt}: failed to move new exe into place: {e}");
            // Best-effort rollback so the app isn't left with no exe at all.
            let _ = std::fs::rename(&old_exe, target_exe);
            std::thread::sleep(RETRY_DELAY);
            continue;
        }

        // Swap succeeded; leftover .old removal is best-effort.
        let _ = std::fs::remove_file(&old_exe);
        return Ok(());
    }

    Err(format!("exhausted {MAX_ATTEMPTS} swap attempts; last error: {last_error}"))
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            print_usage();
            log_failure(&format!("argument error: {e}"));
            std::process::exit(1);
        }
    };

    wait_for_caller_exit(args.pid);

    std::thread::sleep(Duration::from_millis(400));

    if let Err(e) = swap_files(&args.new_exe, &args.target_exe) {
        log_failure(&e);
        std::process::exit(1);
    }

    if let Err(e) = std::process::Command::new(&args.target_exe).spawn() {
        log_failure(&format!("file swap succeeded but relaunch failed: {e}"));
    }
}
