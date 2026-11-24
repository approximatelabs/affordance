//! A deployment, as `affordance run` reads it: the base file (`affordance.yaml`, found from the
//! current directory upward, or the path given), then `affordance.local.yaml` beside it, then each
//! `-f` file, then each `--set`, then `--dir` and `--app`. Maps merge key by key; lists and scalars
//! replace; `null` removes a key. In files, `${VAR}` and `${VAR:-default}` are filled from the
//! environment, and every path is relative to the file that names it; paths given on the command
//! line are relative to the current directory. See docs/RUNTIME.md.
use serde_json::{Map, Value as J};
use std::path::{Path, PathBuf};

pub const FILE: &str = "affordance.yaml";
pub const LOCAL: &str = "affordance.local.yaml";
/// The journal directory when the deployment names none, beside the base file.
pub const STATE: &str = ".affordance";

/// Command-line layers over the files.
#[derive(Default)]
pub struct Overrides {
    pub files: Vec<PathBuf>,
    pub sets: Vec<String>,
    pub dir: Option<PathBuf>,
    pub app: Option<PathBuf>,
}

pub struct Deployment {
    pub config: J,
    /// The files read, in order.
    pub files: Vec<PathBuf>,
}

/// The base file: PATH (a file, or a directory holding `affordance.yaml`), else `affordance.yaml`
/// in the current directory or the nearest parent that has one.
pub fn find(path: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(p) = path {
        let file = if p.is_dir() { p.join(FILE) } else { p.to_path_buf() };
        if !file.is_file() {
            return Err(stray(p).unwrap_or_else(|| format!("{}: no such file", file.display())));
        }
        return Ok(file);
    }
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    for dir in cwd.ancestors() {
        if dir.join(FILE).is_file() {
            return Ok(dir.join(FILE));
        }
        if let Some(e) = stray(dir) {
            return Err(e);
        }
    }
    Err(format!(
        "no {FILE} in {} or its parents (give a path, or run `affordance init`)",
        cwd.display()
    ))
}

/// The error for a deployment under another name the search would otherwise skip.
fn stray(dir: &Path) -> Option<String> {
    let yml = if dir.is_dir() {
        dir.join("affordance.yml")
    } else {
        return None;
    };
    yml.is_file().then(|| format!("{}: rename it to {FILE}", yml.display()))
}

pub fn load(base: &Path, overrides: &Overrides) -> Result<Deployment, String> {
    let mut files = vec![base.to_path_buf()];
    let local = base.parent().unwrap_or(Path::new(".")).join(LOCAL);
    if local.is_file() && local != base {
        files.push(local);
    }
    files.extend(overrides.files.iter().cloned());
    let mut config = J::Object(Map::new());
    for file in &files {
        merge(&mut config, read(file)?);
    }
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    for set in &overrides.sets {
        let mut layer = assignment(set)?;
        resolve(&mut layer, &cwd);
        merge(&mut config, layer);
    }
    let object = config.as_object_mut().ok_or("a deployment must be a map")?;
    if let Some(dir) = &overrides.dir {
        object.insert("dir".into(), J::String(path_string(&cwd.join(dir))));
    }
    if let Some(app) = &overrides.app {
        // The deployment's app: every instance that names none of its own runs it.
        let app = J::String(path_string(&cwd.join(app)));
        object
            .entry("app")
            .or_insert_with(|| J::Object(Map::new()))
            .as_object_mut()
            .ok_or("app must be a map")?
            .insert("view".into(), app);
    }
    if !object.contains_key("dir") {
        let home = base.parent().unwrap_or(Path::new("."));
        object.insert("dir".into(), J::String(path_string(&absolute(home).join(STATE))));
    }
    // Unknown fields are refused here, for `config` and for every command that runs it.
    crate::format::validate(&config)?;
    Ok(Deployment { config, files })
}

/// One file: parsed (YAML; JSON is YAML too), its `${VAR}`s filled, its paths made absolute.
fn read(file: &Path) -> Result<J, String> {
    let text = std::fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))?;
    let options = serde_saphyr::options! { strict_booleans: true };
    let mut value: J =
        serde_saphyr::from_str_with_options(&text, options).map_err(|e| format!("{}: {e}", file.display()))?;
    if value.is_null() {
        value = J::Object(Map::new());
    }
    if !value.is_object() {
        return Err(format!("{}: a deployment must be a map", file.display()));
    }
    interpolate(&mut value, "", file)?;
    resolve(&mut value, &absolute(file.parent().unwrap_or(Path::new("."))));
    Ok(value)
}

