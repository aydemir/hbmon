//! stderr pattern matching for missing dependencies (RFC 7.4).
//! Returns (pattern_id, category) on first match.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use std::path::{Path, PathBuf};

struct Pat {
    id: &'static str,
    category: &'static str,
    re: Lazy<Regex>,
}

macro_rules! pat {
    ($id:expr, $cat:expr, $rx:expr) => {
        Pat {
            id: $id,
            category: $cat,
            re: Lazy::new(|| Regex::new($rx).unwrap()),
        }
    };
}

static PATTERNS: Lazy<Vec<Pat>> = Lazy::new(|| {
    vec![
        pat!("cmd_not_found", "dep_missing:command", r"(?i)command not found"),
        pat!("no_such_file", "dep_missing:path", r"No such file or directory"),
        pat!("npm_module", "dep_missing:npm", r#"Cannot find module ['"]?(\S+)['"]?"#),
        pat!("py_module", "dep_missing:pymod", r#"ModuleNotFoundError: No module named ['"]?(\S+)['"]?"#),
        pat!("cargo_manifest", "dep_missing:crates", r"error: failed to parse manifest"),
        pat!("cabal_pkg", "dep_missing:cabal", r"error: package ID specification .* lacked"),
        pat!("c_header", "dep_missing:header", r"fatal error: .*: No such file"),
        pat!("ld_lib", "dep_missing:lib", r"Could not find .* in"),
        pat!("pkg_config", "dep_missing:pkg", r"Package .* was not found"),
        pat!("link_fail", "dep_missing:link", r"error: linking with \S+ failed"),
    ]
});

/// User-supplied patterns: `$XDG_CONFIG_HOME/hbmon/patterns.json`
/// (fallback `~/.config/hbmon/patterns.json`), shape:
/// `[{"id":"go-mod","category":"dep_missing:go","regex":"..."}]`.
/// Checked BEFORE builtins; invalid regexes are skipped, never fatal.
#[derive(Deserialize)]
struct CustomDef {
    id: String,
    category: String,
    regex: String,
}

pub struct CustomPat {
    pub id: String,
    pub category: String,
    pub re: Regex,
}

pub fn custom_path() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return Some(PathBuf::from(xdg).join("hbmon/patterns.json"));
        }
    }
    std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".config/hbmon/patterns.json"))
}

pub fn load_from(path: &Path) -> Vec<CustomPat> {
    let data = match std::fs::read_to_string(path) {
        Ok(d) => d,
        Err(_) => return vec![],
    };
    let defs: Vec<CustomDef> = serde_json::from_str(&data).unwrap_or_default();
    defs
        .into_iter()
        .filter_map(|d| {
            Regex::new(&d.regex).ok().map(|re| CustomPat {
                id: d.id,
                category: d.category,
                re,
            })
        })
        .collect()
}

static CUSTOM: Lazy<Vec<CustomPat>> = Lazy::new(|| {
    custom_path().map(|p| load_from(&p)).unwrap_or_default()
});

pub struct DepMatch {
    pub pattern_id: String,
    pub category: String,
    pub match_text: String,
}

pub fn match_line(line: &str) -> Option<DepMatch> {
    match_line_with(line, &CUSTOM)
}

fn match_line_with(line: &str, customs: &[CustomPat]) -> Option<DepMatch> {
    // keep line short for event payloads
    let short: String = line.chars().take(300).collect();
    for c in customs {
        if c.re.is_match(&short) {
            return Some(DepMatch {
                pattern_id: c.id.clone(),
                category: c.category.clone(),
                match_text: short,
            });
        }
    }
    for p in PATTERNS.iter() {
        if p.re.is_match(&short) {
            return Some(DepMatch {
                pattern_id: p.id.to_string(),
                category: p.category.to_string(),
                match_text: short,
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn npm_message_matches() {
        let m = match_line("Error: Cannot find module 'express'").unwrap();
        assert_eq!(m.pattern_id, "npm_module");
        assert_eq!(m.category, "dep_missing:npm");
    }

    #[test]
    fn python_message_matches() {
        let m = match_line("ModuleNotFoundError: No module named 'requests'").unwrap();
        assert_eq!(m.pattern_id, "py_module");
    }

    #[test]
    fn shell_not_found_matches() {
        assert!(match_line("make: gcc: command not found").is_some());
    }

    #[test]
    fn missing_file_matches() {
        assert!(match_line("cc: foo.c: No such file or directory").is_some());
    }

    #[test]
    fn ordinary_output_no_match() {
        assert!(match_line("Compiling hbmon v0.1.0 (/root/hbmon)").is_none());
        assert!(match_line("    Finished dev profile in 20.71s").is_none());
        assert!(match_line("").is_none());
    }

    #[test]
    fn match_text_truncated_to_300() {
        let long = "x".repeat(500);
        let m = match_line(&format!("command not found {}", long)).unwrap();
        assert!(m.match_text.chars().count() <= 300);
    }

    #[test]
    fn custom_file_loads_valid_and_skips_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("patterns.json");
        std::fs::write(
            &p,
            r#"[{"id":"go-mod","category":"dep_missing:go","regex":"no required module provides"},{"id":"bad","category":"x","regex":"([}]"}]"#,
        )
        .unwrap();
        let v = load_from(&p);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].id, "go-mod");
    }

    #[test]
    fn missing_file_is_empty() {
        assert!(load_from(Path::new("/nonexistent-hbmon-xyz.json")).is_empty());
    }

    #[test]
    fn custom_pattern_wins_over_builtin() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("patterns.json");
        std::fs::write(
            &p,
            r#"[{"id":"my-cmd","category":"dep_missing:custom","regex":"command not found"}]"#,
        )
        .unwrap();
        let customs = load_from(&p);
        let m = match_line_with("sh: foo: command not found", &customs).unwrap();
        assert_eq!(m.pattern_id, "my-cmd");
        assert_eq!(m.category, "dep_missing:custom");
    }
}
