//! Evaluates the `rules` arrays Mojang's version JSON attaches to libraries
//! and launch arguments. Strata treats every feature flag as unset/false.

use serde_json::Value;
use std::collections::HashMap;

pub fn mojang_os_name() -> &'static str {
    match std::env::consts::OS {
        "macos" => "osx",
        "windows" => "windows",
        _ => "linux",
    }
}

pub fn mojang_arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86" => "x86",
        "x86_64" => "x86_64",
        "aarch64" => "arm64",
        other => other,
    }
}

/// `rules`: the raw JSON array from a library or argument entry, if present.
/// `features`: active feature flags (empty for Strata today).
pub fn rules_allow(rules: Option<&Vec<Value>>, features: &HashMap<String, bool>) -> bool {
    let Some(rules) = rules else { return true };
    let mut allowed = false;
    for rule in rules {
        if !rule_conditions_match(rule, features) {
            continue;
        }
        allowed = rule.get("action").and_then(Value::as_str) == Some("allow");
    }
    allowed
}

fn rule_conditions_match(rule: &Value, features: &HashMap<String, bool>) -> bool {
    if let Some(os) = rule.get("os") {
        if let Some(name) = os.get("name").and_then(Value::as_str) {
            if name != mojang_os_name() {
                return false;
            }
        }
        if let Some(arch) = os.get("arch").and_then(Value::as_str) {
            if arch != mojang_arch() {
                return false;
            }
        }
    }
    if let Some(required) = rule.get("features").and_then(Value::as_object) {
        for (key, want) in required {
            let want = want.as_bool().unwrap_or(false);
            let have = *features.get(key).unwrap_or(&false);
            if want != have {
                return false;
            }
        }
    }
    true
}