fn absolute(p: &Path) -> PathBuf {
    std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf())
}

fn path_string(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

/// Maps merge key by key; anything else replaces; `null` removes the key. A map of `[N]` keys
/// over a list (from `--set key[N]=...`) merges into those elements.
pub fn merge(into: &mut J, layer: J) {
    if let (J::Array(list), J::Object(b)) = (&mut *into, &layer)
        && !b.is_empty()
        && b.keys().all(|k| k.starts_with('[') && k.ends_with(']'))
    {
        for (k, v) in b.clone() {
            if let Ok(n) = k[1..k.len() - 1].parse::<usize>() {
                if n < list.len() {
                    merge(&mut list[n], v);
                } else if n == list.len() {
                    list.push(v);
                } else {
                    eprintln!(
                        "affordance: warning: [{n}] is past the end of a list of {}: that value is not applied",
                        list.len()
                    );
                }
            }
        }
        return;
    }
    match (into, layer) {
        (J::Object(a), J::Object(b)) => {
            for (k, v) in b {
                if v.is_null() {
                    a.remove(&k);
                } else if let Some(existing) = a.get_mut(&k) {
                    merge(existing, v);
                } else {
                    a.insert(k, v);
                }
            }
        }
        (slot, layer) => *slot = layer,
    }
}

/// `KEY=VALUE` as a layer: KEY a dotted path (`[N]` indexes a list), VALUE parsed as YAML.
fn assignment(set: &str) -> Result<J, String> {
    let (key, raw) = set
        .split_once('=')
        .ok_or_else(|| format!("--set {set}: expected KEY=VALUE"))?;
    let options = serde_saphyr::options! { strict_booleans: true };
    let value: J = if raw.trim().is_empty() {
        J::String(String::new())
    } else {
        serde_saphyr::from_str_with_options(raw, options).map_err(|e| format!("--set {set}: {e}"))?
    };
    let mut layer = value;
    for part in key.split('.').rev() {
        if part.is_empty() {
            return Err(format!("--set {set}: empty key in {key:?}"));
        }
        let (name, index) = match part.split_once('[') {
            Some((name, rest)) => {
                let n: usize = rest
                    .strip_suffix(']')
                    .and_then(|n| n.parse().ok())
                    .ok_or_else(|| format!("--set {set}: bad index in {part:?}"))?;
                (name, Some(n))
            }
            None => (part, None),
        };
        if let Some(n) = index {
            // A list element: the other elements are kept by merging into the existing list.
            layer = J::Object(Map::from_iter([(format!("[{n}]"), layer)]));
        }
        layer = J::Object(Map::from_iter([(name.to_string(), layer)]));
    }
    Ok(layer)
}

/// `${VAR}`, `${VAR:-default}` and `$$` in every string.
fn interpolate(value: &mut J, at: &str, file: &Path) -> Result<(), String> {
    match value {
        J::String(s) if s.contains('$') => *s = fill(s).map_err(|e| format!("{} at {at}: {e}", file.display()))?,
        J::Array(a) => {
            for (i, v) in a.iter_mut().enumerate() {
                interpolate(v, &format!("{at}[{i}]"), file)?;
            }
        }
        J::Object(o) => {
            for (k, v) in o.iter_mut() {
                let key = if at.is_empty() { k.clone() } else { format!("{at}.{k}") };
                interpolate(v, &key, file)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn fill(s: &str) -> Result<String, String> {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find('$') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        if let Some(r) = after.strip_prefix('$') {
            out.push('$');
            rest = r;
        } else if let Some(r) = after.strip_prefix('{') {
            let end = r.find('}').ok_or("unclosed ${")?;
            let (name, default) = match r[..end].split_once(":-") {
                Some((n, d)) => (n, Some(d)),
                None => (&r[..end], None),
            };
            match (std::env::var(name), default) {
                (Ok(v), _) => out.push_str(&v),
                (Err(_), Some(d)) => out.push_str(d),
                (Err(_), None) => return Err(format!("${{{name}}} is not set (use ${{{name}:-default}})")),
            }
            rest = &r[end + 1..];
        } else {
            out.push('$');
            rest = after;
        }
    }
    out.push_str(rest);
    Ok(out)
}

/// Make the deployment's local paths absolute against `base`: the app and its views, tokenizers,
/// key files, the library, the journal directory, and the host paths of local machines.
fn resolve(config: &mut J, base: &Path) {
    let at = |v: &mut J| {
        // `~/x` is the user's home, as a shell reads it; left as written it would name a
        // directory called `~` under the current one.
        if let J::String(s) = v
            && let Some(rest) = s.strip_prefix("~/")
            && let Some(home) = std::env::var_os("HOME")
        {
            *s = path_string(&Path::new(&home).join(rest));
            return;
        }
        if let J::String(s) = v
            && !s.is_empty()
            && Path::new(s.as_str()).is_relative()
            && !s.starts_with('~')
        {
            *s = path_string(&base.join(s.as_str()));
        }
    };
    use crate::format;
    let Some(o) = config.as_object_mut() else { return };
    // Which fields hold a local path is the format table's to say (`format.rs`).
    format::paths(o, format::TOP).into_iter().for_each(&at);
    let named = |object: Option<&mut J>, names: &[&str]| {
        for (key, value) in object.and_then(J::as_object_mut).into_iter().flatten() {
            if names.contains(&key.as_str()) {
                at(value);
            }
        }
    };
    let instance = |a: &mut J| {
        if let Some(a) = a.as_object_mut() {
            format::paths(a, format::INSTANCE).into_iter().for_each(&at);
        }
    };
    named(o.get_mut("app"), format::APP_PATHS);
    named(o.get_mut("model"), format::MODEL_PATHS);
    for m in o
        .get_mut("models")
        .and_then(J::as_object_mut)
        .into_iter()
        .flatten()
        .map(|(_, m)| m)
    {
        named(Some(m), format::MODEL_PATHS);
    }
    if let Some(J::Array(list)) = o.get_mut("instances") {
        list.iter_mut().for_each(instance);
    }
    if let Some(machines) = o.get_mut("machines").and_then(J::as_object_mut) {
        for m in machines.values_mut() {
            let Some(m) = m.as_object_mut() else { continue };
            let kind = m.get("kind").and_then(J::as_str).unwrap_or("").to_string();
            if let Some(fields) = format::machine(&kind) {
                format::paths(m, fields).into_iter().for_each(&at);
            }
            if kind == "command"
                && let Some(first) = m.get_mut("argv").and_then(J::as_array_mut).and_then(|a| a.first_mut())
                && first.as_str().is_some_and(|s| s.contains('/'))
            {
                at(first);
            }
        }
    }
}

/// The deployment with the values of its `secrets` replaced, for printing.
pub fn redacted(config: &J) -> J {
    let mut out = config.clone();
    let secrets: Vec<String> = config["secrets"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|s| s.as_str().map(str::to_owned))
        .collect();
    fn hide(v: &mut J, secrets: &[String]) {
        if let Some(env) = v.get_mut("env").and_then(J::as_object_mut) {
            for s in secrets {
                if let Some(x) = env.get_mut(s) {
                    *x = J::String("<redacted>".into());
                }
            }
        }
    }
    hide(&mut out, &secrets);
    if let Some(J::Array(list)) = out.get_mut("instances") {
        list.iter_mut().for_each(|a| hide(a, &secrets));
    }
    // The same value written anywhere else (a prop, a header, another variable) is hidden too:
    // a secret is its value, wherever the file puts it.
    let mut values = vec![];
    let mut collect = |v: &J| {
        for s in &secrets {
            if let Some(value) = v["env"][s].as_str().filter(|value| !value.is_empty()) {
                values.push(value.to_string());
            }
        }
    };
    collect(config);
    if let J::Array(list) = &config["instances"] {
        list.iter().for_each(&mut collect);
    }
    fn scrub(v: &mut J, values: &[String]) {
        match v {
            J::String(s) => {
                if values.iter().any(|value| s == value) {
                    *s = "<redacted>".into();
                } else {
                    // Inside a longer string (`Bearer <key>`), for values long enough not to be
                    // an ordinary word.
                    for value in values.iter().filter(|value| value.len() >= 8) {
                        if s.contains(value.as_str()) {
                            *s = s.replace(value.as_str(), "<redacted>");
                        }
                    }
                }
            }
            J::Array(list) => list.iter_mut().for_each(|x| scrub(x, values)),
            J::Object(map) => map.values_mut().for_each(|x| scrub(x, values)),
            _ => {}
        }
    }
    scrub(&mut out, &values);
    out
}
