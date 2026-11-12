//! Test support: fixture loading and a scripted local HTTP server (no extra crates).
#![cfg(test)]
#![allow(dead_code)] // each test binary uses a different subset

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// `contracts/fixtures/` at the same relative place as in the kristoff repo.
pub fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../contracts/fixtures")
        .join(name)
}

pub fn fixture_bytes(name: &str) -> Vec<u8> {
    std::fs::read(fixture_path(name)).unwrap()
}

pub fn fixture_json(name: &str) -> serde_json::Value {
    serde_json::from_slice(&fixture_bytes(name)).unwrap()
}

/// One recorded request.
#[derive(Debug, Clone)]
pub struct Recorded {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Recorded {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// One scripted reply: status, headers, body chunks (each written then flushed), then an
/// optional stall before closing.
#[derive(Debug, Clone)]
pub struct Reply {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub chunks: Vec<Vec<u8>>,
    pub stall: Option<Duration>,
}

impl Reply {
    pub fn json(status: u16, body: &str) -> Self {
        Self {
            status,
            headers: vec![
                ("content-type".into(), "application/json".into()),
                ("content-length".into(), body.len().to_string()),
            ],
            chunks: vec![body.as_bytes().to_vec()],
            stall: None,
        }
    }

    pub fn sse(chunks: Vec<Vec<u8>>) -> Self {
        Self {
            status: 200,
            headers: vec![("content-type".into(), "text/event-stream".into())],
            chunks,
            stall: None,
        }
    }

    pub fn with_header(mut self, k: &str, v: &str) -> Self {
        self.headers.push((k.into(), v.into()));
        self
    }
}

pub struct MockServer {
    pub base_url: String,
    pub requests: Arc<Mutex<Vec<Recorded>>>,
}

/// Serves `replies` in order, one per connection.
pub async fn serve(replies: Vec<Reply>) -> MockServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let requests = Arc::new(Mutex::new(Vec::new()));
    let log = requests.clone();
    tokio::spawn(async move {
        for reply in replies {
            let Ok((mut sock, _)) = listener.accept().await else {
                return;
            };
            let recorded = read_request(&mut sock).await;
            log.lock().unwrap().push(recorded);
            write_reply(&mut sock, &reply).await;
        }
    });
    MockServer { base_url, requests }
}

async fn read_request(sock: &mut tokio::net::TcpStream) -> Recorded {
    let mut buf = Vec::new();
    let head_end = loop {
        let n = read_more(sock, &mut buf).await;
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i;
        }
        assert!(n > 0, "connection closed before headers");
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).into_owned();
    let mut lines = head.split("\r\n");
    let mut first = lines.next().unwrap().split(' ');
    let method = first.next().unwrap().to_owned();
    let path = first.next().unwrap().to_owned();
    let headers: Vec<(String, String)> = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_owned(), v.trim().to_owned()))
        .collect();
    let mut recorded = Recorded {
        method,
        path,
        headers,
        body: buf[head_end + 4..].to_vec(),
    };
    read_body(sock, &mut recorded).await;
    recorded
}

async fn read_more(sock: &mut tokio::net::TcpStream, buf: &mut Vec<u8>) -> usize {
    let mut tmp = [0u8; 8192];
    let n = sock.read(&mut tmp).await.unwrap();
    buf.extend_from_slice(&tmp[..n]);
    n
}

async fn read_body(sock: &mut tokio::net::TcpStream, r: &mut Recorded) {
    if let Some(len) = r
        .header("content-length")
        .and_then(|v| v.parse::<usize>().ok())
    {
        while r.body.len() < len {
            assert!(read_more(sock, &mut r.body).await > 0);
        }
    } else if r
        .header("transfer-encoding")
        .is_some_and(|v| v.contains("chunked"))
    {
        while !r.body.ends_with(b"0\r\n\r\n") {
            assert!(read_more(sock, &mut r.body).await > 0);
        }
        r.body = dechunk(&r.body);
    }
}

fn dechunk(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut rest = raw;
    loop {
        let i = rest.windows(2).position(|w| w == b"\r\n").unwrap();
        let size =
            usize::from_str_radix(std::str::from_utf8(&rest[..i]).unwrap().trim(), 16).unwrap();
        if size == 0 {
            return out;
        }
        out.extend_from_slice(&rest[i + 2..i + 2 + size]);
        rest = &rest[i + 2 + size + 2..];
    }
}

async fn write_reply(sock: &mut tokio::net::TcpStream, reply: &Reply) {
    let fields = reply.headers.iter().fold(String::new(), |mut acc, (k, v)| {
        use std::fmt::Write as _;
        let _ = write!(acc, "{k}: {v}\r\n");
        acc
    });
    let head = format!(
        "HTTP/1.1 {} X\r\nconnection: close\r\n{fields}\r\n",
        reply.status
    );
    if sock.write_all(head.as_bytes()).await.is_err() {
        return;
    }
    for chunk in &reply.chunks {
        if sock.write_all(chunk).await.is_err() {
            return;
        }
        let _ = sock.flush().await;
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    if let Some(stall) = reply.stall {
        tokio::time::sleep(stall).await;
    }
    let _ = sock.shutdown().await;
}

/// Splits bytes into chunks of `n` (the last may be shorter).
pub fn chunks_of(bytes: &[u8], n: usize) -> Vec<Vec<u8>> {
    bytes.chunks(n).map(<[u8]>::to_vec).collect()
}
