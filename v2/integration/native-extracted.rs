// Exact complete functions / canonical-selection block from host fixtures.
#[allow(clippy::all)]
mod before {
use super::*;
fn slug(id: &str) -> Result<String, String> {
    let name = id
        .trim()
        .strip_prefix("com.tezbar.native.")
        .or_else(|| id.trim().strip_prefix("raycast."))
        .unwrap_or(id.trim());
    if name.is_empty()
        || name.len() > 200
        || !name.as_bytes()[0].is_ascii_alphanumeric()
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    {
        return Err("Invalid extension id".into());
    }
    Ok(name.into())
}
fn icon_path(extension: &Installed) -> Option<String> {
    let name = fallback(&extension.manifest, "icon", "icon.png");
    if name.starts_with("https://") || name.starts_with("http://") {
        return Some(name.into());
    }
    let relative = Path::new(name.strip_prefix("./").unwrap_or(name));
    if relative
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return None;
    }
    let mut candidates = vec![
        extension.path.join(relative),
        extension.path.join("assets").join(relative),
    ];
    if extension.id.starts_with("com.tezbar.native.") {
        if let Some(packages) = extension.path.parent() {
            if let Some(extensions) = packages.parent() {
                let legacy = extensions.join(&extension.slug);
                candidates.push(legacy.join(relative));
                candidates.push(legacy.join("assets").join(relative));
            }
            let legacy = packages.join(format!("raycast.{}", extension.slug));
            candidates.push(legacy.join(relative));
            candidates.push(legacy.join("assets").join(relative));
        }
    }
    candidates
        .into_iter()
        .find(|p| p.is_file())
        .map(|p| p.to_string_lossy().into())
}
fn preference_metadata(base: &Path, path: &Path) -> Result<Option<std::fs::Metadata>, String> {
    let relative = path.strip_prefix(base).map_err(|e| e.to_string())?;
    let mut current = base.to_path_buf();
    let mut parts = relative.components().peekable();
    let mut result = None;
    while let Some(part) = parts.next() {
        if !matches!(part, Component::Normal(_)) {
            return Err("Unsafe preference path".into());
        }
        current.push(part);
        let metadata = match std::fs::symlink_metadata(&current) {
            Ok(metadata) => metadata,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        if metadata.file_type().is_symlink() || (parts.peek().is_some() && !metadata.is_dir()) {
            return Err("Preference path symlink/non-directory rejected".into());
        }
        result = Some(metadata);
    }
    Ok(result)
}
fn preference_file(base: &Path, path: &Path) -> Result<bool, String> {
    match preference_metadata(base, path)? {
        Some(metadata) if !metadata.is_file() => Err("Preference file is not regular".into()),
        Some(_) => Ok(true),
        None => Ok(false),
    }
}
fn retained_preferences_path(extension: &Installed) -> PathBuf {
    extension
        .data_dir
        .join("extension-preferences")
        .join(format!("{}.json", extension.id))
}
fn preferences_path(extension: &Installed) -> Result<PathBuf, String> {
    let own = extension.path.join("preferences.json");
    if !extension.id.starts_with("com.tezbar.native.") {
        return Ok(own);
    }
    // Explicit external search roots keep their existing boundary. Managed
    // packages additionally validate every descendant of the profile root.
    let base = if extension.path.starts_with(&extension.data_dir) {
        &extension.data_dir
    } else {
        extension.path.parent().unwrap_or(&extension.path)
    };
    if preference_file(base, &own)? {
        return Ok(own);
    }
    let retained = retained_preferences_path(extension);
    if preference_file(&extension.data_dir, &retained)? {
        return Ok(retained);
    }
    let packages = extension.path.parent().unwrap_or(&extension.path);
    let extensions = packages.parent().unwrap_or(packages);
    for legacy in [
        extensions.join(&extension.slug).join("preferences.json"),
        packages
            .join(format!("raycast.{}", extension.slug))
            .join("preferences.json"),
    ] {
        let base = if legacy.starts_with(&extension.data_dir) {
            &extension.data_dir
        } else {
            extensions
        };
        if preference_file(base, &legacy)? {
            return Ok(legacy);
        }
    }
    Ok(own)
}
pub(super) fn installed_command(ctx: &CoreContext, id: &str, command: &str) -> Option<Installed> {
    let native = if id.starts_with("raycast.") {
        let slug = slug(id).ok()?;
        let canonical = super::v2_runtime::canonical_id(&slug);
        installed(ctx, &canonical).ok()
    } else if id.starts_with("com.tezbar.native.") {
        installed(ctx, id).ok()
    } else {
        None
    };
    if let Some(extension) = native {
        let active = extension.manifest["healthError"].is_null()
            && extension.manifest["commands"]
                .as_array()?
                .iter()
                .any(|c| c["name"] == command && c["runtime"] == "rust" && compatible(c, true));
        if active {
            return Some(extension);
        }
        if id.starts_with("com.tezbar.native.") {
            return None;
        }
    }
    if id.starts_with("com.tezbar.native.") {
        return None;
    }
    let extension = installed(ctx, id).ok()?;
    (compatible(&extension.manifest, false)
        && extension.manifest["commands"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|cmd| cmd["name"].as_str() == Some(command) && compatible(cmd, true)))
    .then_some(extension)
}

pub(super) fn route(ctx: &CoreContext,requested: &str, command: &str)->Result<String,String>{
let _ = (ctx,command);
let canonical = if requested.starts_with("com.tezbar.native.") {
            requested.to_string()
        } else {
            v2_runtime::canonical_id(v2_runtime::slug(requested).ok_or("Unknown extension")?)
        };
Ok(canonical)
}

pub(super) fn test_icon(e:&Installed)->Option<String>{icon_path(e)}
pub(super) fn test_preferences(e:&Installed)->Result<PathBuf,String>{preferences_path(e)}

}
#[allow(clippy::all)]
mod after {
use super::*;
use super::after as registry;
fn slug(id: &str) -> Result<String, String> {
    let name = id
        .trim()
        .strip_prefix("com.tezbar.native.")
        .or_else(|| id.trim().strip_prefix("raycast."))
        .unwrap_or(id.trim());
    if name.is_empty()
        || name.len() > 200
        || !name.as_bytes()[0].is_ascii_alphanumeric()
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    {
        return Err("Invalid extension id".into());
    }
    Ok(name.into())
}
fn icon_path(extension: &Installed) -> Option<String> {
    let name = fallback(&extension.manifest, "icon", "icon.png");
    if name.starts_with("https://") || name.starts_with("http://") {
        return Some(name.into());
    }
    let relative = Path::new(name.strip_prefix("./").unwrap_or(name));
    if relative
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return None;
    }
    let mut candidates = vec![
        extension.resource_dir.join(relative),
        extension.resource_dir.join("assets").join(relative),
    ];
    if extension.id.starts_with("com.tezbar.native.") {
        if let Some(packages) = extension.path.parent() {
            if let Some(extensions) = packages.parent() {
                let legacy = extensions.join(&extension.slug);
                candidates.push(legacy.join(relative));
                candidates.push(legacy.join("assets").join(relative));
            }
            let legacy = packages.join(format!("raycast.{}", extension.slug));
            candidates.push(legacy.join(relative));
            candidates.push(legacy.join("assets").join(relative));
        }
    }
    candidates
        .into_iter()
        .find(|p| p.is_file())
        .map(|p| p.to_string_lossy().into())
}
fn preference_metadata(base: &Path, path: &Path) -> Result<Option<std::fs::Metadata>, String> {
    let relative = path.strip_prefix(base).map_err(|e| e.to_string())?;
    let mut current = base.to_path_buf();
    let mut parts = relative.components().peekable();
    let mut result = None;
    while let Some(part) = parts.next() {
        if !matches!(part, Component::Normal(_)) {
            return Err("Unsafe preference path".into());
        }
        current.push(part);
        let metadata = match std::fs::symlink_metadata(&current) {
            Ok(metadata) => metadata,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        if metadata.file_type().is_symlink() || (parts.peek().is_some() && !metadata.is_dir()) {
            return Err("Preference path symlink/non-directory rejected".into());
        }
        result = Some(metadata);
    }
    Ok(result)
}
fn preference_file(base: &Path, path: &Path) -> Result<bool, String> {
    match preference_metadata(base, path)? {
        Some(metadata) if !metadata.is_file() => Err("Preference file is not regular".into()),
        Some(_) => Ok(true),
        None => Ok(false),
    }
}
fn retained_preferences_path(extension: &Installed) -> PathBuf {
    extension
        .data_dir
        .join("extension-preferences")
        .join(format!("{}.json", extension.id))
}
fn preferences_path(extension: &Installed) -> Result<PathBuf, String> {
    let own = extension.path.join("preferences.json");
    if !extension.id.starts_with("com.tezbar.native.") {
        return Ok(own);
    }
    // Explicit external search roots keep their existing boundary. Managed
    // packages additionally validate every descendant of the profile root.
    let base = if extension.path.starts_with(&extension.data_dir) {
        &extension.data_dir
    } else {
        extension.path.parent().unwrap_or(&extension.path)
    };
    if preference_file(base, &own)? {
        return Ok(own);
    }
    let retained = retained_preferences_path(extension);
    if preference_file(&extension.data_dir, &retained)? {
        return Ok(retained);
    }
    let packages = extension.path.parent().unwrap_or(&extension.path);
    let extensions = packages.parent().unwrap_or(packages);
    for legacy in [
        extensions.join(&extension.slug).join("preferences.json"),
        packages
            .join(format!("raycast.{}", extension.slug))
            .join("preferences.json"),
    ] {
        let base = if legacy.starts_with(&extension.data_dir) {
            &extension.data_dir
        } else {
            extensions
        };
        if preference_file(base, &legacy)? {
            return Ok(legacy);
        }
    }
    Ok(own)
}
pub(super) fn installed_command(ctx: &CoreContext, id: &str, command: &str) -> Option<Installed> {
    let native = if id.starts_with("raycast.") {
        let slug = slug(id).ok()?;
        let canonical = super::v2_runtime::canonical_id(&slug);
        installed(ctx, &canonical).ok()
    } else if id.starts_with("com.tezbar.native.") {
        installed(ctx, id).ok()
    } else {
        None
    };
    if let Some(extension) = native {
        let active = extension.manifest["healthError"].is_null()
            && extension.manifest["commands"]
                .as_array()?
                .iter()
                .any(|c| c["name"] == command && c["runtime"] == "rust" && compatible(c, true));
        if active {
            return Some(extension);
        }
        if id.starts_with("com.tezbar.native.") {
            return None;
        }
    }
    if id.starts_with("com.tezbar.native.") {
        return None;
    }
    let extension = installed(ctx, id).ok()?;
    (compatible(&extension.manifest, false)
        && extension.manifest["commands"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|cmd| cmd["name"].as_str() == Some(command) && compatible(cmd, true)))
    .then_some(extension)
}
pub(super) fn verified_native_alias(ctx: &CoreContext, requested: &str, command: &str) -> Option<String> {
    let mut matches = collect(ctx).into_iter().filter(|entry| {
        entry.id.starts_with("com.tezbar.native.") &&
        entry.manifest["healthError"].is_null() &&
        entry.manifest["replaces"].as_array().is_some_and(|a| a.iter().any(|id| id.as_str() == Some(requested))) &&
        super::process_install::resolve(ctx, &entry.id).is_ok() &&
        installed_command(ctx, &entry.id, command).is_some()
    }).map(|e| e.id);
    let first = matches.next()?;
    matches.next().is_none().then_some(first)
}

pub(super) fn route(ctx: &CoreContext,requested: &str, command: &str)->Result<String,String>{
let _ = (ctx,command);
let canonical = if requested.starts_with("com.tezbar.native.") {
            requested.to_string()
        } else {
            registry::verified_native_alias(ctx, requested, command).ok_or("Unknown or uncovered extension alias")?
        };
Ok(canonical)
}

pub(super) fn test_icon(e:&Installed)->Option<String>{icon_path(e)}
pub(super) fn test_preferences(e:&Installed)->Result<PathBuf,String>{preferences_path(e)}

}
