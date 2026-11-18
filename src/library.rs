//! The TSX library (`views/`: template apps, `lib/`, strategies) bundled into the crate, so an app
//! run without a source checkout can import it as `@affordance/...`. It is written once to a cache
//! directory named for its contents, so a different library never reuses another's files.
use include_dir::{Dir, include_dir};
use std::path::PathBuf;

static VIEWS: Dir = include_dir!("$CARGO_MANIFEST_DIR/views");
/// GPT-2's tokenizer: the default for bindings to chat servers, which tokenize the messages
/// themselves, so the runtime's own tokenizer only sizes the layout.
static DEFAULT_TOKENIZER: &[u8] = include_bytes!("../assets/gpt2-tokenizer.json");

/// The library for an app: the library it lives in (an ancestor directory with the library's
/// `index.json` and `lib/model.tsx`, as `views/` has), else the `views/` of the source checkout
/// it lives in, else the bundled one.
pub fn for_app(app: &std::path::Path) -> Result<PathBuf, String> {
    let app = std::path::absolute(app).map_err(|e| e.to_string())?;
    // A library is known by its own files, not by any `index.json` (a web project has one too).
    let library = |dir: &std::path::Path| dir.join("index.json").is_file() && dir.join("lib/model.tsx").is_file();
    for dir in app.ancestors().skip(1) {
        if library(dir) {
            return Ok(dir.to_path_buf());
        }
        if library(&dir.join("views")) {
            return Ok(dir.join("views"));
        }
    }
    bundled()
}

/// The bundled library's directory, written on first use under the user's cache directory
/// (`$XDG_CACHE_HOME/affordance` or `~/.cache/affordance`).
pub fn bundled() -> Result<PathBuf, String> {
    let mut files = vec![];
    collect(&VIEWS, &mut files);
    files.sort_by(|a, b| a.0.cmp(&b.0));
    unpack("library", &files)
}

/// The default tokenizer's file (GPT-2), written on first use beside the bundled library: for a
/// chat, Anthropic or Responses binding that names none.
pub fn default_tokenizer() -> Result<PathBuf, String> {
    Ok(unpack("tokenizer", &[("gpt2-tokenizer.json".into(), DEFAULT_TOKENIZER)])?.join("gpt2-tokenizer.json"))
}

