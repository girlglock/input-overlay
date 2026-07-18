use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

#[derive(rust_embed::Embed)]
#[folder = "$CARGO_MANIFEST_DIR/web-bundle"]
struct WebAssets;

pub struct HttpServer {
    _stop_tx: oneshot::Sender<()>,
}

impl HttpServer {
    pub fn start(host: &str, port: u16) -> Option<Self> {
        ensure_user_mount_dir();
        let (stop_tx, stop_rx) = oneshot::channel();
        let bind_addr = format!("{host}:{port}");
        tauri::async_runtime::spawn(run(bind_addr, stop_rx));
        Some(HttpServer { _stop_tx: stop_tx })
    }
}

async fn run(bind_addr: String, mut stop_rx: oneshot::Receiver<()>) {
    let listener = match TcpListener::bind(&bind_addr).await {
        Ok(l) => {
            tracing::info!("http: listening on http://{bind_addr}");
            l
        }
        Err(e) => {
            tracing::error!("http: failed to bind {bind_addr}: {e}");
            return;
        }
    };
    loop {
        tokio::select! {
            _ = &mut stop_rx => break,
            res = listener.accept() => match res {
                Ok((stream, addr)) => { tokio::spawn(serve(stream, addr)); }
                Err(e) => tracing::warn!("http: accept error: {e}"),
            }
        }
    }
    tracing::info!("http: server stopped");
}

async fn serve(stream: tokio::net::TcpStream, _addr: SocketAddr) {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);

    let mut request_line = String::new();
    if reader.read_line(&mut request_line).await.is_err() {
        return;
    }

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line).await {
            Ok(0) | Err(_) => return,
            Ok(_) if line == "\r\n" || line == "\n" => break,
            _ => {}
        }
    }

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let raw_path = parts.next().unwrap_or("/");
    if method != "GET" {
        return;
    }

    let path = raw_path.split('?').next().unwrap_or("/");
    let rel = path.trim_start_matches('/');

    if rel == "user_mount" || rel.starts_with("user_mount/") {
        let sub = rel
            .strip_prefix("user_mount")
            .unwrap()
            .trim_start_matches('/');
        serve_user_mount(sub, &mut write_half).await;
        return;
    }

    let key = if rel.is_empty() {
        "index.html".into()
    } else {
        rel.to_string()
    };

    let asset = WebAssets::get(&key).or_else(|| WebAssets::get(&format!("{key}/index.html")));

    match asset {
        Some(file) => {
            let body = file.data.as_ref();
            let mime = mime_of(&key);
            tracing::debug!("http: 200 {path}");
            let header = format!(
                "HTTP/1.0 200 OK\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = write_half.write_all(header.as_bytes()).await;
            let _ = write_half.write_all(body).await;
        }
        None => {
            tracing::debug!("http: 404 {path}");
            respond_err(&mut write_half, 404).await;
        }
    }
}

fn local_mount_dir() -> Option<PathBuf> {
    let dir = std::env::current_exe().ok()?.parent()?.join("user_mount");
    dir.is_dir().then_some(dir)
}

fn ensure_user_mount_dir() {
    let Some(exe) = std::env::current_exe().ok() else {
        return;
    };
    let Some(dir) = exe.parent().map(|p| p.join("user_mount")) else {
        return;
    };
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }

    let readme = dir.join("place your custom files here.txt");
    if !readme.exists() {
        let _ = std::fs::write(
            &readme,
            "everything you put in here will be hosted with the http server. you can host your own html overlays from here... they will end up on the address eg http://localhost:4456/user_mount/youroverlay.html",
        );
    }
}

async fn serve_user_mount(rel: &str, write_half: &mut tokio::net::tcp::OwnedWriteHalf) {
    if rel.is_empty() {
        let local = match local_mount_dir() {
            Some(base) => tokio::task::spawn_blocking(move || list_dir_recursive(&base))
                .await
                .unwrap_or_default(),
            None => Vec::new(),
        };
        let mut bundle: Vec<String> = WebAssets::iter().map(|f| f.to_string()).collect();
        bundle.sort();
        let payload = serde_json::json!({ "local": local, "bundle": bundle });
        respond_json(write_half, &payload.to_string()).await;
        return;
    }

    let Some(base) = local_mount_dir() else {
        respond_err(write_half, 404).await;
        return;
    };

    let rel_path = Path::new(rel);
    let is_safe = rel_path
        .components()
        .all(|c| matches!(c, Component::Normal(_)));
    if !is_safe {
        respond_err(write_half, 403).await;
        return;
    }

    match tokio::fs::read(base.join(rel_path)).await {
        Ok(body) => {
            let mime = mime_of(rel);
            tracing::debug!("http: 200 /user_mount/{rel}");
            let header = format!(
                "HTTP/1.0 200 OK\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = write_half.write_all(header.as_bytes()).await;
            let _ = write_half.write_all(&body).await;
        }
        Err(_) => {
            tracing::debug!("http: 404 /user_mount/{rel}");
            respond_err(write_half, 404).await;
        }
    }
}

fn list_dir_recursive(base: &Path) -> Vec<String> {
    fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, base, out);
            } else if let Ok(rel) = path.strip_prefix(base) {
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut out = Vec::new();
    walk(base, base, &mut out);
    out.sort();
    out
}

async fn respond_json(w: &mut tokio::net::tcp::OwnedWriteHalf, body: &str) {
    let header = format!(
        "HTTP/1.0 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = w.write_all(header.as_bytes()).await;
    let _ = w.write_all(body.as_bytes()).await;
}

async fn respond_err(w: &mut tokio::net::tcp::OwnedWriteHalf, code: u16) {
    let (text, body): (&str, &[u8]) = match code {
        403 => ("Forbidden", b"403 Forbidden"),
        404 => ("Not Found", b"404 Not Found"),
        500 => ("Internal Server Error", b"500 Internal Server Error"),
        _ => ("Error", b"Error"),
    };
    let resp = format!(
        "HTTP/1.0 {code} {text}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = w.write_all(resp.as_bytes()).await;
    let _ = w.write_all(body).await;
}

fn mime_of(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}
