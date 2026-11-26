//! Web Analytics Dev by Don PELY — local analytics collector.
//!
//! A tiny dependency-light HTTP server (default 127.0.0.1:3415):
//!   GET  /t.js   serves our tracker script (embedded at compile time)
//!   POST /e      ingests a JSON analytics event
//!   GET  /health liveness probe
//!
//! Cookieless by design: only the JSON payload fields are stored.

use crate::store::{EventRow, Store};
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use tiny_http::{Header, Method, Response, Server};

const TRACKER_JS: &str = include_str!("../../tracker/t.js");

fn header(key: &str, value: &str) -> Header {
    Header::from_bytes(key.as_bytes(), value.as_bytes()).expect("valid header")
}

fn handle_request(store: &Store, mut req: tiny_http::Request) {
    let cors = [
        header("Access-Control-Allow-Origin", "*"),
        header("Access-Control-Allow-Methods", "GET, POST, OPTIONS"),
        header("Access-Control-Allow-Headers", "Content-Type"),
    ];
    let with_cors = |mut res: Response<std::io::Cursor<Vec<u8>>>| {
        for h in &cors {
            res.add_header(h.clone());
        }
        res
    };

    let response = match (req.method(), req.url()) {
        (Method::Options, _) => with_cors(Response::from_string("").with_status_code(204)),
        (Method::Get, "/t.js") => {
            let mut res = with_cors(Response::from_string(TRACKER_JS));
            res.add_header(header("Content-Type", "application/javascript; charset=utf-8"));
            res.add_header(header("Cache-Control", "max-age=3600"));
            res
        }
        (Method::Get, "/health") => {
            let mut res = with_cors(Response::from_string(r#"{"ok":true,"app":"web-analytics-dev"}"#));
            res.add_header(header("Content-Type", "application/json"));
            res
        }
        (Method::Post, "/e") => {
            let mut body = String::new();
            let read = req.as_reader().take(64 * 1024).read_to_string(&mut body);
            match read {
                Ok(_) => match serde_json::from_str::<EventRow>(&body) {
                    Ok(evt) => {
                        let known = store.list_sites().iter().any(|s| s.domain == evt.site.trim().to_lowercase());
                        match store.record_event(evt) {
                            Ok(()) => {
                                let mut res = with_cors(Response::from_string(format!(r#"{{"ok":true,"knownSite":{known}}}"#)));
                                res.add_header(header("Content-Type", "application/json"));
                                res.with_status_code(202)
                            }
                            Err(e) => {
                                let mut res = with_cors(Response::from_string(format!(r#"{{"ok":false,"error":{}}}"#, serde_json::to_string(&e).unwrap_or_default())));
                                res.add_header(header("Content-Type", "application/json"));
                                res.with_status_code(400)
                            }
                        }
                    }
                    Err(e) => {
                        let msg = serde_json::to_string(&e.to_string()).unwrap_or_default();
                        let mut res = with_cors(Response::from_string(format!(r#"{{"ok":false,"error":{msg}}}"#)));
                        res.add_header(header("Content-Type", "application/json"));
                        res.with_status_code(400)
                    }
                },
                Err(_) => with_cors(Response::from_string(r#"{"ok":false,"error":"read failed"}"#).with_status_code(400)),
            }
        }
        _ => with_cors(Response::from_string(r#"{"ok":false,"error":"not found"}"#).with_status_code(404)),
    };

    let _ = req.respond(response);
}

/// A running collector server; dropping the struct does not stop it — call [`Collector::stop`].
pub struct Collector {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    port: Option<u16>,
}

impl Collector {
    pub fn stopped() -> Self {
        Self { stop: Arc::new(AtomicBool::new(true)), thread: None, port: None }
    }

    pub fn start(store: Store, port: u16) -> Self {
        let server = match Server::http(format!("127.0.0.1:{port}")) {
            Ok(s) => s,
            Err(_) => return Self::stopped(), // port busy — UI reports it as stopped
        };
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let thread = std::thread::Builder::new()
            .name("wadev-collector".into())
            .spawn(move || {
                while !flag.load(Ordering::Relaxed) {
                    match server.recv_timeout(Duration::from_millis(500)) {
                        Ok(Some(req)) => {
                            handle_request(&store, req);
                        }
                        Ok(None) => {}
                        Err(_) => break,
                    }
                }
            })
            .ok();
        let running = thread.is_some();
        Self { stop, thread, port: running.then_some(port) }
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        self.port = None;
    }

    pub fn port(&self) -> Option<u16> {
        self.port
    }
}
