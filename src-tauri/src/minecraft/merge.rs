//! Implements the same "inheritsFrom" merge the vanilla launcher uses for
//! loader profile JSONs: the patch is layered onto the vanilla JSON.

/// `base` is the vanilla version JSON; `patch` is the loader's profile JSON.
pub fn merge_inherited(mut base: serde_json::Value, patch: serde_json::Value) -> serde_json::Value {
    let Some(patch_obj) = patch.as_object() else {
        return base;
    };
    let base_obj = base
        .as_object_mut()
        .expect("vanilla version JSON must be a JSON object");

    // Loader arguments come first, followed by the inherited vanilla ones.
    if let Some(patch_args) = patch_obj.get("arguments").and_then(|v| v.as_object()) {
        let base_args = base_obj
            .entry("arguments")
            .or_insert_with(|| serde_json::json!({}))
            .as_object_mut()
            .expect("arguments must be an object");

        for key in ["game", "jvm"] {
            if let Some(patch_arr) = patch_args.get(key).and_then(|v| v.as_array()) {
                let mut combined = patch_arr.clone();
                if let Some(existing) = base_args.get(key).and_then(|v| v.as_array()) {
                    combined.extend(existing.clone());
                }
                base_args.insert(key.to_string(), serde_json::Value::Array(combined));
            }
        }
    }

    // Loader libraries (loader jar, mappings, ASM, etc.) go first on the
    // classpath, then the inherited vanilla libraries.
    if let Some(patch_libs) = patch_obj.get("libraries").and_then(|v| v.as_array()) {
        let mut combined = patch_libs.clone();
        if let Some(base_libs) = base_obj.get("libraries").and_then(|v| v.as_array()) {
            combined.extend(base_libs.clone());
        }
        base_obj.insert("libraries".to_string(), serde_json::Value::Array(combined));
    }

    // The loader owns the entry point; everything else (downloads,
    // assetIndex, type, javaVersion...) stays inherited from vanilla.
    for key in ["mainClass", "id"] {
        if let Some(v) = patch_obj.get(key) {
            base_obj.insert(key.to_string(), v.clone());
        }
    }

    base
}
