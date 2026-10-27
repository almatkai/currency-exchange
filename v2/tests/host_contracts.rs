//! Narrow extracted native boundaries, NOT a full host build or installation.
#![allow(dead_code)]
use serde_json::{json, Value};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};
#[derive(Clone)]
struct Installed {
    id: String,
    slug: String,
    path: PathBuf,
    resource_dir: PathBuf,
    manifest: Value,
    data_dir: PathBuf,
}
struct CoreContext {
    entries: Vec<Installed>,
    valid: bool,
}
fn collect(c: &CoreContext) -> Vec<Installed> {
    c.entries.clone()
}
fn installed(c: &CoreContext, id: &str) -> Result<Installed, String> {
    c.entries
        .iter()
        .find(|e| e.id == id)
        .cloned()
        .ok_or_else(|| "not installed".into())
}
fn compatible(v: &Value, _: bool) -> bool {
    v["platforms"]
        .as_array()
        .is_some_and(|a| a.contains(&json!("macOS")))
}
fn fallback<'a>(v: &'a Value, k: &str, d: &'a str) -> &'a str {
    v[k].as_str().unwrap_or(d)
}
mod process_install {
    use super::*;
    pub fn resolve(c: &CoreContext, _: &str) -> Result<(), String> {
        if c.valid {
            Ok(())
        } else {
            Err("unverified".into())
        }
    }
}
mod v2_catalog {
    use super::*;
    pub fn source() -> Vec<Value> {
        vec![json!({"slug":"perplexity"})]
    }
}
mod v2_runtime {
    pub(super) fn slug(id: &str) -> Option<&str> {
        let value = id
            .strip_prefix("com.tezbar.native.")
            .or_else(|| id.strip_prefix("raycast."))
            .unwrap_or(id);
        super::v2_catalog::source()
            .iter()
            .any(|m| m["slug"] == value)
            .then_some(value)
    }
    pub(super) fn canonical_id(slug: &str) -> String {
        format!("com.tezbar.native.{slug}")
    }
}
include!("../integration/native-extracted.rs");
fn dir() -> tempfile::TempDir {
    tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.qa/tmp")).unwrap()
}
fn entry(path: &Path) -> Installed {
    Installed{id:"com.tezbar.native.currency-exchange".into(),slug:"currency-exchange".into(),path:path.join("extensions/packages/com.tezbar.native.currency-exchange"),resource_dir:path.join("extensions/packages/com.tezbar.native.currency-exchange/versions/0.1.0/aarch64-apple-darwin"),data_dir:path.into(),manifest:json!({"icon":"resources/currency.png","replaces":["com.tezbar.currency-exchange","raycast.currency-exchange"],"commands":[{"name":"index","runtime":"rust","platforms":["macOS","Windows"]}]})}
}
#[test]
fn verified_exact_alias_command_coverage_before_after() {
    let dir = dir();
    let e = entry(dir.path());
    let mut c = CoreContext {
        entries: vec![e.clone()],
        valid: true,
    };
    for alias in ["raycast.currency-exchange", "com.tezbar.currency-exchange"] {
        assert!(before::route(&c, alias, "index").is_err());
        assert_eq!(after::route(&c, alias, "index").unwrap(), e.id);
        assert!(after::route(&c, alias, "other").is_err());
    }
    for id in [
        "currency-exchange",
        "raycast.Currency-Exchange",
        " raycast.currency-exchange",
        "raycast.currency-exchange ",
        "com.tezbar.currency-exchange.evil",
        "raycast.currency-exchangе",
    ] {
        assert!(after::route(&c, id, "index").is_err());
    }
    c.valid = false;
    assert!(after::route(&c, "raycast.currency-exchange", "index").is_err());
    c.valid = true;
    let mut duplicate = e.clone();
    duplicate.id = "com.tezbar.native.other".into();
    c.entries.push(duplicate);
    assert!(after::route(&c, "raycast.currency-exchange", "index").is_err());
}
#[test]
fn active_resource_root_independent_of_canonical_preferences() {
    let dir = dir();
    let e = entry(dir.path());
    fs::create_dir_all(e.resource_dir.join("resources")).unwrap();
    fs::write(
        e.resource_dir.join("resources/currency.png"),
        include_bytes!("../assets/currency.png"),
    )
    .unwrap();
    fs::write(
        e.path.join("preferences.json"),
        b"{\"api_key\":\"owned-fixture-key\",\"unknown\":true}",
    )
    .unwrap();
    assert!(before::test_icon(&e).is_none());
    assert_eq!(
        after::test_icon(&e).unwrap(),
        e.resource_dir
            .join("resources/currency.png")
            .to_str()
            .unwrap()
    );
    assert_eq!(
        before::test_preferences(&e).unwrap(),
        after::test_preferences(&e).unwrap()
    );
    assert_eq!(
        after::test_preferences(&e).unwrap(),
        e.path.join("preferences.json")
    );
    assert!(!e.resource_dir.join("preferences.json").exists());
}
#[test]
fn selected_clipboard_producer_ack_matches_strict_contract() {
    let source = include_str!(
        "../integration/fixtures/host/src-tauri/src/native_core/launcher/clipboard.rs"
    );
    let branch = source
        .split("\"clipboard:write\" => {")
        .nth(1)
        .unwrap()
        .split("\"clipboard:write-png\"")
        .next()
        .unwrap();
    assert!(branch.contains("self.write_text(text_arg(a, 0)?.into())?;"));
    assert!(branch.contains("Ok(json!({\"ok\":true}))"));
    let host = include_str!(
        "../integration/fixtures/host/src-tauri/src/native_core/extensions/process_host.rs"
    );
    assert!(host.contains("else{self.ctx.request(\"clipboard:write\",vec![json!(text)]).await}"));
}
