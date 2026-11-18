//! Flags: this machine's settings for the `affordance` command itself, as a browser keeps its
//! `about:flags` choices in the user's profile and not in any page. They change what the command
//! reports, never what a run does, so they live outside the deployment and the journal.
//!
//! The file is `$XDG_CONFIG_HOME/affordance/flags` (else `~/.config/affordance/flags`): one
//! `name = value` per line, `#` starts a comment. `--flag name=value` overrides it for one run.
use std::collections::BTreeMap;
use std::io::IsTerminal;
use std::path::PathBuf;

/// Every flag, its default, and what it does (shown by `affordance --help`).
pub const KNOWN: &[(&str, &str, &str)] = &[
    (
        "cache-warn",
        "auto",
        "warn when a prompt throws away most of the server's prefix cache: on, off, or auto (on when stderr is a terminal)",
    ),
    (
        "cache-warn-below",
        "0.2",
        "warn when less than this share of the previous prompt starts the new one",
    ),
    (
        "cache-warn-min-tokens",
        "1024",
        "ignore prompts shorter than this many tokens",
    ),
];

/// The resolved flags.
#[derive(Debug, Clone)]
pub struct Flags(BTreeMap<String, String>);

impl Flags {
    /// The defaults, then the flags file, then each `--flag name=value` in order.
    pub fn load(overrides: &[String]) -> Result<Flags, String> {
        let mut flags: BTreeMap<String, String> =
            KNOWN.iter().map(|(k, v, _)| (k.to_string(), v.to_string())).collect();
        if let Some(path) = path()
            && let Ok(text) = std::fs::read_to_string(&path)
        {
            for (n, line) in text.lines().enumerate() {
                let line = line.split('#').next().unwrap_or("").trim();
                if line.is_empty() {
                    continue;
                }
                set(&mut flags, line).map_err(|e| format!("{}:{}: {e}", path.display(), n + 1))?;
            }
        }
        for o in overrides {
            set(&mut flags, o).map_err(|e| format!("--flag {o}: {e}"))?;
        }
        let f = Flags(flags);
        f.cache_warn()?;
        Ok(f)
    }

    fn get(&self, name: &str) -> &str {
        &self.0[name]
    }

    fn number(&self, name: &str) -> Result<f64, String> {
        self.get(name)
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite() && *v >= 0.0)
            .ok_or_else(|| format!("flag {name}: expected a nonnegative number, got {:?}", self.get(name)))
    }

    /// The cache warning's settings, or `None` when it is off.
    pub fn cache_warn(&self) -> Result<Option<CacheWarn>, String> {
        let on = match self.get("cache-warn") {
            "on" => true,
            "off" => false,
            "auto" => std::io::stderr().is_terminal(),
            v => return Err(format!("flag cache-warn: expected on, off or auto, got {v:?}")),
        };
        let below = self.number("cache-warn-below")?;
        if below > 1.0 {
            return Err(format!(
                "flag cache-warn-below: expected a share from 0 to 1, got {below}"
            ));
        }
        let min_tokens = self.number("cache-warn-min-tokens")? as usize;
        Ok(on.then_some(CacheWarn { below, min_tokens }))
    }
}

/// When to warn about a prompt that throws away most of the server's prefix cache.
#[derive(Debug, Clone, Copy)]
pub struct CacheWarn {
    /// Warn when the shared start is less than this share of the previous prompt.
    pub below: f64,
    /// Ignore previous prompts shorter than this.
    pub min_tokens: usize,
}

fn set(flags: &mut BTreeMap<String, String>, line: &str) -> Result<(), String> {
    let (name, value) = line.split_once('=').ok_or("expected name = value")?;
    let (name, value) = (name.trim(), value.trim());
    if !flags.contains_key(name) {
        let names: Vec<&str> = KNOWN.iter().map(|(k, _, _)| *k).collect();
        return Err(format!("unknown flag {name:?} (known: {})", names.join(", ")));
    }
    flags.insert(name.to_string(), value.to_string());
    Ok(())
}

/// The flags file's path.
pub fn path() -> Option<PathBuf> {
    match std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        Some(dir) => Some(PathBuf::from(dir).join("affordance/flags")),
        None => std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config/affordance/flags")),
    }
}
