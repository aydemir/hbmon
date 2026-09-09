//! stderr pattern matching for missing dependencies (RFC 7.4).
//! Returns (pattern_id, category) on first match.

use once_cell::sync::Lazy;
use regex::Regex;

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

pub struct DepMatch {
    pub pattern_id: String,
    pub category: String,
    pub match_text: String,
}

pub fn match_line(line: &str) -> Option<DepMatch> {
    // keep line short for event payloads
    let short: String = line.chars().take(300).collect();
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
}
