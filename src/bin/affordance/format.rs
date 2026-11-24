//! The deployment format's fields, in one table. Which fields exist at each level and which of
//! them hold a local path is stated here and nowhere else: loading a deployment (paths made
//! absolute against the file that names them), `affordance config` and `affordance run` (an
//! unknown field is an error) all read this table.
//!
//! A model binding's own fields are the runtime's (`affordance::runtime::model::from_config`
//! checks them when the binding is built); the two of them that are local paths are listed here.
use serde_json::Value as J;

/// A field: its name, and whether it holds a local path.
pub struct Field(pub &'static str, pub bool);

const fn f(name: &'static str) -> Field {
    Field(name, false)
}
const fn path(name: &'static str) -> Field {
    Field(name, true)
}

/// The top level of a deployment.
pub const TOP: &[Field] = &[
    f("app"),
    f("instances"),
    f("model"),
    f("models"),
    f("machines"),
    path("dir"),
    f("workers"),
    f("input_batch"),
    f("model_threads"),
    f("exec_threads"),
    f("model_inflight"),
    f("restore_concurrency"),
    f("spill_bytes"),
    f("replay"),
    f("call_timeout_s"),
    f("max_wall_s"),
    f("keep_running"),
    f("serve"),
    f("env"),
    f("secrets"),
    path("library"),
];

/// One instance (an entry of `instances`; `count` makes copies of it).
pub const INSTANCE: &[Field] = &[
    f("id"),
    f("props"),
    f("env"),
    f("max_new"),
    f("max_turns"),
    f("machines"),
    f("capabilities"),
    f("inspect"),
    f("inspectable"),
    f("max_prompt"),
    f("rank"),
    f("seed"),
    f("count"),
    path("app"),
    f("entry"),
    f("model"),
    f("dedicated_worker"),
];

/// The paths in `app` and in a model binding.
pub const APP_PATHS: &[&str] = &["view", "tokenizer"];
pub const MODEL_PATHS: &[&str] = &["tokenizer", "key_file"];

/// `serve`.
pub const SERVE: &[Field] = &[f("listen"), f("max_body_bytes"), f("timeout_s")];

const LOCAL: &[Field] = &[f("kind"), path("home"), f("per_instance"), f("timeout_s"), f("count")];
const SSH: &[Field] = &[
    f("kind"),
    f("target"),
    path("key"),
    path("control_dir"),
    path("known_hosts"),
    f("timeout_s"),
    f("count"),
];
const DOCKER: &[Field] = &[f("kind"), f("image"), f("name"), f("timeout_s"), f("count")];
const BWRAP: &[Field] = &[
    f("kind"),
    path("home"),
    f("per_instance"),
    f("network"),
    f("read"),
    f("timeout_s"),
    f("count"),
];
const COMMAND: &[Field] = &[f("kind"), f("argv"), f("max_reply_bytes")];

/// A machine of `kind`. An `http` machine's fields are its own type's (`HttpConfig`); an unknown
/// kind has none.
pub fn machine(kind: &str) -> Option<&'static [Field]> {
    Some(match kind {
        "local" => LOCAL,
        "ssh" => SSH,
        "docker" => DOCKER,
        "bwrap" => BWRAP,
        "command" => COMMAND,
        _ => return None,
    })
}

/// `value` is an object whose every key is one of `fields` (or of `also`).
pub fn check(value: &J, fields: &[Field], also: &[&str], what: &str) -> Result<(), String> {
    let object = value.as_object().ok_or_else(|| format!("{what} must be an object"))?;
    match object
        .keys()
        .find(|key| !fields.iter().any(|field| field.0 == key.as_str()) && !also.contains(&key.as_str()))
    {
        Some(key) => Err(format!("{what}: unknown field `{key}`")),
        None => Ok(()),
    }
}

/// Every level of a deployment names only fields that exist. Run by `config` and by `run`.
pub fn validate(config: &J) -> Result<(), String> {
    check(config, TOP, &[], "deployment")?;
    match &config["instances"] {
        J::Null => {}
        J::Array(list) => {
            for (n, instance) in list.iter().enumerate() {
                check(instance, INSTANCE, &[], &format!("instance {n}"))?;
            }
        }
        _ => return Err("instances must be a list of instances".into()),
    }
    for (role, spec) in config["machines"].as_object().into_iter().flatten() {
        let kind = spec["kind"].as_str().unwrap_or("");
        if let Some(fields) = machine(kind) {
            check(spec, fields, &[], &format!("machine {role}"))?;
        } else if kind != "http" {
            return Err(format!(
                "machine {role}: unknown kind {kind:?} (local, ssh, docker, bwrap, command, http)"
            ));
        }
    }
    if !config["serve"].is_null() {
        check(&config["serve"], SERVE, &[], "serve")?;
    }
    Ok(())
}

/// The values of `object`'s path fields, for making them absolute.
pub fn paths<'a>(object: &'a mut serde_json::Map<String, J>, fields: &[Field]) -> Vec<&'a mut J> {
    object
        .iter_mut()
        .filter(|(key, _)| fields.iter().any(|field| field.1 && field.0 == key.as_str()))
        .map(|(_, value)| value)
        .collect()
}
