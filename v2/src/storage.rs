//! Fresh atomic updates under a process lane and file lock. No key or URL is stored.
use crate::{domain::currency, provider::Rates};
use serde_json::{json, Value};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};
use tezbar_extension_sdk::ExtensionContext;
const MAX_BYTES: usize = 1024 * 1024;
const MAX_CACHE: usize = 32;
static LANE: Mutex<()> = Mutex::new(());
fn err() -> String {
    "Private storage could not be read or saved. Keep the draft, repair the extension-private store, and retry.".into()
}
fn directory(ctx: &ExtensionContext) -> Result<PathBuf, String> {
    let p = ctx.paths.data_dir.join("currency-exchange");
    // Host owns dataDir; never follow a symlink below that trusted root.
    if fs::symlink_metadata(&p).is_ok_and(|m| m.file_type().is_symlink() || !m.is_dir()) {
        return Err(err());
    }
    fs::create_dir_all(&p).map_err(|_| err())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).map_err(|_| err())?;
    }
    Ok(p)
}
fn no_link(p: &Path) -> Result<(), String> {
    match fs::symlink_metadata(p) {
        Ok(m) if m.file_type().is_symlink() || !m.is_file() => Err(err()),
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(err()),
        _ => Ok(()),
    }
}
fn read(p: &Path) -> Result<Value, String> {
    no_link(p)?;
    let f = match fs::File::open(p) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(json!({})),
        Err(_) => return Err(err()),
    };
    let mut bytes = Vec::new();
    f.take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| err())?;
    if bytes.len() > MAX_BYTES {
        return Err(err());
    }
    let v: Value = serde_json::from_slice(&bytes).map_err(|_| err())?;
    if !v.is_object() {
        return Err(err());
    }
    Ok(v)
}
pub fn access<T>(
    ctx: &ExtensionContext,
    update: bool,
    f: impl FnOnce(&mut Value) -> Result<T, String>,
) -> Result<T, String> {
    let _lane = LANE.lock().map_err(|_| err())?;
    let dir = directory(ctx)?;
    let lock = dir.join("storage.lock");
    no_link(&lock)?;
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let lock = options.open(lock).map_err(|_| err())?;
    // Bounded wait: no blocking background retry loops or indefinite file-lock waits.
    fs2::FileExt::try_lock_exclusive(&lock)
        .map_err(|_| "Private storage is busy; retry manually.".to_string())?;
    let path = dir.join("storage.json");
    let mut value = read(&path)?;
    let result = f(&mut value)?;
    if update {
        let bytes = serde_json::to_vec(&value).map_err(|_| err())?;
        if bytes.len() > MAX_BYTES {
            return Err("Private storage size limit reached; no data was overwritten.".into());
        }
        let mut tmp = tempfile::NamedTempFile::new_in(&dir).map_err(|_| err())?;
        tmp.write_all(&bytes)
            .and_then(|_| tmp.as_file().sync_all())
            .map_err(|_| err())?;
        tmp.persist(path).map_err(|_| err())?;
        #[cfg(unix)]
        fs::File::open(dir)
            .and_then(|f| f.sync_all())
            .map_err(|_| err())?;
    }
    Ok(result)
}
pub fn settings(ctx: &ExtensionContext) -> Result<(String, Vec<String>), String> {
    access(ctx, false, |v| {
        let from = v["fromCurrencyCode"]
            .as_str()
            .filter(|s| currency(s).is_some())
            .unwrap_or("USD")
            .to_owned();
        let pins = pins(v);
        Ok((from, pins))
    })
}
fn pins(v: &Value) -> Vec<String> {
    v["pinnedCurrencyCode"]
        .as_array()
        .map_or_else(Vec::new, |a| {
            a.iter()
                .take(512)
                .filter_map(Value::as_str)
                .filter(|s| currency(s).is_some())
                .map(str::to_owned)
                .collect()
        })
}
pub fn set_from(ctx: &ExtensionContext, code: &str) -> Result<(), String> {
    access(ctx, true, |v| {
        v["fromCurrencyCode"] = json!(code);
        Ok(())
    })
}
pub fn toggle_pin(ctx: &ExtensionContext, code: &str) -> Result<(), String> {
    access(ctx, true, |v| {
        let mut p = pins(v);
        if p.iter().any(|x| x == code) {
            p.retain(|x| x != code);
        } else if p.len() < 512 {
            p.push(code.into());
        } else {
            return Err("Pinned currency limit reached".into());
        }
        v["pinnedCurrencyCode"] = json!(p);
        Ok(())
    })
}
pub fn cached(ctx: &ExtensionContext, key: &str) -> Result<Option<Rates>, String> {
    access(ctx, false, |v| {
        Ok(v["cache"][key]
            .get("data")
            .and_then(|d| serde_json::from_value::<Rates>(d.clone()).ok())
            .filter(|r| r.validate().is_ok()))
    })
}
pub fn cache(ctx: &ExtensionContext, key: &str, rates: &Rates) -> Result<(), String> {
    access(ctx, true, |v| {
        if !v["cache"].is_object() {
            v["cache"] = json!({});
        }
        let c = v["cache"].as_object_mut().unwrap();
        c.shift_remove(key);
        while c.len() >= MAX_CACHE {
            let Some(old) = c.keys().next().cloned() else {
                break;
            };
            c.shift_remove(&old);
        }
        c.insert(key.into(), json!({"data":rates}));
        Ok(())
    })
}
