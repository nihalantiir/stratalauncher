/// Bakes real API keys into release builds without ever putting them in git.
/// `secrets.release.env` (gitignored) only exists on a release machine; a fresh clone still compiles fine with `option_env!` resolving to `None`.
fn embed_release_secrets() {
    let path = "secrets.release.env";
    println!("cargo:rerun-if-changed={path}");
    let Ok(contents) = std::fs::read_to_string(path) else { return };

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            println!("cargo:rustc-env=BUILTIN_{}={}", key.trim(), value.trim());
        }
    }
}

fn main() {
    println!("cargo:rerun-if-changed=icons");
    embed_release_secrets();
    tauri_build::build()
}
