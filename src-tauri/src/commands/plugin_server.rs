//! Local loopback HTTP server that serves plugin assets.
//!
//! WebView2 refuses to embed `file://` frames inside a page whose origin
//! is `http://localhost:1420` (dev) or `http://tauri.localhost` (release),
//! and Tauri's asset/custom protocols are known to break for iframe
//! sub-resources on Windows. Hosting the plugin directory over
//! `http://127.0.0.1:<port>/<token>` is the reliable fix; the random
//! token keeps the surface private to this process.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::io::Cursor;
use std::path::{Component, Path, PathBuf};

use tiny_http::{Header, Method, Request, Response, Server};

use crate::services::plugins::SharedPluginRegistry;

/// Bridge client installed in every served HTML page, giving the plugin
/// iframe a `window.mytools` surface whose capability calls are
/// forwarded to the host over `postMessage`. Also hooks iframe
/// `error` / `unhandledrejection` and forwards a `$mytools` envelope so
/// the host can surface plugin page errors.
///
/// Single source of truth: `resources/bridge-client.js`. The typed
/// constant in `src/composables/pluginBridge.ts` imports the same file
/// with Vite's `?raw`, so the two runtimes cannot drift.
const BRIDGE_CLIENT: &str = include_str!("../../resources/bridge-client.js");

pub struct PluginServer {
    base: Option<String>,
}

impl PluginServer {
    pub fn start(registry: SharedPluginRegistry) -> Self {
        let server = match Server::http("127.0.0.1:0") {
            Ok(server) => server,
            Err(e) => {
                tracing::warn!("plugin server bind failed, falling back to file://: {e}");
                return Self { base: None };
            }
        };
        let port = server.server_addr().to_ip().map(|a| a.port()).unwrap_or(0);
        let token = random_token();
        let base = format!("http://127.0.0.1:{port}/{token}");
        let thread_token = token;
        let registry_handle = registry;
        let shared = PluginServerShared {
            registry: registry_handle,
        };
        std::thread::spawn(move || {
            for request in server.incoming_requests() {
                handle(request, &shared, &thread_token);
            }
        });
        tracing::info!("plugin server listening on http://127.0.0.1:{port}");
        Self { base: Some(base) }
    }

    pub fn base(&self) -> Option<&str> {
        self.base.as_deref()
    }
}

struct PluginServerShared {
    registry: SharedPluginRegistry,
}

/// Build the URL a plugin file is served at, relative to `root`.
pub fn serve_url(base: &str, root: &Path, dir: &str, file: &str) -> String {
    let dir_path = Path::new(dir);
    let rel_dir = dir_path
        .strip_prefix(root)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| PathBuf::from(dir_path.file_name().unwrap_or_default()));
    let rel = rel_dir.join(file).to_string_lossy().replace('\\', "/");
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        percent_encode_path(&rel)
    )
}

fn handle(request: Request, shared: &PluginServerShared, token: &str) {
    if request.method() != &Method::Get {
        let _ = request.respond(plain(405, "method not allowed"));
        return;
    }
    let Some(rel_encoded) = strip_token(request.url(), token) else {
        let _ = request.respond(plain(404, "not found"));
        return;
    };
    let rel = percent_decode(rel_encoded);

    if let Some(root) = shared.registry.root() {
        if let Some(target) = resolve_under(&root, &rel) {
            return respond_with_file(request, target);
        }
    }

    let _ = request.respond(plain(404, "not found"));
}

fn respond_with_file(request: Request, target: PathBuf) {
    let Ok(bytes) = std::fs::read(&target) else {
        let _ = request.respond(plain(404, "not found"));
        return;
    };
    let content_type = content_type(&target);
    let body = if content_type.starts_with("text/html") {
        inject_bridge(&String::from_utf8_lossy(&bytes)).into_bytes()
    } else {
        bytes
    };
    let response = Response::from_data(body)
        .with_status_code(200)
        .with_header(header("Content-Type", content_type))
        .with_header(header("Cache-Control", "no-store"))
        .with_header(header("X-Content-Type-Options", "nosniff"));
    let _ = request.respond(response);
}

