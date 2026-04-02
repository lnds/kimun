/// Claude Code permissions installer for `km` commands.
///
/// Adds bash permission rules to `.claude/settings.local.json` so that
/// Claude Code can run `km` subcommands without prompting the user.
use std::fs;
use std::path::Path;

/// All `km` subcommand patterns that Claude Code should be allowed to run.
const KM_PERMISSIONS: &[&str] = &[
    "Bash(km loc*)",
    "Bash(km score*)",
    "Bash(km hal*)",
    "Bash(km cycom*)",
    "Bash(km cogcom*)",
    "Bash(km indent*)",
    "Bash(km mi *)",
    "Bash(km miv*)",
    "Bash(km dups*)",
    "Bash(km hotspots*)",
    "Bash(km knowledge*)",
    "Bash(km tc*)",
];

/// Install `km` bash permissions into `.claude/settings.local.json` at the
/// given project root. Merges with any existing permissions in the file.
pub fn install(project_root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let claude_dir = project_root.join(".claude");
    fs::create_dir_all(&claude_dir)?;

    let settings_path = claude_dir.join("settings.local.json");

    // Parse existing settings or start fresh.
    let mut settings: serde_json::Value = if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)?;
        serde_json::from_str(&content)?
    } else {
        serde_json::json!({})
    };

    // Ensure permissions.allow array exists.
    let allow = settings
        .as_object_mut()
        .ok_or("settings.local.json is not a JSON object")?
        .entry("permissions")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or("permissions is not a JSON object")?
        .entry("allow")
        .or_insert_with(|| serde_json::json!([]))
        .as_array_mut()
        .ok_or("permissions.allow is not an array")?;

    // Collect existing entries for dedup.
    let existing: std::collections::HashSet<String> = allow
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();

    let mut added = 0usize;
    for &perm in KM_PERMISSIONS {
        if !existing.contains(perm) {
            allow.push(serde_json::Value::String(perm.to_string()));
            added += 1;
        }
    }

    // Write back with pretty formatting.
    let output = serde_json::to_string_pretty(&settings)?;
    fs::write(&settings_path, format!("{output}\n"))?;

    if added == 0 {
        println!(
            "All km permissions already present in {}",
            settings_path.display()
        );
    } else {
        println!("Added {added} permission(s) to {}", settings_path.display());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn install_creates_settings_from_scratch() {
        let dir = tempfile::tempdir().unwrap();
        install(dir.path()).unwrap();

        let path = dir.path().join(".claude/settings.local.json");
        assert!(path.exists());

        let content: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let allow = content["permissions"]["allow"].as_array().unwrap();
        assert_eq!(allow.len(), KM_PERMISSIONS.len());
    }

    #[test]
    fn install_merges_with_existing() {
        let dir = tempfile::tempdir().unwrap();
        let claude_dir = dir.path().join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();
        fs::write(
            claude_dir.join("settings.local.json"),
            r#"{"permissions":{"allow":["Bash(wc:*)"]}}"#,
        )
        .unwrap();

        install(dir.path()).unwrap();

        let content: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(claude_dir.join("settings.local.json")).unwrap(),
        )
        .unwrap();
        let allow = content["permissions"]["allow"].as_array().unwrap();
        // Original + new permissions.
        assert_eq!(allow.len(), 1 + KM_PERMISSIONS.len());
        assert_eq!(allow[0].as_str().unwrap(), "Bash(wc:*)");
    }

    #[test]
    fn install_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        install(dir.path()).unwrap();
        install(dir.path()).unwrap();

        let content: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(dir.path().join(".claude/settings.local.json")).unwrap(),
        )
        .unwrap();
        let allow = content["permissions"]["allow"].as_array().unwrap();
        assert_eq!(allow.len(), KM_PERMISSIONS.len());
    }
}
