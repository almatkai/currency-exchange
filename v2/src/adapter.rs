use crate::{
    domain::{self, Date, Query},
    provider::{self, Preferences, Rates, Transport},
    storage,
};
use base64::Engine;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    task::{Context, Poll, Waker},
};
use tezbar_extension_sdk::{unchanged, BoxFuture, CoreResult, Extension, ExtensionContext};
const ID: &str = "com.tezbar.native.currency-exchange";
static SEQUENCE: AtomicU64 = AtomicU64::new(1);
fn token() -> String {
    format!("ce:{}", SEQUENCE.fetch_add(1, Ordering::Relaxed))
}
#[derive(Clone)]
enum Action {
    Copy(String),
    RetryCopy(String),
    Pin(String),
    Date,
    SubmitDate,
    Back,
    From,
    Retry,
}
#[derive(Clone)]
struct PendingCopy {
    text: String,
    checkpoint: String,
}
struct Session {
    query: String,
    parsed: Query,
    from: String,
    pins: Vec<String>,
    prefs: Result<Preferences, String>,
    rates: Option<Rates>,
    error: Option<String>,
    notice: Option<String>,
    form: bool,
    date: Option<Date>,
    view_token: String,
    actions: HashMap<String, Action>,
    pending: Option<PendingCopy>,
}
struct Entry {
    session: Arc<Mutex<Session>>,
    cancelled: Pin<Box<dyn Future<Output = ()> + Send>>,
}
impl Entry {
    fn dead(&mut self) -> bool {
        matches!(
            self.cancelled
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop())),
            Poll::Ready(())
        )
    }
}
pub struct Adapter {
    sessions: Mutex<HashMap<String, Entry>>,
    transport: Transport,
}
impl Adapter {
    #[cfg(test)]
    pub(crate) fn session_count(&self) -> usize {
        let mut s = self.sessions.lock().unwrap();
        s.retain(|_, e| !e.dead());
        s.len()
    }
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            sessions: Mutex::default(),
            transport: Transport::new()?,
        })
    }
    #[cfg(test)]
    pub(crate) fn with_transport(transport: Transport) -> Self {
        Self {
            sessions: Mutex::default(),
            transport,
        }
    }
    fn get(&self, id: &str) -> Result<Arc<Mutex<Session>>, String> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "Session state unavailable")?;
        sessions.retain(|_, e| !e.dead());
        sessions
            .get(id)
            .map(|e| e.session.clone())
            .ok_or_else(|| "Session expired; reopen the command.".into())
    }
    fn open(&self, ctx: &ExtensionContext, p: &Value) -> Result<Arc<Mutex<Session>>, String> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "Session state unavailable")?;
        sessions.retain(|_, e| !e.dead());
        let id = p["sessionId"].as_str().ok_or("Missing session identity")?;
        if sessions.len() >= 128 || sessions.contains_key(id) {
            return Err("Session capacity reached; close an existing command.".into());
        }
        let prefs = Preferences::parse(&p["preferences"]);
        let settings = storage::settings(ctx);
        let error = settings
            .as_ref()
            .err()
            .cloned()
            .or_else(|| prefs.as_ref().err().cloned());
        let (from, pins) = settings.unwrap_or_else(|_| ("USD".into(), vec![]));
        let query = p["searchText"].as_str().unwrap_or("").to_owned();
        if query.len() > 4096 {
            return Err("Search is too long".into());
        }
        let session = Arc::new(Mutex::new(Session {
            parsed: domain::parse(&query),
            query,
            from,
            pins,
            prefs,
            rates: None,
            error,
            notice: None,
            form: false,
            date: None,
            view_token: String::new(),
            actions: HashMap::new(),
            pending: None,
        }));
        let owned = ctx.clone();
        sessions.insert(
            id.into(),
            Entry {
                session: session.clone(),
                cancelled: Box::pin(async move { owned.cancelled().await }),
            },
        );
        Ok(session)
    }
    async fn exchange(&self, ctx: &ExtensionContext, s: &Arc<Mutex<Session>>) -> CoreResult {
        alive(ctx).await?;
        let (prefs, parsed) = {
            let mut v = s.lock().map_err(|_| "Session state unavailable")?;
            v.actions.clear();
            v.rates = None;
            v.notice = None;
            v.error = None;
            match storage::settings(ctx) {
                Ok((_, pins)) => {
                    v.pins = pins;
                }
                Err(e) => v.error = Some(e),
            }
            (v.prefs.clone(), v.parsed.clone())
        };
        let result = if parsed.amount <= 0.0 {
            Ok(None)
        } else {
            match prefs {
                Err(e) => Err(e),
                Ok(p) => {
                    let key = p.identity(parsed.date);
                    match storage::cached(ctx, &key) {
                        Err(e) => Err(e),
                        Ok(Some(r)) if r.valid_at(parsed.date, provider::now()) => {
                            Ok(Some((r, false, key)))
                        }
                        _ => self
                            .transport
                            .fetch(ctx, &p, parsed.date)
                            .await
                            .map(|r| Some((r, true, key))),
                    }
                }
            }
        };
        // This is the storage/UI commit linearization point. No await occurs between
        // the cancellation check and the synchronous bounded atomic update.
        alive(ctx).await?;
        let mut v = s.lock().map_err(|_| "Session state unavailable")?;
        match result {
            Ok(Some((r, fresh, key))) => {
                if fresh {
                    if let Err(e) = storage::cache(ctx, &key, &r) {
                        v.error = Some(e);
                    }
                }
                v.rates = Some(r);
            }
            Ok(None) => {}
            Err(e) => v.error = Some(e),
        }
        Ok(render(&mut v))
    }
    async fn copy(
        &self,
        ctx: &ExtensionContext,
        s: &Arc<Mutex<Session>>,
        text: String,
    ) -> CoreResult {
        alive(ctx).await?;
        let checkpoint = token();
        {
            let mut v = s.lock().map_err(|_| "Session state unavailable")?;
            v.actions.clear();
            v.pending = Some(PendingCopy {
                text: text.clone(),
                checkpoint: checkpoint.clone(),
            });
            v.notice=Some("Clipboard acknowledgement is unknown. Retry explicitly to write these exact bytes again.".into());
        }
        let reply = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            ctx.host("host.clipboard.write", json!({"text":text})),
        )
        .await;
        alive(ctx).await?;
        let mut v = s.lock().map_err(|_| "Session state unavailable")?;
        if v.pending
            .as_ref()
            .is_none_or(|p| p.checkpoint != checkpoint)
        {
            return Ok(unchanged());
        }
        if matches!(reply,Ok(Ok(ref value)) if clipboard_ack(value)) {
            v.pending = None;
            v.notice = Some("Copied to Clipboard".into());
        } else {
            v.notice=Some("Clipboard acknowledgement is unknown. Retry explicitly to write these exact bytes again.".into());
        }
        Ok(render(&mut v))
    }
}
async fn alive(ctx: &ExtensionContext) -> Result<(), String> {
    tokio::select! {biased;_=ctx.cancelled()=>Err("Operation cancelled".into()),_=async{}=>Ok(())}
}
pub(crate) fn clipboard_ack(v: &Value) -> bool {
    v.as_object().is_some_and(|o| {
        o.get("ok") == Some(&json!(true))
            && o.keys().all(|k| k == "ok" || k == "message")
            && o.get("message").is_none_or(Value::is_string)
    })
}
impl Extension for Adapter {
    fn id(&self) -> &'static str {
        ID
    }
    fn version(&self) -> &'static str {
        "0.1.0"
    }
    fn commands(&self) -> &'static [&'static str] {
        &["index"]
    }
    fn handle<'a>(
        &'a self,
        ctx: &'a ExtensionContext,
        command: &'a str,
        op: &'a str,
        p: &'a Value,
        _state: &'a mut Value,
    ) -> BoxFuture<'a, CoreResult> {
        Box::pin(async move {
            if command != "index" {
                return Err("Unknown command".into());
            }
            let id = p["sessionId"].as_str().ok_or("Missing session identity")?;
            if op == "extension:dispose-session" {
                self.sessions
                    .lock()
                    .map_err(|_| "Session state unavailable")?
                    .remove(id);
                return Ok(unchanged());
            }
            alive(ctx).await?;
            if op == "extension:run-command" {
                let s = self.open(ctx, p)?;
                return self.exchange(ctx, &s).await;
            }
            let s = self.get(id)?;
            match op {
                "extension:refresh-session" => {
                    let mut v = s.lock().map_err(|_| "Session state unavailable")?;
                    Ok(render(&mut v))
                }
                "extension:load-more" => Ok(unchanged()),
                "extension:search-text-changed" => {
                    let q = p["searchText"].as_str().ok_or("Missing search text")?;
                    if q.len() > 4096 {
                        return Err("Search is too long".into());
                    }
                    {
                        let mut v = s.lock().map_err(|_| "Session state unavailable")?;
                        v.actions.clear();
                        v.pending = None;
                        v.form = false;
                        v.query = q.into();
                        v.parsed = domain::parse(q);
                    }
                    self.exchange(ctx, &s).await
                }
                "extension:invoke-action" => {
                    let action = {
                        let mut v = s.lock().map_err(|_| "Session state unavailable")?;
                        // Closed view/action authorization: missing or stale renderer draft tokens cause no effects.
                        let fields = &p["formValues"];
                        if fields["__tezbarViewToken"] != v.view_token
                            || fields["__tezbarSearchText"] != v.query
                            || fields["__tezbarFromCurrency"] != v.from
                        {
                            return Ok(unchanged());
                        }
                        let Some(a) = v.actions.remove(p["actionId"].as_str().unwrap_or("")) else {
                            return Ok(unchanged());
                        };
                        v.actions.clear();
                        a
                    };
                    match action {
                        Action::Copy(text) | Action::RetryCopy(text) => {
                            self.copy(ctx, &s, text).await
                        }
                        Action::From => {
                            let code = p["formValues"]["value"]
                                .as_str()
                                .filter(|s| domain::currency(s).is_some())
                                .ok_or("Invalid base currency")?;
                            alive(ctx).await?;
                            if let Err(e) = storage::set_from(ctx, code) {
                                s.lock().unwrap().error = Some(e);
                                return Ok(render(&mut s.lock().unwrap()));
                            }
                            {
                                let mut v = s.lock().unwrap();
                                v.from = code.to_owned();
                                v.pending = None;
                            }
                            self.exchange(ctx, &s).await
                        }
                        Action::Pin(code) => {
                            alive(ctx).await?;
                            if let Err(e) = storage::toggle_pin(ctx, &code) {
                                s.lock().unwrap().error = Some(e);
                                return Ok(render(&mut s.lock().unwrap()));
                            }
                            s.lock().unwrap().pending = None;
                            self.exchange(ctx, &s).await
                        }
                        Action::Date => {
                            let mut v = s.lock().unwrap();
                            v.form = true;
                            v.date = v.parsed.date;
                            v.notice = None;
                            Ok(render(&mut v))
                        }
                        Action::Back => {
                            let mut v = s.lock().unwrap();
                            v.form = false;
                            v.error = None;
                            Ok(render(&mut v))
                        }
                        Action::SubmitDate => {
                            let raw = p["formValues"]["historyDate"].as_str().unwrap_or("");
                            let date = if raw.is_empty() {
                                None
                            } else {
                                match Date::from_iso(raw) {
                                    Some(d) => Some(d),
                                    None => {
                                        let mut v = s.lock().unwrap();
                                        v.error = Some("Choose a valid calendar date.".into());
                                        return Ok(render(&mut v));
                                    }
                                }
                            };
                            {
                                let mut v = s.lock().unwrap();
                                v.form = false;
                                v.error = None;
                                if date.is_none() || v.query.is_empty() {
                                    return Ok(render(&mut v));
                                }
                                v.query = domain::set_date(&v.query, date);
                                v.parsed = domain::parse(&v.query);
                                v.pending = None;
                            }
                            self.exchange(ctx, &s).await
                        }
                        Action::Retry => self.exchange(ctx, &s).await,
                    }
                }
                _ => Err("Unsupported operation".into()),
            }
        })
    }
    fn shutdown<'a>(&'a self, _ctx: &'a ExtensionContext) -> BoxFuture<'a, CoreResult> {
        Box::pin(async move {
            self.sessions
                .lock()
                .map_err(|_| "Session state unavailable")?
                .clear();
            Ok(json!({}))
        })
    }
}
fn node(kind: &str, props: Value, children: Vec<Value>) -> Value {
    json!({"type":kind,"props":props,"children":children})
}
fn add(s: &mut Session, out: &mut Vec<Value>, title: &str, kind: &str, a: Action) -> String {
    let id = token();
    s.actions.insert(id.clone(), a);
    out.push(json!({"id":id,"title":title,"kind":kind}));
    id
}
fn render(s: &mut Session) -> Value {
    s.actions.clear();
    s.view_token = token();
    let mut actions = vec![];
    let mut props = json!({"searchText":s.query,"viewToken":s.view_token,"fromCurrencyCode":s.from,"navigationTitle":"Currency Exchange"});
    let root = if s.form {
        let back = add(s, &mut actions, "Cancel", "pop", Action::Back);
        props["backActionId"] = json!(back);
        add(
            s,
            &mut actions,
            "Set to Date",
            "submit-form",
            Action::SubmitDate,
        );
        let mut children = vec![node(
            "Form.DatePicker",
            json!({"id":"historyDate","title":"Currecy History Date","value":s.date.map_or_else(String::new,Date::iso)}),
            vec![],
        )];
        if let Some(e) = &s.error {
            children.push(node("Form.Description", json!({"text":e}), vec![]));
        }
        node("Form", props, children)
    } else {
        props["hasSearchTextChangeHandler"] = json!(true);
        props["__hasServerSearch"] = json!(true);
        props["searchBarPlaceholder"] = json!(format!(
            "Input how much {}",
            domain::currency(&s.from).map_or("", |c| c.name.as_str())
        ));
        props["filtering"] = json!(false);
        props["isLoading"] = json!(false);
        let from_action = add(s, &mut actions, "From Currency", "action", Action::From);
        props["searchBarAccessory"]=node("List.Dropdown",json!({"tooltip":"From Currency","value":s.from,"actionId":from_action}),domain::currencies().iter().map(|c|node("List.Dropdown.Item",json!({"title":format!("{} - {}",c.code,c.country),"value":c.code,"icon":domain::flag(&c.code)}),vec![])).collect());
        let mut children = vec![];
        let retry_copy = if let Some(p) = s.pending.clone() {
            Some(add(
                s,
                &mut actions,
                "Retry exact clipboard write",
                "action",
                Action::RetryCopy(p.text),
            ))
        } else {
            None
        };
        if let Some(rates) = s.rates.clone() {
            let (pins, rest) = rates.entries(&s.parsed, &s.from, &s.pins);
            for (pinned, entries) in [(true, pins), (false, rest)] {
                let title = format!(
                    "{}Exchange from {} {}",
                    if pinned { "Pinned " } else { "" },
                    domain::format(s.parsed.amount, &s.from),
                    domain::currency(&s.from).map_or("", |c| c.name.as_str())
                );
                let subtitle = if let Some(d) = rates.date {
                    format!("History Currency Rate Mode: Date is {}", d.slash())
                } else {
                    format!(
                        "Currency Rate Date is {}",
                        rates.utc.as_deref().unwrap_or("unknown")
                    )
                };
                let rows=entries.into_iter().map(|entry|{
                    let mut ids=retry_copy.iter().cloned().collect::<Vec<_>>();
                    if s.pending.is_none(){ids.push(add(s,&mut actions,"Copy to Clipboard","copy",Action::Copy(domain::js_number(entry.value))));}
                    let pin= s.pins.contains(&entry.code);
                    ids.push(add(s,&mut actions,if pin{"Unpin It"}else{"Pin It"},"action",Action::Pin(entry.code.clone())));
                    let last=actions.last_mut().unwrap();last["shortcut"]=json!({"modifiers":["shift"],"key":"return"});
                    ids.push(add(s,&mut actions,"Set Currency Date","push",Action::Date));
                    let display=if entry.value==f64::INFINITY{if pinned{"No Currency"}else{"∞"}.to_owned()}else{domain::format(entry.value,&entry.code)};
                    node("List.Item",json!({"id":format!("{}:{}",if pinned{"pin"}else{"rate"},entry.code),"title":entry.code,"subtitle":display,"icon":{"source":domain::flag(&entry.code)},"accessories":[{"text":domain::currency(&entry.code).map_or("",|c|c.name.as_str()),"icon":if pinned{Some("Icon.Pin")}else{None}}],"actionIds":ids}),vec![])
                }).collect();
                children.push(node(
                    "List.Section",
                    json!({"title":title,"subtitle":if pinned{""}else{&subtitle}}),
                    rows,
                ));
            }
        }
        if s.rates.is_none() || s.error.is_some() {
            add(s, &mut actions, "Retry Exchange", "action", Action::Retry);
        }
        let empty_title = s.error.clone().unwrap_or_else(|| {
            if s.parsed.amount > 0.0 && s.rates.is_some() {
                "No matching currencies".into()
            } else {
                "Type something to get exchange result?".into()
            }
        });
        children.push(node("List.EmptyView",json!({"icon":{"source":format!("data:image/png;base64,{}",base64::engine::general_purpose::STANDARD.encode(include_bytes!("../assets/start.png")))},"title":empty_title,"description":"Example: '10', '1+2' '1 in USD' or '5.5 in USD at 2022/9/25'"}),vec![]));
        node("List", props, children)
    };
    // Dropdown callbacks are discoverable but never the default row/empty action.
    if let Some(i) = actions.iter().position(|a| a["title"] == "From Currency") {
        let a = actions.remove(i);
        actions.push(a);
    }
    let mut result = tezbar_extension_sdk::view("Currency Exchange", root, actions);
    result["refreshMs"] = Value::Null;
    if let Some(message) = s.notice.as_ref().or(s.error.as_ref()) {
        result["message"] = json!(message);
    }
    result
}
