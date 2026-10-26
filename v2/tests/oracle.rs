use serde_json::{json, Value};
use tezbar_currency_exchange::{
    domain,
    provider::{self, Provider, Rates},
};
fn fixture() -> Value {
    serde_json::from_str(include_str!("source-oracle.json")).unwrap()
}
fn number(s: &str, negative_zero: bool) -> f64 {
    if negative_zero {
        -0.0
    } else {
        match s {
            "NaN" => f64::NAN,
            "Infinity" => f64::INFINITY,
            "-Infinity" => f64::NEG_INFINITY,
            _ => s.parse().unwrap(),
        }
    }
}
#[test]
fn whole_source_parse_vectors() {
    for c in fixture()["parsed"].as_array().unwrap() {
        let q = domain::parse(c["text"].as_str().unwrap());
        assert_eq!(
            domain::js_number(q.amount),
            c["amount"]["raw"],
            "{}",
            c["text"]
        );
        assert_eq!(
            q.amount == 0.0 && q.amount.is_sign_negative(),
            c["amount"]["negativeZero"]
        );
        assert_eq!(q.filter, c["filter"]);
        assert_eq!(
            q.date
                .map(|d| json!([d.year, d.month, d.day]))
                .unwrap_or(Value::Null),
            c["date"],
            "{}",
            c["text"]
        );
    }
}
#[test]
fn all_source_currency_formats() {
    for c in fixture()["formats"].as_array().unwrap() {
        let n = number(
            c["input"].as_str().unwrap(),
            c["negativeZero"].as_bool().unwrap(),
        );
        assert_eq!(
            domain::format(n, c["code"].as_str().unwrap()),
            c["expected"],
            "{c}"
        );
    }
}
fn compare_entries(rows: (Vec<provider::Conversion>, Vec<provider::Conversion>), v: &Value) {
    let mapped = vec![rows.0, rows.1]
        .into_iter()
        .map(|rows| {
            rows.iter()
                .map(|r| json!({"code":r.code,"value":domain::js_number(r.value)}))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    assert_eq!(json!(mapped), *v);
}
#[test]
fn own_source_order_filter_pins_and_f64() {
    let f = fixture();
    for c in f["enrich"].as_array().unwrap() {
        let r = Rates {
            rates: c["rates"].as_object().unwrap().clone(),
            next: None,
            last: None,
            utc: None,
            date: None,
        };
        let pins = serde_json::from_value::<Vec<String>>(c["pinned"].clone()).unwrap();
        compare_entries(
            r.entries(
                &domain::Query {
                    amount: 123.456,
                    filter: c["filter"].as_str().unwrap().into(),
                    date: None,
                },
                c["base"].as_str().unwrap(),
                &pins,
            ),
            &c["expected"],
        );
    }
    for c in f["special"].as_array().unwrap() {
        let r = Rates {
            rates: c["rates"].as_object().unwrap().clone(),
            next: None,
            last: None,
            utc: None,
            date: None,
        };
        compare_entries(
            r.entries(
                &domain::Query {
                    amount: 10.0,
                    filter: String::new(),
                    date: None,
                },
                "USD",
                &["GBP".into(), "EUR".into()],
            ),
            &c["expected"],
        );
    }
}
#[test]
fn normalization_matches_whole_source() {
    for c in fixture()["sourceProvider"].as_array().unwrap() {
        let provider = if c["provider"] == "unirate" {
            Provider::UniRate
        } else {
            Provider::ExchangeRate
        };
        let date = c["history"].as_bool().unwrap().then_some(domain::Date {
            year: 2023,
            month: 2,
            day: 28,
        });
        let raw = if provider == Provider::UniRate {
            json!({"base":"USD","rates":{"USD":"1","JPY":"150.5","EUR":".9","ZZZ":"notnumber","GBP":"0x10"}})
        } else {
            c["body"].clone()
        };
        let r = provider::normalize(provider, 200, raw, date, 1735689600).unwrap();
        assert_eq!(
            r.rates.keys().collect::<Vec<_>>(),
            c["body"]["conversion_rates"]
                .as_object()
                .unwrap()
                .keys()
                .collect::<Vec<_>>()
        );
        for (k, v) in &r.rates {
            assert_eq!(v.as_f64(), c["body"]["conversion_rates"][k].as_f64());
        }
        if date.is_some() {
            assert!(r.next.is_none())
        } else {
            assert_eq!(r.next, c["body"]["time_next_update_unix"].as_i64())
        }
    }
}
#[test]
fn parser_bounds_and_date_navigation() {
    assert!(domain::arithmetic(&"(".repeat(2000)).is_err());
    assert!(domain::arithmetic(&format!("{}1", "-".repeat(200))).is_err());
    assert_eq!(
        domain::set_date(
            "10 in EUR at 2022/1/1 suffix",
            Some(domain::Date {
                year: 2023,
                month: 2,
                day: 28
            })
        ),
        "10 in EUR suffix at 2023/2/28"
    );
    assert_eq!(
        domain::set_date(
            "",
            Some(domain::Date {
                year: 2023,
                month: 2,
                day: 28
            })
        ),
        ""
    );
    assert_eq!(domain::flag("XDR"), "🇺🇳");
    assert_eq!(domain::flag("ANG"), "🇺🇳");
    assert_eq!(domain::flag("USD"), "🇺🇸");
}
#[test]
fn provider_errors_and_explicit_numeric_exclusions() {
    for p in [Provider::ExchangeRate, Provider::UniRate] {
        for status in [301, 302, 400, 401, 403, 429, 500] {
            let e = provider::normalize(
                p,
                status,
                json!({"message":"owned-fixture-secret"}),
                None,
                1,
            )
            .unwrap_err();
            assert!(!e.contains("secret"));
        }
    }
    for key in [
        "invalid-key",
        "inactive-account",
        "quota-reached",
        "plan-upgrade-required",
        "no-data-available",
        "unsupported-code",
        "owned-fixture-secret",
    ] {
        assert!(!provider::normalize(
            Provider::ExchangeRate,
            200,
            json!({"result":"error","error-type":key}),
            None,
            1
        )
        .unwrap_err()
        .contains("secret"));
    }
    for v in [
        json!({"base":"EUR","rates":{"USD":1}}),
        json!({"rates":{"USD":"Infinity"}}),
        json!({"rates":{"USD":"1e9999"}}),
        json!({"error":"owned-fixture-secret"}),
    ] {
        assert!(provider::normalize(Provider::UniRate, 200, v, None, 1).is_err());
    }
    let n = provider::normalize(
        Provider::UniRate,
        200,
        json!({"rates":{"USD":"1","EUR":"", "JPY":"NaN","GBP":"0b11"}}),
        None,
        42,
    )
    .unwrap();
    assert_eq!(n.rates["EUR"], 0.0);
    assert!(n.rates.get("JPY").is_none());
    assert_eq!(n.rates["GBP"], 3.0);
    assert_eq!(n.last, Some(42));
    assert_eq!(n.next, Some(86442));
    assert!(n.valid_at(None, 86441));
    assert!(!n.valid_at(None, 86442));
}
#[test]
fn auth_request_encoding_and_scope() {
    let p = provider::Preferences::parse(
        &json!({"api_key":"owned a/b?&#+%雪","provider":"exchangerate-api"}),
    )
    .unwrap();
    let url = p
        .url(Some(domain::Date {
            year: 2023,
            month: 2,
            day: 28,
        }))
        .unwrap();
    assert_eq!(url.as_str(),"https://v6.exchangerate-api.com/v6/owned%20a%2Fb%3F&%23+%25%E9%9B%AA/history/USD/2023/2/28");
    let p2 =
        provider::Preferences::parse(&json!({"api_key":"owned a/b?&#+%雪","provider":"unirate"}))
            .unwrap();
    let url = p2.url(None).unwrap();
    assert_eq!(
        url.query_pairs().find(|(k, _)| k == "api_key").unwrap().1,
        "owned a/b?&#+%雪"
    );
    assert_ne!(p.identity(None), p2.identity(None));
    assert_ne!(
        p.identity(None),
        p.identity(Some(domain::Date {
            year: 2023,
            month: 2,
            day: 28
        }))
    );
    assert!(!p.identity(None).contains("owned"));
}
