use super::*;
use rustls::pki_types::pem::PemObject;
use serde_json::{json, Value};
use std::{io::BufReader as StdBufReader, sync::Arc, time::Duration};
use tezbar_extension_sdk::{Extension, ExtensionContext, ExtensionPaths, TestContext};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
fn own_dir() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("qa-")
        .tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.qa/tmp"))
        .unwrap()
}
fn context(dir: &std::path::Path) -> TestContext {
    TestContext::new(
        ExtensionPaths {
            package_dir: dir.into(),
            data_dir: dir.join("data"),
            temp_dir: dir.join("temp"),
        },
        &["clipboard.write"],
    )
}
fn payload(id: &str) -> Value {
    json!({"sessionId":id,"preferences":{"provider":"exchangerate-api","api_key":"owned-fixture-key"},"argumentValues":{}})
}
async fn handle(a: &Adapter, c: &ExtensionContext, op: &str, p: Value) -> Value {
    a.handle(c, "index", op, &p, &mut json!({})).await.unwrap()
}
fn rows(v: &Value) -> Vec<&Value> {
    v["root"]["children"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| n["type"] == "List.Section")
        .flat_map(|n| n["children"].as_array().unwrap())
        .collect()
}
fn action(v: &Value, title: &str) -> String {
    v["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["title"] == title)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .into()
}
fn invoke(v: &Value, id: &str, action: String, fields: Value) -> Value {
    let mut fields = fields.as_object().cloned().unwrap_or_default();
    for (key, value) in [
        ("__tezbarViewToken", v["root"]["props"]["viewToken"].clone()),
        (
            "__tezbarSearchText",
            v["root"]["props"]["searchText"].clone(),
        ),
        (
            "__tezbarFromCurrency",
            v["root"]["props"]["fromCurrencyCode"].clone(),
        ),
    ] {
        fields.insert(key.into(), value);
    }
    json!({"sessionId":id,"actionId":action,"formValues":fields})
}
fn seed(ctx: &ExtensionContext, p: &Value) {
    let key = provider::Preferences::parse(&p["preferences"])
        .unwrap()
        .identity(None);
    let r=provider::normalize(provider::Provider::ExchangeRate,200,json!({"result":"success","base_code":"USD","conversion_rates":{"USD":1,"JPY":150,"EUR":0.9,"GBP":0.8},"time_last_update_unix":1700000000,"time_next_update_unix":4102444800_i64,"time_last_update_utc":"Tue, 14 Nov 2023 22:13:20 GMT"}),None,0).unwrap();
    storage::cache(ctx, &key, &r).unwrap();
}
#[tokio::test]
async fn actions_tokens_repeat_copy_unknown_ack_forms_and_storage() {
    let dir = own_dir();
    let c = context(dir.path());
    let a = Adapter::new().unwrap();
    let mut p = payload("s");
    seed(&c.context, &p);
    p["searchText"] = json!("10");
    let mut v = handle(&a, &c.context, "extension:run-command", p).await;
    assert_eq!(rows(&v)[0]["props"]["title"], "JPY");
    let copy = action(&v, "Copy to Clipboard");
    let request = invoke(&v, "s", copy.clone(), json!({}));
    let mut missing = request.clone();
    missing["formValues"] = json!({});
    assert_eq!(
        handle(&a, &c.context, "extension:invoke-action", missing).await["mode"],
        "unchanged"
    );
    assert!(c.calls().is_empty());
    c.reply(Ok(json!({"ok":true})));
    v = c
        .run(handle(
            &a,
            &c.context,
            "extension:invoke-action",
            request.clone(),
        ))
        .await;
    assert_eq!(v["message"], "Copied to Clipboard");
    assert_ne!(action(&v, "Copy to Clipboard"), copy);
    assert_eq!(c.calls().len(), 1);
    assert_eq!(
        handle(&a, &c.context, "extension:invoke-action", request).await["mode"],
        "unchanged"
    );
    assert_eq!(c.calls().len(), 1);
    for bad in [
        Value::Null,
        json!(true),
        json!({"ok":true,"error":"contradiction"}),
        json!({"ok":true,"unknown":1}),
        json!({"ok":false}),
        json!({"ok":true,"message":null}),
    ] {
        let req = invoke(&v, "s", action(&v, "Copy to Clipboard"), json!({}));
        c.reply(Ok(bad));
        v = c
            .run(handle(&a, &c.context, "extension:invoke-action", req))
            .await;
        assert!(v["message"].as_str().unwrap().contains("unknown"));
        let exact = c.calls().last().unwrap()["params"].clone();
        let retry = invoke(
            &v,
            "s",
            action(&v, "Retry exact clipboard write"),
            json!({}),
        );
        c.reply(Ok(json!({"ok":true,"message":"fine"})));
        v = c
            .run(handle(&a, &c.context, "extension:invoke-action", retry))
            .await;
        assert_eq!(exact, c.calls().last().unwrap()["params"]);
        assert!(v["actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x["title"] == "Copy to Clipboard"));
    }
    let req = invoke(&v, "s", action(&v, "Pin It"), json!({}));
    v = handle(&a, &c.context, "extension:invoke-action", req).await;
    assert_eq!(rows(&v)[0]["props"]["id"], "pin:JPY");
    let req = invoke(&v, "s", action(&v, "From Currency"), json!({"value":"EUR"}));
    v = handle(&a, &c.context, "extension:invoke-action", req).await;
    assert_eq!(v["root"]["props"]["fromCurrencyCode"], "EUR");
    assert_eq!(v["root"]["props"]["searchText"], "10");
    let req = invoke(&v, "s", action(&v, "Set Currency Date"), json!({}));
    v = handle(&a, &c.context, "extension:invoke-action", req).await;
    assert_eq!(v["root"]["type"], "Form");
    let req = invoke(
        &v,
        "s",
        action(&v, "Set to Date"),
        json!({"historyDate":""}),
    );
    v = handle(&a, &c.context, "extension:invoke-action", req).await;
    assert_eq!(v["root"]["type"], "List");
    assert_eq!(v["root"]["props"]["searchText"], "10");
    let req = invoke(&v, "s", action(&v, "Set Currency Date"), json!({}));
    v = handle(&a, &c.context, "extension:invoke-action", req).await;
    let req = invoke(
        &v,
        "s",
        action(&v, "Set to Date"),
        json!({"historyDate":"2023-02-31"}),
    );
    v = handle(&a, &c.context, "extension:invoke-action", req).await;
    assert_eq!(v["root"]["type"], "Form");
    let req = invoke(&v, "s", action(&v, "Cancel"), json!({}));
    v = handle(&a, &c.context, "extension:invoke-action", req).await;
    assert_eq!(v["root"]["props"]["searchText"], "10");
}
#[test]
fn strict_closed_clipboard_ack() {
    for v in [json!({"ok":true}), json!({"ok":true,"message":"done"})] {
        assert!(crate::adapter::clipboard_ack(&v));
    }
    for v in [
        Value::Null,
        json!(false),
        json!({"ok":true,"error":null}),
        json!({"ok":true,"message":5}),
        json!({}),
    ] {
        assert!(!crate::adapter::clipboard_ack(&v));
    }
}
#[tokio::test]
async fn bounded_atomic_corruption_unknown_and_concurrent_storage() {
    let dir = own_dir();
    let c = Arc::new(context(dir.path()));
    storage::access(&c.context, true, |v| {
        v["unknown"] = json!({"keep":[1,2]});
        v["pinnedCurrencyCode"] = json!(["EUR", "EUR", "ZZZ"]);
        Ok(())
    })
    .unwrap();
    assert_eq!(storage::settings(&c.context).unwrap().1, vec!["EUR", "EUR"]);
    let mut jobs = vec![];
    for code in ["USD", "JPY", "GBP", "AED", "CAD", "CHF"] {
        let c = c.clone();
        jobs.push(tokio::spawn(async move {
            storage::toggle_pin(&c.context, code).unwrap()
        }));
    }
    for job in jobs {
        job.await.unwrap();
    }
    assert_eq!(storage::settings(&c.context).unwrap().1.len(), 8);
    storage::toggle_pin(&c.context, "EUR").unwrap();
    assert_eq!(storage::settings(&c.context).unwrap().1.len(), 6);
    assert_eq!(
        storage::access(&c.context, false, |v| Ok(v["unknown"].clone())).unwrap(),
        json!({"keep":[1,2]})
    );
    let p = payload("s");
    seed(&c.context, &p);
    let r = storage::cached(
        &c.context,
        &provider::Preferences::parse(&p["preferences"])
            .unwrap()
            .identity(None),
    )
    .unwrap()
    .unwrap();
    for i in 0..50 {
        storage::cache(&c.context, &format!("owned:{i}"), &r).unwrap();
    }
    assert_eq!(
        storage::access(&c.context, false, |v| Ok(v["cache"]
            .as_object()
            .unwrap()
            .len()))
        .unwrap(),
        32
    );
    let path = dir.path().join("data/currency-exchange/storage.json");
    std::fs::write(&path, b"{broken").unwrap();
    let a = Adapter::new().unwrap();
    let v = handle(&a, &c.context, "extension:run-command", payload("s")).await;
    assert_eq!(v["root"]["type"], "List");
    assert!(v["root"]["props"]["hasSearchTextChangeHandler"]
        .as_bool()
        .unwrap());
    assert_eq!(std::fs::read(&path).unwrap(), b"{broken");
    assert!(storage::set_from(&c.context, "JPY").is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"{broken");
    std::fs::write(&path, vec![b' '; 1024 * 1024 + 1]).unwrap();
    assert!(storage::settings(&c.context).is_err());
}
struct Server {
    port: u16,
    seen: tokio::sync::mpsc::UnboundedReceiver<()>,
    task: tokio::task::JoinHandle<Vec<String>>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn server(responses: Vec<(u16, String, Duration)>) -> Server {
    use rustls::pki_types::{CertificateDer, PrivateKeyDer};
    let cert = CertificateDer::from_pem_reader(&mut StdBufReader::new(
        include_bytes!("../tests/fixtures/localhost-cert.pem").as_slice(),
    ))
    .unwrap();
    let key = PrivateKeyDer::from_pem_reader(&mut StdBufReader::new(
        include_bytes!("../tests/fixtures/localhost-key.pem").as_slice(),
    ))
    .unwrap();
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(vec![cert], key)
    .unwrap();
    let tls = tokio_rustls::TlsAcceptor::from(Arc::new(config));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let (seen_tx, seen) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        let mut captures = vec![];
        for (status, body, delay) in responses {
            let (socket, _) = listener.accept().await.unwrap();
            let Ok(mut socket) = tls.accept(socket).await else {
                continue;
            };
            let mut request = vec![];
            loop {
                let mut b = [0];
                if socket.read_exact(&mut b).await.is_err() {
                    break;
                }
                request.push(b[0]);
                if request.ends_with(b"\r\n\r\n") {
                    break;
                }
                assert!(request.len() < 16384);
            }
            captures.push(String::from_utf8(request).unwrap());
            let _ = seen_tx.send(());
            tokio::time::sleep(delay).await;
            let header = if status == 299 {
                format!("HTTP/1.1 200 Fixture\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n{:X}\r\n",body.len())
            } else if status == 302 {
                format!("HTTP/1.1 302 Found\r\nLocation: https://example.invalid/deny\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len())
            } else {
                format!("HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n",body.len())
            };
            let _ = socket.write_all(header.as_bytes()).await;
            let _ = socket.write_all(body.as_bytes()).await;
            if status == 299 {
                let _ = socket.write_all(b"\r\n0\r\n\r\n").await;
            }
            let _ = socket.shutdown().await;
        }
        captures
    });
    Server { port, seen, task }
}
fn transport(server: &Server) -> provider::Transport {
    provider::Transport::owned_tls(
        &format!("https://localhost:{}", server.port),
        include_bytes!("../tests/fixtures/localhost-ca.pem"),
    )
}
#[tokio::test]
async fn actual_reqwest_owned_tls_success_auth_history_errors_limits() {
    let dir = own_dir();
    let c = context(dir.path());
    let date = domain::Date {
        year: 2023,
        month: 2,
        day: 28,
    };
    for provider in ["exchangerate-api", "unirate"] {
        let p =
            provider::Preferences::parse(&json!({"provider":provider,"api_key":"owned/key &雪"}))
                .unwrap();
        let body = if provider == "unirate" {
            json!({"base":"USD","rates":{"USD":"1","EUR":".9"}})
        } else {
            json!({"result":"success","base_code":"USD","year":2023,"month":2,"day":28,"conversion_rates":{"USD":1,"EUR":0.9}})
        };
        let mut srv = server(vec![(200, body.to_string(), Duration::ZERO)]).await;
        let r = transport(&srv)
            .fetch(&c.context, &p, Some(date))
            .await
            .unwrap();
        assert_eq!(r.rates["EUR"].as_f64(), Some(0.9));
        let requests = (&mut srv.task).await.unwrap();
        assert_eq!(requests.len(), 1);
        let line = requests[0].lines().next().unwrap();
        if provider == "unirate" {
            assert!(line.starts_with("GET /api/historical/rates?from=USD&date=2023-02-28&api_key=owned%2Fkey%20%26%E9%9B%AA HTTP/1.1"));
        } else {
            assert!(
                line.starts_with("GET /v6/owned%2Fkey%20&%E9%9B%AA/history/USD/2023/2/28 HTTP/1.1")
            );
        }
    }
    for (status, body) in [
        (401, "owned-fixture-key".into()),
        (429, "owned-fixture-key".into()),
        (302, "owned-fixture-key".into()),
        (500, "owned-fixture-key".into()),
        (200, "<html>owned-fixture-key".into()),
        (200, "x".repeat(provider::MAX_BODY + 1)),
    ] {
        let mut srv = server(vec![(status, body, Duration::ZERO)]).await;
        let p = provider::Preferences::parse(&json!({"api_key":"owned-fixture-key"})).unwrap();
        let e = transport(&srv)
            .fetch(&c.context, &p, None)
            .await
            .unwrap_err();
        assert!(!e.contains("owned-fixture-key"));
        assert!(!e.contains("https://"));
        (&mut srv.task).await.unwrap();
    }
}

struct Shared(Arc<Adapter>);
impl Extension for Shared {
    fn id(&self) -> &'static str {
        self.0.id()
    }
    fn version(&self) -> &'static str {
        self.0.version()
    }
    fn commands(&self) -> &'static [&'static str] {
        self.0.commands()
    }
    fn handle<'a>(
        &'a self,
        c: &'a ExtensionContext,
        n: &'a str,
        o: &'a str,
        p: &'a Value,
        s: &'a mut Value,
    ) -> tezbar_extension_sdk::BoxFuture<'a, tezbar_extension_sdk::CoreResult> {
        self.0.handle(c, n, o, p, s)
    }
}
struct Pipes {
    write: tokio::io::WriteHalf<tokio::io::DuplexStream>,
    read: BufReader<tokio::io::ReadHalf<tokio::io::DuplexStream>>,
    task: tokio::task::JoinHandle<Result<(), String>>,
    seq: u64,
}
impl Drop for Pipes {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Pipes {
    async fn new(a: Arc<Adapter>, dir: &std::path::Path) -> Self {
        let (host, child) = tokio::io::duplex(1024 * 1024);
        let (input, output) = tokio::io::split(child);
        let (read, write) = tokio::io::split(host);
        let task = tokio::spawn(tezbar_extension_sdk::serve_io(
            Shared(a),
            BufReader::new(input),
            output,
        ));
        let mut p = Self {
            write,
            read: BufReader::new(read),
            task,
            seq: 0,
        };
        assert_eq!(p.recv().await["type"], "hello");
        let id=p.send("initialize",json!({"protocol":1,"extensionId":"com.tezbar.native.currency-exchange","platform":"macOS","packageDir":dir,"dataDir":dir.join("data"),"tempDir":dir.join("temp"),"grantedCapabilities":["clipboard.write"],"migrationPaths":{}})).await;
        assert_eq!(p.response(&id).await["result"]["ready"], true);
        p
    }
    async fn frame(&mut self, v: Value) {
        self.write
            .write_all(format!("{v}\n").as_bytes())
            .await
            .unwrap();
    }
    async fn send(&mut self, method: &str, params: Value) -> String {
        self.seq += 1;
        let id = format!("h:00000000-0000-4000-8000-{:012}", self.seq);
        self.frame(json!({"type":"request","id":id,"method":method,"params":params}))
            .await;
        id
    }
    async fn recv(&mut self) -> Value {
        let mut line = String::new();
        let n = tokio::time::timeout(Duration::from_secs(6), self.read.read_line(&mut line))
            .await
            .unwrap()
            .unwrap();
        assert!(n > 0);
        serde_json::from_str(&line).unwrap()
    }
    async fn response(&mut self, id: &str) -> Value {
        loop {
            let v = self.recv().await;
            if v["id"] == id {
                return v;
            }
            assert!(
                v["type"] == "cancel" || v["type"] == "event",
                "unexpected frame {v}"
            );
        }
    }
    async fn call(&mut self, op: &str, p: Value) -> String {
        let sid = p["sessionId"].clone();
        self.send(
            "session.call",
            json!({"sessionId":sid,"operation":op,"payload":p}),
        )
        .await
    }
    async fn close(&mut self) {
        self.write.shutdown().await.unwrap();
        assert!(tokio::time::timeout(Duration::from_secs(3), &mut self.task)
            .await
            .unwrap()
            .unwrap()
            .is_ok());
    }
}
#[tokio::test]
async fn sdk_jsonl_unknown_ack_cancel_late_ack_repeat_copy_dropdown_pending_and_eof() {
    let dir = own_dir();
    let ctx = context(dir.path());
    let a = Arc::new(Adapter::new().unwrap());
    let mut p = payload("wire");
    seed(&ctx.context, &p);
    p["commandName"] = json!("index");
    p["searchText"] = json!("10");
    let mut pipe = Pipes::new(a, dir.path()).await;
    let id = pipe.send("session.open", p).await;
    let mut v = pipe.response(&id).await["result"].clone();
    let req = invoke(&v, "wire", action(&v, "Copy to Clipboard"), json!({}));
    let id = pipe.call("extension:invoke-action", req).await;
    let reverse = pipe.recv().await;
    assert_eq!(reverse["method"], "host.clipboard.write");
    pipe.frame(json!({"type":"cancel","requestId":id,"reason":"owned cancel"}))
        .await;
    assert!(pipe.response(&id).await.get("error").is_some());
    pipe.frame(json!({"type":"response","id":reverse["id"],"result":{"ok":true}}))
        .await;
    let id = pipe
        .call("extension:refresh-session", json!({"sessionId":"wire"}))
        .await;
    v = pipe.response(&id).await["result"].clone();
    let id = pipe
        .call(
            "extension:invoke-action",
            invoke(
                &v,
                "wire",
                action(&v, "Retry exact clipboard write"),
                json!({}),
            ),
        )
        .await;
    let retry = loop {
        let m = pipe.recv().await;
        if m["type"] != "cancel" {
            break m;
        }
    };
    assert_eq!(retry["params"], reverse["params"]);
    pipe.frame(json!({"type":"response","id":retry["id"],"result":{"ok":true}}))
        .await;
    v = pipe.response(&id).await["result"].clone();
    let old = invoke(&v, "wire", action(&v, "Copy to Clipboard"), json!({}));
    let id = pipe.call("extension:invoke-action", old.clone()).await;
    let reverse = pipe.recv().await;
    assert_eq!(reverse["method"], "host.clipboard.write");
    pipe.frame(json!({"type":"response","id":reverse["id"],"result":{"ok":true}}))
        .await;
    v = pipe.response(&id).await["result"].clone();
    assert_ne!(old["actionId"], action(&v, "Copy to Clipboard"));
    let id = pipe.call("extension:invoke-action", old).await;
    assert_eq!(pipe.response(&id).await["result"]["mode"], "unchanged");
    let stale = invoke(
        &v,
        "wire",
        action(&v, "From Currency"),
        json!({"value":"JPY"}),
    );
    let id = pipe
        .call(
            "extension:search-text-changed",
            json!({"sessionId":"wire","searchText":"20 in eur"}),
        )
        .await;
    v = pipe.response(&id).await["result"].clone();
    let id = pipe.call("extension:invoke-action", stale).await;
    assert_eq!(pipe.response(&id).await["result"]["mode"], "unchanged");
    assert_eq!(v["root"]["props"]["searchText"], "20 in eur");
    let id = pipe
        .call(
            "extension:invoke-action",
            invoke(&v, "wire", action(&v, "Copy to Clipboard"), json!({})),
        )
        .await;
    let reverse = pipe.recv().await;
    assert_eq!(reverse["method"], "host.clipboard.write");
    assert!(id.starts_with("h:"));
    pipe.close().await;
}
#[tokio::test]
async fn sdk_capacity_128_dispose_and_140_canceled_open_prune() {
    let dir = own_dir();
    let adapter = Arc::new(Adapter::new().unwrap());
    let mut pipe = Pipes::new(adapter.clone(), dir.path()).await;
    for i in 0..140 {
        let mut p = payload(&format!("cancel-{i}"));
        p["commandName"] = json!("index");
        let id = pipe.send("session.open", p).await;
        pipe.frame(json!({"type":"cancel","requestId":id,"reason":"owned canceled open"}))
            .await;
        let r = pipe.response(&id).await;
        if r.get("result").is_some() {
            let id = pipe
                .send(
                    "session.dispose",
                    json!({"sessionId":format!("cancel-{i}")}),
                )
                .await;
            pipe.response(&id).await;
        }
        assert_eq!(adapter.session_count(), 0);
    }
    for i in 0..128 {
        let mut p = payload(&format!("capacity-{i}"));
        p["commandName"] = json!("index");
        let id = pipe.send("session.open", p).await;
        assert_eq!(pipe.response(&id).await["result"]["mode"], "view");
    }
    let mut p = payload("overflow");
    p["commandName"] = json!("index");
    let id = pipe.send("session.open", p).await;
    assert!(pipe.response(&id).await.get("error").is_some());
    assert_eq!(adapter.session_count(), 128);
    for i in 0..128 {
        let id = pipe
            .send(
                "session.dispose",
                json!({"sessionId":format!("capacity-{i}")}),
            )
            .await;
        assert_eq!(pipe.response(&id).await["result"]["mode"], "unchanged");
    }
    assert_eq!(adapter.session_count(), 0);
    pipe.close().await;
}
fn rate_body(eur: f64) -> String {
    json!({"result":"success","base_code":"USD","conversion_rates":{"USD":1,"EUR":eur,"JPY":150},"time_last_update_unix":1700000000,"time_next_update_unix":4102444800_i64,"time_last_update_utc":"Tue, 14 Nov 2023 22:13:20 GMT"}).to_string()
}
#[tokio::test]
async fn tls_sdk_latest_a_b_a_cancellation_cache_and_stale_action_fences() {
    let dir = own_dir();
    let mut srv = server(vec![
        (200, rate_body(111.0), Duration::from_millis(120)),
        (200, rate_body(222.0), Duration::from_millis(120)),
        (200, rate_body(0.9), Duration::ZERO),
    ])
    .await;
    let a = Arc::new(Adapter::with_transport(transport(&srv)));
    let mut pipe = Pipes::new(a, dir.path()).await;
    let mut p = payload("latest");
    p["commandName"] = json!("index");
    let id = pipe.send("session.open", p).await;
    let old = pipe.response(&id).await["result"].clone();
    let first = pipe
        .call(
            "extension:search-text-changed",
            json!({"sessionId":"latest","searchText":"10 in eur"}),
        )
        .await;
    tokio::time::timeout(Duration::from_secs(3), srv.seen.recv())
        .await
        .unwrap()
        .unwrap();
    let second = pipe
        .call(
            "extension:search-text-changed",
            json!({"sessionId":"latest","searchText":"20 in eur"}),
        )
        .await;
    assert!(pipe.response(&first).await.get("error").is_some());
    tokio::time::timeout(Duration::from_secs(3), srv.seen.recv())
        .await
        .unwrap()
        .unwrap();
    let third = pipe
        .call(
            "extension:search-text-changed",
            json!({"sessionId":"latest","searchText":"10 in eur"}),
        )
        .await;
    assert!(pipe.response(&second).await.get("error").is_some());
    let v = pipe.response(&third).await["result"].clone();
    assert_eq!(v["root"]["props"]["searchText"], "10 in eur");
    assert_eq!(rows(&v)[0]["props"]["subtitle"], "€9");
    let id = pipe
        .call(
            "extension:invoke-action",
            invoke(
                &old,
                "latest",
                action(&old, "From Currency"),
                json!({"value":"JPY"}),
            ),
        )
        .await;
    assert_eq!(pipe.response(&id).await["result"]["mode"], "unchanged");
    let ctx = context(dir.path());
    let key = provider::Preferences::parse(&payload("latest")["preferences"])
        .unwrap()
        .identity(None);
    assert_eq!(
        storage::cached(&ctx.context, &key).unwrap().unwrap().rates["EUR"].as_f64(),
        Some(0.9)
    );
    assert_eq!((&mut srv.task).await.unwrap().len(), 3);
    pipe.close().await;
}
#[tokio::test]
async fn tls_cancel_after_open_allocation_prunes_authoritative_state() {
    let dir = own_dir();
    let mut srv = server(
        (0..140)
            .map(|_| (200, rate_body(0.9), Duration::from_millis(3)))
            .collect(),
    )
    .await;
    let a = Arc::new(Adapter::with_transport(transport(&srv)));
    let mut pipe = Pipes::new(a.clone(), dir.path()).await;
    for i in 0..140 {
        let mut p = payload(&format!("allocated-{i}"));
        p["commandName"] = json!("index");
        p["searchText"] = json!("10");
        let id = pipe.send("session.open", p).await;
        tokio::time::timeout(Duration::from_secs(4), srv.seen.recv())
            .await
            .unwrap()
            .unwrap();
        pipe.frame(json!({"type":"cancel","requestId":id,"reason":"owned allocated-open cancel"}))
            .await;
        assert!(pipe.response(&id).await.get("error").is_some());
        assert_eq!(a.session_count(), 0);
    }
    assert_eq!((&mut srv.task).await.unwrap().len(), 140);
    let ctx = context(dir.path());
    assert!(storage::cached(
        &ctx.context,
        &provider::Preferences::parse(&payload("x")["preferences"])
            .unwrap()
            .identity(None)
    )
    .unwrap()
    .is_none());
    pipe.close().await;
}
#[tokio::test]
async fn invalid_cache_recovery_history_auth_isolation_and_http_failure_not_fresh() {
    let dir = own_dir();
    let ctx = context(dir.path());
    storage::access(&ctx.context, true, |v| {
        v["unknown"] = json!("retained");
        v["cache"] = json!(false);
        Ok(())
    })
    .unwrap();
    let mut srv=server(vec![(200,rate_body(0.9),Duration::ZERO),(500,"owned-fixture-key".into(),Duration::ZERO),(200,json!({"result":"success","base_code":"USD","year":2023,"month":2,"day":28,"conversion_rates":{"USD":1,"EUR":0.5}}).to_string(),Duration::ZERO)]).await;
    let a = Adapter::with_transport(transport(&srv));
    let mut p = payload("s");
    p["searchText"] = json!("10 in eur");
    let v = handle(&a, &ctx.context, "extension:run-command", p.clone()).await;
    assert_eq!(rows(&v)[0]["props"]["subtitle"], "€9");
    let mut other = payload("other");
    other["preferences"]["api_key"] = json!("another-owned-key");
    other["searchText"] = json!("20");
    let v = handle(&a, &ctx.context, "extension:run-command", other.clone()).await;
    assert!(rows(&v).is_empty());
    assert!(v["message"].as_str().unwrap().contains("failed"));
    let otherkey = provider::Preferences::parse(&other["preferences"])
        .unwrap()
        .identity(None);
    assert!(storage::cached(&ctx.context, &otherkey).unwrap().is_none());
    let v = handle(
        &a,
        &ctx.context,
        "extension:search-text-changed",
        json!({"sessionId":"s","searchText":"10 in eur at 2023/2/28"}),
    )
    .await;
    assert_eq!(rows(&v)[0]["props"]["subtitle"], "€5");
    let v = handle(
        &a,
        &ctx.context,
        "extension:search-text-changed",
        json!({"sessionId":"s","searchText":"20 in eur at 2023/2/28"}),
    )
    .await;
    assert_eq!(rows(&v)[0]["props"]["subtitle"], "€10");
    assert_eq!(
        storage::access(&ctx.context, false, |v| Ok(v["unknown"].clone())).unwrap(),
        "retained"
    );
    assert_eq!((&mut srv.task).await.unwrap().len(), 3);
}
#[tokio::test]
async fn tls_untrusted_certificate_and_streamed_body_limit() {
    let dir = own_dir();
    let c = context(dir.path());
    let p = provider::Preferences::parse(&json!({"api_key":"owned-fixture-key"})).unwrap();
    let mut srv = server(vec![(200, rate_body(0.9), Duration::ZERO)]).await;
    let e = provider::Transport::untrusted_owned_tls(&format!("https://localhost:{}", srv.port))
        .fetch(&c.context, &p, None)
        .await
        .unwrap_err();
    assert_eq!(e, "HTTPS request failed; retry manually.");
    assert!((&mut srv.task).await.unwrap().is_empty());
    let mut srv = server(vec![(
        299,
        "x".repeat(provider::MAX_BODY + 1),
        Duration::ZERO,
    )])
    .await;
    let e = transport(&srv)
        .fetch(&c.context, &p, None)
        .await
        .unwrap_err();
    assert!(e.contains("size limit"));
    (&mut srv.task).await.unwrap();
}
#[tokio::test]
async fn concurrent_session_base_selection_is_local_while_default_persists() {
    let dir = own_dir();
    let c = context(dir.path());
    let a = Adapter::new().unwrap();
    let mut p = payload("a");
    seed(&c.context, &p);
    p["searchText"] = json!("10");
    let old_a = handle(&a, &c.context, "extension:run-command", p).await;
    let b = handle(&a, &c.context, "extension:run-command", payload("b")).await;
    let b = handle(
        &a,
        &c.context,
        "extension:invoke-action",
        invoke(&b, "b", action(&b, "From Currency"), json!({"value":"EUR"})),
    )
    .await;
    assert_eq!(b["root"]["props"]["fromCurrencyCode"], "EUR");
    let av = handle(
        &a,
        &c.context,
        "extension:search-text-changed",
        json!({"sessionId":"a","searchText":"10 in eur"}),
    )
    .await;
    assert_eq!(av["root"]["props"]["fromCurrencyCode"], "USD");
    assert_eq!(rows(&av)[0]["props"]["subtitle"], "€9");
    let cv = handle(&a, &c.context, "extension:run-command", payload("c")).await;
    assert_eq!(cv["root"]["props"]["fromCurrencyCode"], "EUR");
    assert_ne!(
        old_a["root"]["props"]["viewToken"],
        av["root"]["props"]["viewToken"]
    );
}
#[tokio::test]
async fn empty_date_pop_keeps_prepared_list_without_refetch_even_after_cache_expiry() {
    let dir = own_dir();
    let c = context(dir.path());
    let mut srv = server(vec![]).await;
    let a = Adapter::with_transport(transport(&srv));
    let mut p = payload("date-pop");
    seed(&c.context, &p);
    p["searchText"] = json!("10 in eur");
    let v = handle(&a, &c.context, "extension:run-command", p.clone()).await;
    let old = rows(&v)[0]["props"]["subtitle"].clone();
    let form = handle(
        &a,
        &c.context,
        "extension:invoke-action",
        invoke(&v, "date-pop", action(&v, "Set Currency Date"), json!({})),
    )
    .await;
    let key = provider::Preferences::parse(&p["preferences"])
        .unwrap()
        .identity(None);
    storage::access(&c.context, true, |v| {
        v["cache"][&key]["data"]["next"] = json!(0);
        Ok(())
    })
    .unwrap();
    assert!((&mut srv.task).await.unwrap().is_empty());
    let v = handle(
        &a,
        &c.context,
        "extension:invoke-action",
        invoke(
            &form,
            "date-pop",
            action(&form, "Set to Date"),
            json!({"historyDate":""}),
        ),
    )
    .await;
    assert_eq!(v["root"]["type"], "List");
    assert_eq!(v["root"]["props"]["searchText"], "10 in eur");
    assert_eq!(rows(&v)[0]["props"]["subtitle"], old);
    assert!(v.get("message").is_none());
}