/// Split `/<token>/<path>` and return the still-percent-encoded path.
fn strip_token<'a>(url: &'a str, token: &str) -> Option<&'a str> {
    let path = url.split('?').next().unwrap_or("");
    let rest = path.trim_start_matches('/');
    let (first, tail) = match rest.split_once('/') {
        Some((first, tail)) => (first, tail),
        None => (rest, ""),
    };
    (first == token).then_some(tail)
}

fn resolve_under(root: &Path, rel: &str) -> Option<PathBuf> {
    if rel.is_empty() {
        return None;
    }
    let candidate = root.join(rel);
    if candidate
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        return None;
    }
    let canon_root = root.canonicalize().ok()?;
    let canon = candidate.canonicalize().ok()?;
    canon.starts_with(&canon_root).then_some(canon)
}

/// Canonicalize `rel` under `root`, rejecting traversal and missing files.
#[allow(dead_code)]
fn resolve_request(root: &Path, rel: &str) -> Option<PathBuf> {
    if rel.is_empty() {
        return None;
    }
    let candidate = root.join(rel);
    if candidate
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        return None;
    }
    let canon_root = root.canonicalize().ok()?;
    let canon = candidate.canonicalize().ok()?;
    canon.starts_with(&canon_root).then_some(canon)
}

fn inject_bridge(html: &str) -> String {
    let lower = html.to_ascii_lowercase();
    if let Some(idx) = lower.find("</head>") {
        let mut out = String::with_capacity(html.len() + BRIDGE_CLIENT.len() + 16);
        out.push_str(&html[..idx]);
        out.push_str("<script>");
        out.push_str(BRIDGE_CLIENT);
        out.push_str("</script>");
        out.push_str(&html[idx..]);
        out
    } else {
        format!("<script>{BRIDGE_CLIENT}</script>{html}")
    }
}

fn content_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("html") | Some("htm") => "text/html; charset=utf-8",
        Some("js") | Some("mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") | Some("map") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        Some("otf") => "font/otf",
        Some("wasm") => "application/wasm",
        Some("txt") | Some("md") => "text/plain; charset=utf-8",
        Some("xml") => "application/xml",
        _ => "application/octet-stream",
    }
}

fn percent_encode_path(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(high), Some(low)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push(high * 16 + low);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_bytes()).expect("valid static header")
}

fn plain(code: u16, message: &str) -> Response<Cursor<Vec<u8>>> {
    Response::from_string(message)
        .with_status_code(code)
        .with_header(header("Content-Type", "text/plain; charset=utf-8"))
}

