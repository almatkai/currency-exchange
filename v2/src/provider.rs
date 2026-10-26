//! Fixed-origin HTTPS GET clients. Errors deliberately contain no URLs, keys or bodies.
use crate::domain::{Date, Query};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tezbar_extension_sdk::ExtensionContext;

pub const MAX_BODY: usize = 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Provider {
    #[serde(rename = "exchangerate-api")]
    ExchangeRate,
    #[serde(rename = "unirate")]
    UniRate,
}
impl Provider {
    pub fn label(self) -> &'static str {
        match self {
            Self::ExchangeRate => "exchangerate-api",
            Self::UniRate => "unirate",
        }
    }
}
#[derive(Clone)]
pub struct Preferences {
    pub provider: Provider,
    key: String,
}
impl Preferences {
    pub fn parse(value: &Value) -> Result<Self, String> {
        let provider = match value["provider"].as_str() {
            Some("unirate") => Provider::UniRate,
            Some("exchangerate-api") | None | Some("") => Provider::ExchangeRate,
            _ => return Err(
                "Choose a supported Rate Provider in extension settings; then reopen the command."
                    .into(),
            ),
        };
        let key = value["api_key"].as_str().unwrap_or("");
        if key.trim().is_empty()
            || key.len() > 4096
            || key.chars().any(char::is_control)
            || [".", ".."].contains(&key)
        {
            return Err("Set the API Key for the selected provider in extension settings; then reopen the command.".into());
        }
        Ok(Self {
            provider,
            key: key.into(),
        })
    }
    pub fn identity(&self, date: Option<Date>) -> String {
        use sha2::{Digest, Sha256};
        format!(
            "{}:{}:{}",
            self.provider.label(),
            hex_digest(Sha256::digest(self.key.as_bytes()).as_slice()),
            date.map_or_else(|| "current".into(), Date::iso)
        )
    }
    pub fn url(&self, date: Option<Date>) -> Result<url::Url, String> {
        let mut u = url::Url::parse(match self.provider {
            Provider::ExchangeRate => "https://v6.exchangerate-api.com/v6/",
            Provider::UniRate => "https://api.unirateapi.com/api/",
        })
        .map_err(|_| "Request setup failed")?;
        match self.provider {
            Provider::ExchangeRate => {
                let mut p = u.path_segments_mut().map_err(|_| "Request setup failed")?;
                p.pop_if_empty()
                    .push(&self.key)
                    .push(if date.is_some() { "history" } else { "latest" })
                    .push("USD");
                if let Some(d) = date {
                    p.push(&d.year.to_string())
                        .push(&d.month.to_string())
                        .push(&d.day.to_string());
                }
            }
            Provider::UniRate => {
                u.set_path(if date.is_some() {
                    "/api/historical/rates"
                } else {
                    "/api/rates"
                });
                let mut q = "from=USD".to_string();
                if let Some(d) = date {
                    q.push_str(&format!("&date={}", d.iso()));
                }
                q.push_str("&api_key=");
                q.push_str(&tezbar_extension_sdk::encode_component(&self.key));
                u.set_query(Some(&q));
            }
        }
        Ok(u)
    }
}
fn hex_digest(v: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(v.len() * 2);
    for &b in v {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 15) as usize] as char);
    }
    out
}

pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| ((d.as_millis() + 500) / 1000) as i64)
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Rates {
    pub rates: Map<String, Value>,
    pub next: Option<i64>,
    pub last: Option<i64>,
    pub utc: Option<String>,
    pub date: Option<Date>,
}
impl Rates {
    pub fn validate(&self) -> Result<(), String> {
        if self.date.is_none()
            && (self.next.is_none() || self.utc.as_deref().is_none_or(|s| !valid_utc(s, self.last)))
        {
            return Err("Invalid cached update metadata; retry to fetch a new response.".into());
        }
        if self.rates.is_empty()
            || self.rates.len() > 512
            || self
                .rates
                .iter()
                .any(|(k, v)| k.len() > 32 || v.as_f64().is_none_or(|v| !v.is_finite()))
        {
            return Err("Invalid cached rates; retry to fetch a new response.".into());
        }
        Ok(())
    }
    pub fn valid_at(&self, date: Option<Date>, now: i64) -> bool {
        self.date == date
            && (date.is_some() || self.next.is_some_and(|n| now < n))
            && self.validate().is_ok()
    }
    pub fn entries(
        &self,
        q: &Query,
        from: &str,
        pins: &[String],
    ) -> (Vec<Conversion>, Vec<Conversion>) {
        let base = self.rates.get(from).and_then(Value::as_f64).unwrap_or(0.0);
        // JS || 0 also canonicalizes -0 to +0.
        let base = if base == 0.0 { 0.0 } else { base };
        let matches = |s: &str| {
            crate::domain::currency(s).is_some_and(|c| {
                s != from
                    && (s.to_lowercase().contains(&q.filter)
                        || c.name.to_lowercase().contains(&q.filter)
                        || c.country.to_lowercase().contains(&q.filter))
            })
        };
        let make = |s: &String| {
            let r = self.rates.get(s).and_then(Value::as_f64).unwrap_or(0.0);
            let r = if r == 0.0 { 0.0 } else { r };
            Conversion {
                code: s.clone(),
                value: (q.amount / base) * r,
            }
        };
        (
            pins.iter().filter(|s| matches(s)).map(make).collect(),
            self.rates
                .keys()
                .filter(|s| !pins.contains(s) && matches(s))
                .map(make)
                .collect(),
        )
    }
}
#[derive(Clone, Debug)]
pub struct Conversion {
    pub code: String,
    pub value: f64,
}
fn valid_utc(s: &str, last: Option<i64>) -> bool {
    static UTC: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    UTC.get_or_init(||regex::Regex::new(r"^(Mon|Tue|Wed|Thu|Fri|Sat|Sun), [0-9]{2} (Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec) [0-9]{4} [0-9]{2}:[0-9]{2}:[0-9]{2} (GMT|UTC|\+0000)$").unwrap()).is_match(s) && chrono::DateTime::parse_from_rfc2822(s).is_ok_and(|d|Some(d.timestamp())==last)
}
fn safe_error(v: &Value) -> String {
    match v.as_str().unwrap_or(""){
        "invalid-key"|"inactive-account"=>"Invalid API Key, please check the selected provider in extension settings and reopen the command.".into(),
        "quota-reached"=>"Provider quota reached. Retry manually after the provider quota resets.".into(),
        "plan-upgrade-required"=>"Historical data requires a compatible provider plan.".into(),
        "no-data-available"=>"No historical rates are available for that date.".into(),
        "unsupported-code"=>"The provider does not support the requested currency.".into(),
        _=>"The selected provider rejected the request. Check provider settings and retry manually.".into()
    }
}
fn js_numeric(s: &str) -> Option<f64> {
    let s = s.trim_matches(crate::domain::js_space);
    if s.is_empty() {
        return Some(0.0);
    }
    for (p, radix) in [
        ("0x", 16),
        ("0X", 16),
        ("0b", 2),
        ("0B", 2),
        ("0o", 8),
        ("0O", 8),
    ] {
        if let Some(rest) = s.strip_prefix(p) {
            if rest.is_empty() {
                return None;
            }
            // Integer-to-f64 conversion rounds once, matching Number; do not
            // accumulate f64 digits (which can round repeatedly).
            return match u128::from_str_radix(rest, radix) {
                Ok(n) => Some(n as f64),
                Err(_) if rest.chars().all(|c| c.is_digit(radix)) => Some(f64::INFINITY),
                Err(_) => None,
            };
        }
    }
    if ["Infinity", "+Infinity", "-Infinity"].contains(&s) {
        return Some(if s.starts_with('-') {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        });
    }
    static DECIMAL: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    if !DECIMAL
        .get_or_init(|| {
            regex::Regex::new(r"^[+-]?(?:[0-9]+\.?[0-9]*|\.[0-9]+)(?:[eE][+-]?[0-9]+)?$").unwrap()
        })
        .is_match(s)
    {
        return None;
    }
    s.parse().ok()
}
pub fn normalize(
    provider: Provider,
    status: u16,
    value: Value,
    date: Option<Date>,
    now: i64,
) -> Result<Rates, String> {
    if status == 401 {
        return Err(safe_error(&Value::String("invalid-key".into())));
    }
    if status == 429 {
        return Err(safe_error(&Value::String("quota-reached".into())));
    }
    if !(200..300).contains(&status) {
        return Err("Provider HTTP request failed. Retry manually.".into());
    }
    let mut out = Rates {
        rates: Map::new(),
        next: None,
        last: None,
        utc: None,
        date,
    };
    match provider {
        Provider::ExchangeRate => {
            if value["result"] != "success" {
                return Err(safe_error(&value["error-type"]));
            }
            if value["base_code"] != "USD" {
                return Err("Unexpected provider base currency; no conversion was used.".into());
            }
            let rates = value["conversion_rates"]
                .as_object()
                .ok_or("Malformed provider rates")?;
            for (k, v) in rates {
                if v.as_f64().is_none_or(|n| !n.is_finite()) {
                    return Err("Malformed provider rate; no conversion was used.".into());
                }
                out.rates.insert(k.clone(), v.clone());
            }
            if let Some(d) = date {
                if value["year"] != d.year || value["month"] != d.month || value["day"] != d.day {
                    return Err("Unexpected historical date; no conversion was used.".into());
                }
            } else {
                out.last = value["time_last_update_unix"].as_i64();
                out.next = value["time_next_update_unix"].as_i64();
                out.utc = value["time_last_update_utc"]
                    .as_str()
                    .filter(|s| valid_utc(s, out.last))
                    .map(str::to_owned);
                if out.last.is_none() || out.next.is_none() || out.utc.is_none() {
                    return Err("Missing provider update metadata; no conversion was used.".into());
                }
            }
        }
        Provider::UniRate => {
            if value
                .get("base")
                .is_some_and(|v| v != "USD" && !v.is_null())
            {
                return Err("Unexpected provider base currency; no conversion was used.".into());
            }
            let rates = value["rates"]
                .as_object()
                .ok_or("Provider returned no rates. Check provider settings and retry manually.")?;
            for (k, v) in rates {
                let n = v.as_f64().or_else(|| v.as_str().and_then(js_numeric));
                if let Some(n) = n {
                    if n.is_nan() {
                        continue;
                    }
                    if !n.is_finite() {
                        return Err(
                            "Non-finite or oversized radix provider rate; no conversion was used."
                                .into(),
                        );
                    }
                    out.rates.insert(k.clone(), serde_json::json!(n));
                }
            }
            if date.is_none() {
                out.last = Some(now);
                out.next = Some(now + 86400);
                out.utc = Some(
                    chrono::DateTime::from_timestamp(now, 0)
                        .ok_or("Invalid clock")?
                        .format("%a, %d %b %Y %H:%M:%S GMT")
                        .to_string(),
                );
            }
        }
    }
    out.validate()?;
    Ok(out)
}
pub struct Transport {
    client: reqwest::Client,
    #[cfg(test)]
    origin: Option<url::Url>,
}
impl Transport {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            client: Self::builder()
                .build()
                .map_err(|_| "HTTPS client setup failed")?,
            #[cfg(test)]
            origin: None,
        })
    }
    fn builder() -> reqwest::ClientBuilder {
        reqwest::Client::builder()
            .https_only(true)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15))
            .pool_max_idle_per_host(0)
            .user_agent("Tezbar-Currency-Exchange/0.1.0")
    }
    #[cfg(test)]
    pub(crate) fn owned_tls(origin: &str, certificate: &[u8]) -> Self {
        let origin = url::Url::parse(origin).unwrap();
        assert_eq!(origin.host_str(), Some("localhost"));
        assert_eq!(origin.scheme(), "https");
        Self {
            client: Self::builder()
                .add_root_certificate(reqwest::Certificate::from_pem(certificate).unwrap())
                .build()
                .unwrap(),
            origin: Some(origin),
        }
    }
    #[cfg(test)]
    pub(crate) fn untrusted_owned_tls(origin: &str) -> Self {
        let mut transport = Self::new().unwrap();
        let url = url::Url::parse(origin).unwrap();
        assert_eq!(url.host_str(), Some("localhost"));
        assert_eq!(url.scheme(), "https");
        transport.origin = Some(url);
        transport
    }
    pub async fn fetch(
        &self,
        ctx: &ExtensionContext,
        p: &Preferences,
        date: Option<Date>,
    ) -> Result<Rates, String> {
        let url = p.url(date)?;
        #[cfg(test)]
        let url = if let Some(origin) = &self.origin {
            let mut u = url;
            u.set_host(origin.host_str()).unwrap();
            u.set_port(origin.port()).unwrap();
            u
        } else {
            url
        };
        let work = async {
            let request = self.client.get(url).header(
                match p.provider {
                    Provider::ExchangeRate => "Content-Type",
                    Provider::UniRate => "Accept",
                },
                "application/json",
            );
            let mut response = request
                .send()
                .await
                .map_err(|_| "HTTPS request failed; retry manually.")?;
            let status = response.status().as_u16();
            if status == 401 || status == 429 || !(200..300).contains(&status) {
                return normalize(p.provider, status, Value::Null, date, now());
            }
            if response
                .content_length()
                .is_some_and(|n| n > MAX_BODY as u64)
            {
                return Err("Provider response exceeded the size limit.".into());
            }
            let mut body = Vec::new();
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|_| "Provider response could not be read.")?
            {
                if body.len() + chunk.len() > MAX_BODY {
                    return Err("Provider response exceeded the size limit.".into());
                }
                body.extend_from_slice(&chunk);
            }
            let value = serde_json::from_slice(&body)
                .map_err(|_| "Provider returned invalid JSON; retry manually.")?;
            normalize(p.provider, status, value, date, now())
        };
        tokio::select! {biased;_=ctx.cancelled()=>Err("Request cancelled".into()),r=tokio::time::timeout(Duration::from_secs(16),work)=>r.map_err(|_|"Provider request deadline exceeded; retry manually.")?}
    }
}
