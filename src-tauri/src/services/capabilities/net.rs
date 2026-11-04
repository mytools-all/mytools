use mytools_core::capability::{
    Capability, CapabilityCall, CapabilityError, CapabilityRegistry, Permission,
};
use serde_json::Value;
use std::io::Read;
use tauri::{AppHandle, Runtime};

pub(super) fn register<R: Runtime>(
    _app: &AppHandle<R>,
    registry: &mut CapabilityRegistry,
) -> Result<(), CapabilityError> {
    registry.register(Capability::new(
        "net.fetch",
        Permission::Exec,
        "Perform an HTTP(S) request",
        Box::new(net_fetch),
    ))?;
    Ok(())
}

fn net_fetch(call: &CapabilityCall) -> Result<Value, CapabilityError> {
    const MAX_BODY_BYTES: u64 = 4 * 1024 * 1024;
    let url = call
        .arg_str("url")
        .ok_or_else(|| CapabilityError::invalid_args("net.fetch expects { url }"))?;
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(CapabilityError::invalid_args(
            "net.fetch only supports http:// and https:// URLs",
        ));
    }
    let method = call.arg_str("method").unwrap_or("GET").to_ascii_uppercase();
    if !matches!(
        method.as_str(),
        "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD"
    ) {
        return Err(CapabilityError::invalid_args(format!(
            "unsupported HTTP method: {method}"
        )));
    }

    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(15))
        .build();
    let mut request = agent.request(&method, url);
    let mut has_content_type = false;
    if let Some(headers) = call.args.get("headers").and_then(Value::as_object) {
        for (name, value) in headers {
            if let Some(value) = value.as_str() {
                if name.eq_ignore_ascii_case("content-type") {
                    has_content_type = true;
                }
                request = request.set(name, value);
            }
        }
    }

    let response = match call.args.get("body") {
        None | Some(Value::Null) => request.call(),
        Some(Value::String(body)) => request.send_string(body),
        Some(body) => {
            if !has_content_type {
                request = request.set("Content-Type", "application/json");
            }
            request.send_string(&body.to_string())
        }
    };

    let response = match response {
        Ok(response) => response,
        Err(ureq::Error::Status(_, response)) => response,
        Err(err) => {
            return Err(CapabilityError::handler_failed(
                "net.fetch",
                err.to_string(),
            ))
        }
    };
    let status = response.status();
    let headers: serde_json::Map<String, Value> = response
        .headers_names()
        .into_iter()
        .filter_map(|name| {
            response
                .header(&name)
                .map(|value| (name, Value::String(value.to_string())))
        })
        .collect();
    let mut body = String::new();
    if method != "HEAD" {
        response
            .into_reader()
            .take(MAX_BODY_BYTES)
            .read_to_string(&mut body)
            .map_err(|e| CapabilityError::handler_failed("net.fetch", e.to_string()))?;
    }
    Ok(serde_json::json!({
        "status": status,
        "body": body,
        "headers": headers,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mytools_core::capability::{CallContext, Origin};
    use serde_json::json;
    #[test]
    fn net_fetch_reads_a_local_http_response() {
        use std::io::Write;
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\n\r\nok",
                )
                .unwrap();
        });

        let call = CapabilityCall {
            id: "net.fetch".to_string(),
            args: json!({ "url": format!("http://{addr}/") }),
            ctx: CallContext::new(Origin::Ecosystem),
        };
        let value = net_fetch(&call).unwrap();
        assert_eq!(value["status"], 200);
        assert_eq!(value["body"], "ok");
        assert!(value["headers"].to_string().contains("text/plain"));
        server.join().unwrap();
    }

    #[test]
    fn head_does_not_read_the_advertised_response_body() {
        use std::io::Write;
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Encoding: gzip\r\nContent-Length: 100\r\nConnection: close\r\n\r\n").unwrap();
        });
        let call = CapabilityCall {
            id: "net.fetch".into(),
            args: json!({"url":format!("http://{addr}/"), "method":"HEAD"}),
            ctx: CallContext::new(Origin::Official),
        };
        let value = net_fetch(&call).unwrap();
        assert_eq!(value["status"], 200);
        assert_eq!(value["body"], "");
        server.join().unwrap();
    }

    #[test]
    fn net_fetch_rejects_non_http_schemes() {
        let call = CapabilityCall {
            id: "net.fetch".to_string(),
            args: json!({ "url": "file:///C:/secret" }),
            ctx: CallContext::new(Origin::Official),
        };
        assert!(matches!(
            net_fetch(&call),
            Err(CapabilityError::InvalidArgs { .. })
        ));
    }
}
