//! `console.log` and friends: what an app says to whoever is watching it, as a page's console is
//! its DevTools view. Lines are collected where the app runs and taken by its host, which shows
//! them (an instance's trace, the command's stderr). They are never state and never journaled.
use std::cell::RefCell;

/// The most lines kept between two takes, and the most characters of one line.
const MAX_LINES: usize = 1000;
const MAX_CHARS: usize = 16 << 10;

thread_local! {
    static LINES: RefCell<Vec<(&'static str, String)>> = const { RefCell::new(Vec::new()) };
}

/// The console method `name` (`log`, `info`, `warn`, `error`, `debug`) as a level.
pub(crate) fn level(name: &str) -> Option<&'static str> {
    ["log", "info", "warn", "error", "debug"]
        .into_iter()
        .find(|l| *l == name)
}

pub(crate) fn push(level: &'static str, mut text: String) {
    if let Some((at, _)) = text.char_indices().nth(MAX_CHARS) {
        text.truncate(at);
        text.push_str(" ...");
    }
    LINES.with(|lines| {
        let mut lines = lines.borrow_mut();
        if lines.len() < MAX_LINES {
            lines.push((level, text));
        }
    });
}

/// The lines written on this thread since the last take: `(level, text)`.
pub fn take() -> Vec<(&'static str, String)> {
    LINES.with(|lines| std::mem::take(&mut *lines.borrow_mut()))
}
