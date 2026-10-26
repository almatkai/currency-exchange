//! Source-compatible arithmetic, civil dates, table order and en-US formatting.
use chrono::{Datelike, Duration, NaiveDate};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Currency {
    pub code: String,
    pub name: String,
    pub country: String,
    pub symbol: String,
}
pub fn currencies() -> &'static [Currency] {
    static TABLE: OnceLock<Vec<Currency>> = OnceLock::new();
    TABLE.get_or_init(|| {
        serde_json::from_str(include_str!("currencies.json")).expect("pinned currency table")
    })
}
pub fn currency(code: &str) -> Option<&'static Currency> {
    currencies().iter().find(|x| x.code == code)
}
pub fn flag(code: &str) -> String {
    if code.starts_with('X') || code.starts_with("AN") {
        return "🇺🇳".into();
    }
    code.chars()
        .take(2)
        .filter_map(|c| char::from_u32(127397 + u32::from(c)))
        .collect()
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Date {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}
impl Date {
    pub fn iso(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
    pub fn slash(self) -> String {
        format!("{}/{}/{}", self.year, self.month, self.day)
    }
    pub fn from_iso(s: &str) -> Option<Self> {
        let date = NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()?;
        let value = Self {
            year: date.year(),
            month: date.month(),
            day: date.day(),
        };
        (value.iso() == s && (1..=9999).contains(&value.year)).then_some(value)
    }
}
const JS_WS: &str = r"[\x09-\x0D\x20\u{00A0}\u{1680}\u{2000}-\u{200A}\u{2028}\u{2029}\u{202F}\u{205F}\u{3000}\u{FEFF}]";
pub fn js_space(c: char) -> bool {
    matches!(c,'\u{0009}'..='\u{000D}'|'\u{0020}'|'\u{00A0}'|'\u{1680}'|'\u{2000}'..='\u{200A}'|'\u{2028}'|'\u{2029}'|'\u{202F}'|'\u{205F}'|'\u{3000}'|'\u{FEFF}')
}
fn history_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(&format!(
            r"{JS_WS}+at{JS_WS}+([0-9]{{4}}/[0-9]{{1,2}}/[0-9]{{1,2}})"
        ))
        .unwrap()
    })
}
fn js_slash_date(s: &str) -> Option<Date> {
    let v: Vec<i32> = s
        .split('/')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    // V8 treats a leading 0001..0012 as a MONTH, even though the source
    // history regex requires four digits there. 0013..0031 are invalid;
    // zero and 0032..0099 instead select the legacy two-digit year.
    let short_year = |year| if year < 50 { 2000 + year } else { 1900 + year };
    let (year, month, day) = match v[0] {
        1..=12 => (short_year(v[2]), v[0], v[1]),
        13..=31 => return None,
        0 | 32..=99 => (short_year(v[0]), v[1], v[2]),
        year => (year, v[1], v[2]),
    };
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let d = NaiveDate::from_ymd_opt(year, month as u32, 1)?
        .checked_add_signed(Duration::days(i64::from(day - 1)))?;
    Some(Date {
        year: d.year(),
        month: d.month(),
        day: d.day(),
    })
}
#[derive(Clone, Debug)]
pub struct Query {
    pub amount: f64,
    pub filter: String,
    pub date: Option<Date>,
}
pub fn parse(query: &str) -> Query {
    let mut out = Query {
        amount: 0.0,
        filter: String::new(),
        date: None,
    };
    let prefix = query
        .bytes()
        .take_while(|c| c.is_ascii_digit() || b"().+-*/^".contains(c))
        .count();
    if prefix == 0 {
        return out;
    }
    out.amount = arithmetic(&query[..prefix]).unwrap_or(0.0);
    // JavaScript's (.*) is not dotAll (also excludes CR and Unicode line separators).
    let rest = query[prefix..]
        .split(['\n', '\r', '\u{2028}', '\u{2029}'])
        .next()
        .unwrap_or("");
    out.date = history_regex()
        .captures(rest)
        .and_then(|c| js_slash_date(&c[1]));
    static FILTER: OnceLock<Regex> = OnceLock::new();
    if let Some(c) = FILTER
        .get_or_init(|| Regex::new(&format!(r"{JS_WS}+in{JS_WS}+([a-zA-Z{JS_WS}]*)")).unwrap())
        .captures(rest)
    {
        static AT: OnceLock<Regex> = OnceLock::new();
        out.filter = AT
            .get_or_init(|| Regex::new(&format!(r"{JS_WS}+at{JS_WS}+")).unwrap())
            .replace(&c[1], " ")
            .trim_matches(js_space)
            .to_lowercase();
    }
    out
}
pub fn set_date(query: &str, date: Option<Date>) -> String {
    date.filter(|_| !query.is_empty()).map_or_else(
        || query.to_owned(),
        |d| format!("{} at {}", history_regex().replace(query, ""), d.slash()),
    )
}
pub fn arithmetic(s: &str) -> Result<f64, &'static str> {
    if s.len() > 1024 {
        return Err("Expression too long");
    }
    struct Parser<'a> {
        s: &'a [u8],
        i: usize,
        depth: usize,
    }
    impl Parser<'_> {
        fn peek(&self) -> u8 {
            *self.s.get(self.i).unwrap_or(&0)
        }
        fn unary(&mut self) -> Result<f64, &'static str> {
            self.depth += 1;
            if self.depth > 128 {
                return Err("Expression too deep");
            }
            let v = match self.peek() {
                b'+' => {
                    self.i += 1;
                    self.unary()?
                }
                b'-' => {
                    self.i += 1;
                    -self.unary()?
                }
                b'(' => {
                    self.i += 1;
                    let v = self.expr()?;
                    if self.peek() != b')' {
                        return Err("Missing parenthesis");
                    }
                    self.i += 1;
                    v
                }
                _ => {
                    let start = self.i;
                    while self.peek().is_ascii_digit() {
                        self.i += 1;
                    }
                    if self.i == start {
                        return Err("Number expected");
                    }
                    if self.peek() == b'.' {
                        self.i += 1;
                        let p = self.i;
                        while self.peek().is_ascii_digit() {
                            self.i += 1;
                        }
                        if p == self.i {
                            return Err("Invalid number");
                        }
                    }
                    let v = std::str::from_utf8(&self.s[start..self.i])
                        .unwrap()
                        .parse::<f64>()
                        .map_err(|_| "Invalid number")?;
                    if !v.is_finite() {
                        return Err("Invalid number");
                    }
                    v
                }
            };
            self.depth -= 1;
            Ok(v)
        }
        fn power(&mut self) -> Result<f64, &'static str> {
            let b = self.unary()?;
            if self.peek() == b'^' {
                self.i += 1;
                self.depth += 1;
                if self.depth > 128 {
                    return Err("Expression too deep");
                }
                let exponent = self.power()?;
                self.depth -= 1;
                // ECMAScript explicitly returns NaN for abs(base)==1 and infinite exponent;
                // C/Rust pow uses 1 here. Intermediate non-finite operands are otherwise allowed.
                Ok(
                    if exponent.is_nan() || (b.abs() == 1.0 && exponent.is_infinite()) {
                        f64::NAN
                    } else {
                        b.powf(exponent)
                    },
                )
            } else {
                Ok(b)
            }
        }
        fn term(&mut self) -> Result<f64, &'static str> {
            let mut v = self.power()?;
            while b"*/".contains(&self.peek()) {
                let op = self.peek();
                self.i += 1;
                let n = self.power()?;
                v = if op == b'*' { v * n } else { v / n };
            }
            Ok(v)
        }
        fn expr(&mut self) -> Result<f64, &'static str> {
            let mut v = self.term()?;
            while b"+-".contains(&self.peek()) {
                let op = self.peek();
                self.i += 1;
                let n = self.term()?;
                v = if op == b'+' { v + n } else { v - n };
            }
            Ok(v)
        }
    }
    let mut p = Parser {
        s: s.as_bytes(),
        i: 0,
        depth: 0,
    };
    let n = p.expr()?;
    if p.i == s.len() && n.is_finite() {
        Ok(n)
    } else {
        Err("Invalid expression")
    }
}
pub fn js_number(n: f64) -> String {
    ryu_js::Buffer::new().format(n).to_owned()
}
// Intl en-US, currency, minFraction=0, maxFraction=4, halfExpand. Currency symbols
// are pinned from the source oracle's ICU table; decimal conversion uses JS's shortest digits.
pub fn format(n: f64, code: &str) -> String {
    let Some(c) = currency(code) else {
        return js_number(n);
    };
    let mut digits = if n.is_nan() {
        "NaN".into()
    } else if n.is_infinite() {
        "∞".into()
    } else {
        decimal4(n.abs())
    };
    if digits.as_bytes().first().is_some_and(u8::is_ascii_digit) {
        let point = digits.find('.').unwrap_or(digits.len());
        let mut grouped = String::new();
        for (i, ch) in digits[..point].chars().enumerate() {
            if i > 0 && (point - i) % 3 == 0 {
                grouped.push(',');
            }
            grouped.push(ch);
        }
        grouped.push_str(&digits[point..]);
        digits = grouped;
    }
    let sign = if n.is_sign_negative() && !n.is_nan() {
        "-"
    } else {
        ""
    };
    let space = if !n.is_nan()
        && !n.is_infinite()
        && c.symbol.chars().last().is_some_and(char::is_alphabetic)
    {
        "\u{a0}"
    } else {
        ""
    };
    format!("{sign}{}{space}{digits}", c.symbol)
}
fn decimal4(n: f64) -> String {
    let s = js_number(n);
    let (mantissa, exponent) = s
        .split_once('e')
        .map_or((s.as_str(), 0), |(m, e)| (m, e.parse::<i32>().unwrap()));
    let point = mantissa.find('.').unwrap_or(mantissa.len()) as i32 + exponent;
    let raw = mantissa.replace('.', "");
    let mut whole = String::new();
    let mut frac = String::new();
    if point <= 0 {
        whole.push('0');
        frac.push_str(&"0".repeat((-point) as usize));
        frac.push_str(&raw);
    } else if point as usize >= raw.len() {
        whole.push_str(&raw);
        whole.push_str(&"0".repeat(point as usize - raw.len()));
    } else {
        whole.push_str(&raw[..point as usize]);
        frac.push_str(&raw[point as usize..]);
    }
    let round = frac.as_bytes().get(4).is_some_and(|c| *c >= b'5');
    frac.truncate(4);
    if round {
        while frac.len() < 4 {
            frac.push('0');
        }
        let mut bytes = format!("{whole}{frac}").into_bytes();
        let mut i = bytes.len();
        loop {
            if i == 0 {
                bytes.insert(0, b'1');
                break;
            }
            i -= 1;
            if bytes[i] == b'9' {
                bytes[i] = b'0';
            } else {
                bytes[i] += 1;
                break;
            }
        }
        let digits = String::from_utf8(bytes).unwrap();
        let p = digits.len() - 4;
        whole = digits[..p].into();
        frac = digits[p..].into();
    }
    while frac.ends_with('0') {
        frac.pop();
    }
    if frac.is_empty() {
        whole
    } else {
        format!("{whole}.{frac}")
    }
}
