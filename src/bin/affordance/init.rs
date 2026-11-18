//! `affordance init [DIR]`: a first agent to edit and run, with types for its editor.
use std::path::Path;

// The first agent of examples/first_agent, so the scaffold and the guide are the same files.
const APP: &str = include_str!("../../../examples/first_agent/app.tsx");
const DEPLOYMENT: &str = include_str!("../../../examples/first_agent/affordance.yaml");

const GITIGNORE: &str = ".affordance/\naffordance.local.yaml\n";
// For editors: what the app's globals and elements are. They describe the platform; nothing runs them.
const TYPES: &str = include_str!("../../../assets/affordance.d.ts");
const TSCONFIG: &str = include_str!("../../../assets/tsconfig.json");

pub fn init(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let files = [
        ("app.tsx", APP),
        ("affordance.yaml", DEPLOYMENT),
        (".gitignore", GITIGNORE),
        ("affordance.d.ts", TYPES),
        ("tsconfig.json", TSCONFIG),
    ];
    if let Some((name, _)) = files.iter().find(|(name, _)| dir.join(name).exists()) {
        return Err(format!(
            "{} already exists; init writes into a new or empty directory",
            dir.join(name).display()
        ));
    }
    for (name, text) in files {
        std::fs::write(dir.join(name), text).map_err(|e| format!("{}: {e}", dir.join(name).display()))?;
    }
    let cd = if dir == Path::new(".") {
        String::new()
    } else {
        format!("cd {} && ", dir.display())
    };
    println!(
        "Wrote app.tsx, affordance.yaml, .gitignore and editor types (affordance.d.ts, tsconfig.json) in {}.\n\nStart a model server on port 8080 (llama-server -m MODEL.gguf --port 8080 --jinja), then:\n\n  {cd}affordance run\n",
        dir.display()
    );
    Ok(())
}
