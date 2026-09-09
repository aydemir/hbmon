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
