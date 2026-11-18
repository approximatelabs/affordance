//! Small offline HTTP/1.1 fixture shared by connector tests. No production protocol code.
#![allow(dead_code)] // Each integration-test binary uses a different subset of the fixture.
use serde_json::Value as J;
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread::JoinHandle,
    time::Duration,
};

#[derive(Clone, Debug)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub headers: BTreeMap<String, String>,
    pub body: String,
}

pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub chunked: bool,
    pub stall: bool,
}
impl Response {
    pub fn text(status: u16, text: &str) -> Self {
        Self {
            status,
            headers: vec![],
            body: text.as_bytes().to_vec(),
            chunked: false,
            stall: false,
        }
    }
    pub fn json(value: J) -> Self {
        let mut response = Self::text(200, &value.to_string());
        response
            .headers
            .push(("Content-Type".into(), "application/json".into()));
        response
    }
    pub fn sse(text: &str) -> Self {
        let mut response = Self::text(200, text);
        response
            .headers
            .push(("Content-Type".into(), "text/event-stream; charset=utf-8".into()));
        response.chunked = true;
        response
    }
}

pub struct Server {
    pub url: String,
    pub requests: Arc<Mutex<Vec<Request>>>,
    pub connections: Arc<AtomicUsize>,
    pub closed: Arc<AtomicUsize>,
    stopping: Arc<AtomicBool>,
    sockets: Arc<Mutex<Vec<TcpStream>>>,
    thread: Option<JoinHandle<()>>,
}

fn read(reader: &mut BufReader<TcpStream>) -> std::io::Result<Option<Request>> {
    let mut line = String::new();
    if reader.read_line(&mut line)? == 0 {
        return Ok(None);
    }
    let mut start = line.split_whitespace();
    let method = start.next().unwrap().to_string();
    let path = start.next().unwrap().to_string();
    let mut headers = BTreeMap::new();
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        if line == "\r\n" {
            break;
        }
        let (name, value) = line.split_once(':').unwrap();
        // A strict server refuses a request that repeats a field; so does this one.
        let repeated = headers.insert(name.to_ascii_lowercase(), value.trim().to_string());
        assert!(repeated.is_none(), "the request sends header {name} twice");
    }
    let size = headers.get("content-length").map_or(0, |v| v.parse::<usize>().unwrap());
    let mut body = vec![0; size];
    reader.read_exact(&mut body)?;
    Ok(Some(Request {
        method,
        path,
        headers,
        body: String::from_utf8(body).unwrap(),
    }))
}

impl Server {
    pub fn new(handle: impl Fn(&Request) -> Response + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(vec![]));
        let connections = Arc::new(AtomicUsize::new(0));
        let closed = Arc::new(AtomicUsize::new(0));
        let stopping = Arc::new(AtomicBool::new(false));
        let sockets = Arc::new(Mutex::new(vec![]));
        let (seen, count, ends, stop, live) = (
            requests.clone(),
            connections.clone(),
            closed.clone(),
            stopping.clone(),
            sockets.clone(),
        );
        let handle = Arc::new(handle);
        let thread = std::thread::spawn(move || {
            let mut workers = vec![];
            while !stop.load(Ordering::SeqCst) {
                let mut stream = match listener.accept() {
                    Ok((stream, _)) => stream,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(2));
                        continue;
                    }
                    Err(e) => panic!("fixture accept: {e}"),
                };
                count.fetch_add(1, Ordering::SeqCst);
                // On macOS an accepted socket inherits the listener's non-blocking mode.
                stream.set_nonblocking(false).unwrap();
                stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
                live.lock().unwrap().push(stream.try_clone().unwrap());
                let (seen, ends, stop, handle) = (seen.clone(), ends.clone(), stop.clone(), handle.clone());
                workers.push(std::thread::spawn(move || {
                    let mut reader = BufReader::new(stream.try_clone().unwrap());
                    while !stop.load(Ordering::SeqCst) {
                        let request = match read(&mut reader) {
                            Ok(Some(r)) => r,
                            _ => break,
                        };
                        seen.lock().unwrap().push(request.clone());
                        let response = handle(&request);
                        let head = (|| -> std::io::Result<()> {
                            write!(stream, "HTTP/1.1 {} Fixture\r\n", response.status)?;
                            for (key, value) in response.headers {
                                write!(stream, "{key}: {value}\r\n")?;
                            }
                            if response.chunked {
                                write!(stream, "Transfer-Encoding: chunked\r\n\r\n")?;
                            } else {
                                write!(
                                    stream,
                                    "Content-Length: {}\r\n\r\n",
                                    if response.stall { 1 } else { response.body.len() }
                                )?;
                            }
                            if response.stall {
                                return stream.flush();
                            }
                            if response.chunked {
                                // Split UTF-8 and SSE events across HTTP chunk boundaries.
                                for chunk in response.body.chunks(7) {
                                    write!(stream, "{:x}\r\n", chunk.len())?;
                                    stream.write_all(chunk)?;
                                    write!(stream, "\r\n")?;
                                }
                                write!(stream, "0\r\nX-Fixture-Trailer: ignored\r\n\r\n")?;
                            } else if request.method != "HEAD" {
                                stream.write_all(&response.body)?;
                            }
                            stream.flush()
                        })();
                        if head.is_err() {
                            break;
                        }
                        if response.stall {
                            let mut b = [0];
                            let _ = stream.read(&mut b);
                            break;
                        }
                    }
                    ends.fetch_add(1, Ordering::SeqCst);
                }));
            }
            for worker in workers {
                worker.join().unwrap();
            }
        });
        Self {
            url,
            requests,
            connections,
            closed,
            stopping,
            sockets,
            thread: Some(thread),
        }
    }

    pub fn wait_for(&self, count: usize) {
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        while self.requests.lock().unwrap().len() < count {
            assert!(
                std::time::Instant::now() < deadline,
                "fixture did not receive {count} requests"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::SeqCst);
        for socket in self.sockets.lock().unwrap().drain(..) {
            let _ = socket.shutdown(Shutdown::Both);
        }
        if let Some(thread) = self.thread.take() {
            let result = thread.join();
            if !std::thread::panicking() {
                result.unwrap();
            }
        }
    }
}
