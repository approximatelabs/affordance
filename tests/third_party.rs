//! Text copied from a model's published chat template lives in `views/third_party/`, one folder
//! per upstream with its notice, and template apps import it. A long prose literal in a template
//! app outside that folder fails here: move it there (error messages excepted).
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn tsx_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            tsx_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "tsx") {
            out.push(path);
        }
    }
}

/// Double-quoted string literals longer than 100 characters with more than 12 words: prose, not
/// format. (Template literals are not checked: a pattern cannot pair their backticks.)
fn prose_literals(source: &str) -> Vec<(usize, String)> {
    let literal = regex::Regex::new(r#""(?:[^"\\\n]|\\.)*""#).unwrap();
    let word = regex::Regex::new(r"[A-Za-z]{3,}|[\p{Han}]").unwrap();
    let lines: Vec<&str> = source.lines().collect();
    let mut found = vec![];
    for m in literal.find_iter(source) {
        let line = source[..m.start()].matches('\n').count();
        let text = lines[line].trim_start();
        if text.starts_with("//") || lines[line].contains("Error(") || lines[line].contains("throw ") {
            continue;
        }
        if m.as_str().len() > 100 && word.find_iter(m.as_str()).count() > 12 {
            found.push((line + 1, m.as_str().chars().take(60).collect()));
        }
    }
    found
}

#[test]
fn template_prose_lives_in_third_party() {
    let views = root().join("views");
    let mut files = vec![];
    for family in std::fs::read_dir(&views).unwrap() {
        let dir = family.unwrap().path();
        let name = dir.file_name().unwrap().to_string_lossy().into_owned();
        // Demos, libraries and agents are this project's own prose.
        if dir.is_dir()
            && !["third_party", "demo", "apps", "agents", "lib", "strategies", "decision"].contains(&name.as_str())
        {
            tsx_files(&dir, &mut files);
        }
    }
    let mut misplaced = vec![];
    for file in files {
        for (line, start) in prose_literals(&std::fs::read_to_string(&file).unwrap()) {
            misplaced.push(format!(
                "{}:{line}: {start}",
                file.strip_prefix(root()).unwrap().display()
            ));
        }
    }
    assert!(
        misplaced.is_empty(),
        "move upstream prose to views/third_party/:\n{}",
        misplaced.join("\n")
    );
}

#[test]
fn every_third_party_folder_has_a_notice() {
    for entry in std::fs::read_dir(root().join("views/third_party")).unwrap() {
        let dir = entry.unwrap().path();
        if dir.is_dir() {
            assert!(dir.join("NOTICE.md").is_file(), "{} has no NOTICE.md", dir.display());
        }
    }
}