fn random_token() -> String {
    let mut out = String::with_capacity(32);
    for _ in 0..2 {
        let mut hasher = RandomState::new().build_hasher();
        hasher.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        );
        out.push_str(&format!("{:016x}", hasher.finish()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_token_requires_exact_prefix() {
        let url = "/abc/index.html?x=1";
        assert_eq!(strip_token(url, "abc"), Some("index.html"));
        assert_eq!(strip_token(url, "nope"), None);
        assert_eq!(strip_token("/abc", "abc"), Some(""));
    }

    #[test]
    fn percent_roundtrip_unicode() {
        let encoded = percent_encode_path("中文 目录/a b.js");
        assert_eq!(encoded, "%E4%B8%AD%E6%96%87%20%E7%9B%AE%E5%BD%95/a%20b.js");
        assert_eq!(percent_decode(&encoded), "中文 目录/a b.js");
    }

    #[test]
    fn resolves_files_and_rejects_traversal() {
        let base = std::env::temp_dir().join(format!("mytools-server-{}", random_token()));
        let root = base.join("root");
        std::fs::create_dir_all(root.join("sub")).expect("create dirs");
        std::fs::write(root.join("sub/index.html"), "<head></head>").expect("write");
        std::fs::write(base.join("secret.txt"), "top secret").expect("write");

        assert!(resolve_request(&root, "sub/index.html").is_some());
        assert!(resolve_request(&root, "sub").is_some());
        assert!(resolve_request(&root, "../secret.txt").is_none());
        assert!(resolve_request(&root, "missing.html").is_none());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn injects_bridge_before_head_close() {
        let out = inject_bridge("<html><head><title>x</title></head><body></body></html>");
        assert!(out.contains("<script>"));
        assert!(out.contains("window.mytools"));
        assert!(out.find("</head>").unwrap() > out.find("window.mytools").unwrap());
    }

    #[test]
    fn injects_bridge_without_head() {
        let out = inject_bridge("<body>hi</body>");
        assert!(out.starts_with("<script>"));
        assert!(out.ends_with("<body>hi</body>"));
    }

    #[test]
    fn bridge_client_hooks_error_and_rejection() {
        assert!(BRIDGE_CLIENT.contains("addEventListener('error'"));
        assert!(BRIDGE_CLIENT.contains("addEventListener('unhandledrejection'"));
        assert!(BRIDGE_CLIENT.contains("$mytools"));
        assert!(BRIDGE_CLIENT.contains("kind: 'error'"));
        assert!(BRIDGE_CLIENT.contains("kind: 'unhandledrejection'"));
    }

    #[test]
    fn bridge_client_filters_known_noise() {
        assert!(BRIDGE_CLIENT.contains("__isNoise"));
        assert!(
            BRIDGE_CLIENT.contains("ResizeObserver loop completed with undelivered notifications")
        );
        assert!(BRIDGE_CLIENT.contains("ResizeObserver loop limit exceeded"));
    }

    #[test]
    fn maps_content_types() {
        assert_eq!(
            content_type(Path::new("a/b/index.HTML")),
            "text/html; charset=utf-8"
        );
        assert_eq!(
            content_type(Path::new("x.js")),
            "text/javascript; charset=utf-8"
        );
        assert_eq!(content_type(Path::new("logo.svg")), "image/svg+xml");
        assert_eq!(
            content_type(Path::new("data.bin")),
            "application/octet-stream"
        );
    }

    #[test]
    fn serve_url_encodes_and_strips_root() {
        let base = "http://127.0.0.1:5555/token";
        let url = serve_url(
            base,
            Path::new("C:/plugins"),
            "C:/plugins/演示",
            "assets/index.js",
        );
        assert_eq!(
            url,
            "http://127.0.0.1:5555/token/%E6%BC%94%E7%A4%BA/assets/index.js"
        );
    }

    #[test]
    fn serves_plugin_file_over_http() {
        use std::io::{Read, Write};
        use std::net::TcpStream;

        let base_dir = std::env::temp_dir().join(format!("mytools-http-{}", random_token()));
        let root = base_dir.join("plugins");
        std::fs::create_dir_all(root.join("demo")).expect("create dirs");
        std::fs::write(
            root.join("demo/index.html"),
            "<html><head></head><body>hi</body></html>",
        )
        .expect("write");
        let registry = std::sync::Arc::new(crate::services::plugins::PluginRegistry::new(
            base_dir.join("state.json"),
        ));
        registry.reload(&root).expect("reload");
        let server = PluginServer::start(std::sync::Arc::clone(&registry));
        let base = server.base().expect("server base");
        let host_port = base
            .strip_prefix("http://")
            .and_then(|r| r.split('/').next())
            .expect("host:port");
        let token = base.rsplit('/').next().expect("token");

        let get = |path: &str| {
            let mut stream = TcpStream::connect(host_port).expect("connect");
            let request =
                format!("GET {path} HTTP/1.1\r\nHost: {host_port}\r\nConnection: close\r\n\r\n");
            stream.write_all(request.as_bytes()).expect("write");
            let mut response = String::new();
            stream.read_to_string(&mut response).expect("read");
            response
        };

        let ok = get(&format!("/{token}/demo/index.html"));
        assert!(ok.starts_with("HTTP/1.1 200"), "{ok}");
        assert!(ok.contains("window.mytools"));

        let blocked = get(&format!("/{token}/../state.json"));
        assert!(blocked.contains("404"), "{blocked}");

        let wrong_token = get("/deadbeef/demo/index.html");
        assert!(wrong_token.contains("404"), "{wrong_token}");

        let _ = std::fs::remove_dir_all(&base_dir);
    }
}
