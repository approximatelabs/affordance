//! A controller of a run over its control socket (`affordance run --control PATH`), as a fleet
//! manager drives one: connect, send requests, read their answers.
use serde_json::{Value as J, json};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::{Duration, Instant};

pub struct Controller {
    stream: UnixStream,
    lines: BufReader<UnixStream>,
    next: u64,
}

impl Controller {
    /// Connect to the run's socket, waiting for the run to open it.
    pub fn connect(path: &Path) -> Controller {
        let until = Instant::now() + Duration::from_secs(60);
        loop {
            match UnixStream::connect(path) {
                Ok(stream) => {
                    let lines = BufReader::new(stream.try_clone().unwrap());
                    return Controller { stream, lines, next: 1 };
                }
                Err(e) if Instant::now() > until => panic!("{}: {e}", path.display()),
                Err(_) => std::thread::sleep(Duration::from_millis(20)),
            }
        }
    }
    /// Send `command` and return its answer (`{type: "result", value}` or `{type: "error", message}`).
    pub fn call(&mut self, mut command: J) -> J {
        let id = self.next;
        self.next += 1;
        command["id"] = json!(id);
        writeln!(self.stream, "{command}").unwrap();
        loop {
            let line = self
                .line()
                .unwrap_or_else(|| panic!("the run closed the socket before answering {command}"));
            if line["id"] == json!(id) {
                return line;
            }
        }
    }
    /// The next line the run sends (an answer, a status, `exit`), or `None` once it has gone.
    pub fn line(&mut self) -> Option<J> {
        let mut line = String::new();
        match self.lines.read_line(&mut line) {
            Ok(0) | Err(_) => None,
            Ok(_) => Some(serde_json::from_str(&line).unwrap()),
        }
    }
    /// Close the run, giving running calls `timeout_ms` to finish.
    pub fn close(&mut self, timeout_ms: u64) {
        let answer = self.call(json!({"type": "close", "timeout_ms": timeout_ms}));
        assert_eq!(answer["type"], "result", "{answer}");
    }
}