/// `files` written once to `<cache>/affordance/<name>-<version>-<content hash>/`.
fn unpack(name: &str, files: &[(String, &'static [u8])]) -> Result<PathBuf, String> {
    use sha2::Digest;
    let mut hash = sha2::Sha256::new();
    for (path, bytes) in files {
        hash.update(path.as_bytes());
        hash.update([0]);
        hash.update(bytes);
        hash.update([0]);
    }
    let digest: String = hash.finalize().iter().take(8).map(|b| format!("{b:02x}")).collect();
    let cache = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .ok_or("no cache directory: set XDG_CACHE_HOME or HOME, or name a library in the deployment")?;
    let dir = cache
        .join("affordance")
        .join(format!("{name}-{}-{digest}", env!("CARGO_PKG_VERSION")));
    if dir.join(".complete").is_file() {
        return Ok(dir);
    }
    // Written beside its final name and renamed into place, so a reader never sees half of it.
    let tmp = dir.with_extension(format!("tmp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    for (path, bytes) in files {
        let file = tmp.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).map_err(|e| format!("{}: {e}", file.display()))?;
        std::fs::write(&file, bytes).map_err(|e| format!("{}: {e}", file.display()))?;
    }
    std::fs::write(tmp.join(".complete"), b"").map_err(|e| e.to_string())?;
    match std::fs::rename(&tmp, &dir) {
        Ok(()) => Ok(dir),
        // Another process wrote it first.
        Err(_) if dir.join(".complete").is_file() => {
            let _ = std::fs::remove_dir_all(&tmp);
            Ok(dir)
        }
        Err(e) => Err(format!("{}: {e}", dir.display())),
    }
}

fn collect(dir: &'static Dir<'static>, out: &mut Vec<(String, &'static [u8])>) {
    for file in dir.files() {
        out.push((file.path().to_string_lossy().into_owned(), file.contents()));
    }
    for sub in dir.dirs() {
        collect(sub, out);
    }
}

/// The module an app imports as `affordance:model`: `templates`, the template app for each of
/// the deployment's model templates, by name. A template app's name (`qwen3`, `messages`, ..:
/// `template_apps/index.json`) gives that app. A registered model's name (a key of the
/// library's `index.json`, written `org/name` or `org__name`) gives `ChatApp`
/// (`template_apps/chat.tsx`) with the model's format, as text (a hand-written format through
/// `textContent`), and what the index records about reading its replies. With no templates named, every template app.
pub fn model_module(library: &std::path::Path, templates: &[String]) -> Result<String, String> {
    let read = |name: &str| -> Result<serde_json::Value, String> {
        let path = library.join(name);
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
    };
    let apps = read("template_apps/index.json")?;
    // A view loaded without its deployment's templates (`View::load`, then a model chosen for
    // an agent or a render) gets every template app, as a library that imports them all would.
    let every: Vec<String> = apps.as_object().into_iter().flatten().map(|(k, _)| k.clone()).collect();
    let mut names: Vec<&String> = if templates.is_empty() {
        every.iter().collect()
    } else {
        templates.iter().collect()
    };
    names.sort();
    names.dedup();
    let at = |file: &str| serde_json::Value::String(library.join(file).to_string_lossy().into_owned()).to_string();
    let quoted = |name: &str| serde_json::Value::String(name.to_string()).to_string();
    let (mut source, mut entries) = (String::new(), vec![]);
    let mut registry: Option<(serde_json::Value, serde_json::Value)> = None;
    for (i, template) in names.iter().enumerate() {
        if let Some(app) = apps.get(template.as_str()) {
            let export = app["app"].as_str().unwrap_or_default();
            source += &format!(
                "import {{ {export} as T{i} }} from {};\n",
                at(&format!("template_apps/{template}.tsx"))
            );
            entries.push(format!("{}: T{i}", quoted(template)));
            continue;
        }
        if registry.is_none() {
            registry = Some((read("index.json")?, read("reply_protocols.json")?));
            source += &format!("import {{ ChatApp }} from {};\n", at("template_apps/chat.tsx"));
        }
        let (index, protocols) = registry.as_ref().unwrap();
        let key = template.replacen('/', "__", 1);
        let e = index.get(&key).filter(|e| e.is_object()).ok_or_else(|| {
            format!("unknown model template {template:?}: it is neither a template app nor a registered model (a key of the library's index.json)")
        })?;
        let (view, entry) = (
            e["view"].as_str().unwrap_or_default(),
            e["entry"].as_str().unwrap_or_default(),
        );
        let converted = e["converted"] == true;
        // What reading this model's replies needs: measured for the model itself (`agent`), over
        // what its view's row says (a hand-written view shared by several models).
        let mut row = protocols
            .get(format!("{view}::{entry}"))
            .filter(|r| r.is_object())
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));
        for (k, v) in e["agent"].as_object().into_iter().flatten() {
            row[k] = v.clone();
        }
        row.as_object_mut().unwrap().remove("reason");
        // Every registered model goes through the chat app's text path, where its pictures,
        // clips and videos become media elements sized by the model's rules and carrying their
        // files. A hand-written format is laid out as its text, as the publisher's template's
        // text is: the string the model's tokenizer reads whole.
        row["text"] = serde_json::json!(true);
        source += &if converted {
            format!("import F{i} from {};\n", at(view))
        } else {
            format!(
                "import {{ {entry} as H{i} }} from {};\nfunction F{i}(props) {{ return textContent(<>{{H{i}(props)}}</>); }}\n",
                at(view)
            )
        };
        source += &format!(
            "const R{i} = {row};\nfunction T{i}({{ children, system, keep_replies, ...options }}) {{\n  return ChatApp({{...R{i}, children, system, keep_replies, options, format: F{i}}});\n}}\n"
        );
        entries.push(format!("{}: T{i}", quoted(template)));
    }
    source += &format!("export const templates = {{{}}};\n", entries.join(", "));
    Ok(source)
}
