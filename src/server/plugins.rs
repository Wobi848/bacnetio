//! Installed plugins (docs/concept-plugins.md): kept as files in the data folder under `plugins/`,
//! listed, switched on and off, removed, and served to a sandboxed frame.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use axum::Json;
use axum::extract::{Path as UrlPath, Query, State};
use axum::http::header;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::ApiError;
use super::session::Shared;
use crate::plugin::{self, Manifest};

const FOLDER: &str = "plugins";
const PAGE_SUFFIX: &str = ".bacnetio-plugin.html";
const STORE_SUFFIX: &str = ".store.json";
/// The ids switched off; a plugin not named here is on.
const OFF_FILE: &str = "off.json";
/// A plugin's own store holds results, not plants: a megabyte is plenty.
pub const MOST_STORE_BYTES: usize = 1024 * 1024;

/// The page may run its own scripts and styles and nothing else: no requests, no access to
/// bacnetio's page or storage (an opaque origin), no popups, no navigation of bacnetio.
const PAGE_POLICY: &str = "sandbox allow-scripts; default-src 'none'; script-src 'unsafe-inline'; \
     style-src 'unsafe-inline'; img-src data:; font-src data:";

#[derive(Serialize)]
pub struct PluginView {
    #[serde(flatten)]
    manifest: Manifest,
    on: bool,
}

#[derive(Deserialize)]
pub struct InstallQuery {
    /// Replace a plugin with the same id; asked of the user first.
    #[serde(default)]
    replace: bool,
}

#[derive(Deserialize)]
pub struct Switch {
    on: bool,
}

fn folder(state: &Shared) -> Result<PathBuf, ApiError> {
    state
        .data_dir
        .as_ref()
        .map(|dir| dir.join(FOLDER))
        .ok_or_else(|| ApiError::conflict("plugins need a data folder"))
}

fn page_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}{PAGE_SUFFIX}"))
}

/// The plugin of that id, if one is installed. The id is checked first: it becomes a file name.
fn installed(dir: &Path, id: &str) -> Result<(String, Manifest), ApiError> {
    let missing = || ApiError::not_found(format!("no plugin {id} is installed"));
    if !id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(missing());
    }
    let page = fs::read_to_string(page_path(dir, id)).map_err(|_| missing())?;
    let manifest = plugin::manifest(&page).map_err(ApiError::internal)?;
    Ok((page, manifest))
}

fn switched_off(dir: &Path) -> Vec<String> {
    fs::read_to_string(dir.join(OFF_FILE))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn set_off(dir: &Path, id: &str, off: bool) -> io::Result<()> {
    let mut ids = switched_off(dir);
    ids.retain(|other| other != id);
    if off {
        ids.push(id.to_string());
    }
    replace(&dir.join(OFF_FILE), &serde_json::to_string(&ids)?)
}

/// Written beside and renamed, so a crash never leaves half a file.
fn replace(path: &Path, text: &str) -> io::Result<()> {
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, text)?;
    fs::rename(temporary, path)
}

fn io_error(error: io::Error) -> ApiError {
    ApiError::internal(format!("cannot keep the plugin: {error}"))
}

pub async fn list(State(state): State<Shared>) -> Result<Json<Vec<PluginView>>, ApiError> {
    let dir = folder(&state)?;
    let off = switched_off(&dir);
    let Ok(files) = fs::read_dir(&dir) else {
        return Ok(Json(Vec::new()));
    };
    let mut plugins: Vec<PluginView> = files
        .flatten()
        .map(|file| file.path())
        .filter(|path| path.to_string_lossy().ends_with(PAGE_SUFFIX))
        .filter_map(|path| fs::read_to_string(path).ok())
        .filter_map(|page| plugin::manifest(&page).ok())
        .map(|manifest| PluginView {
            on: !off.contains(&manifest.id),
            manifest,
        })
        .collect();
    plugins.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
    Ok(Json(plugins))
}

pub async fn install(
    State(state): State<Shared>,
    Query(query): Query<InstallQuery>,
    page: String,
) -> Result<Json<PluginView>, ApiError> {
    let manifest = plugin::manifest(&page).map_err(ApiError::bad_request)?;
    let dir = folder(&state)?;
    if !query.replace
        && let Ok((_, old)) = installed(&dir, &manifest.id)
    {
        return Err(ApiError::conflict(format!(
            "plugin {} {} is installed already",
            old.name, old.version
        )));
    }
    fs::create_dir_all(&dir).map_err(io_error)?;
    replace(&page_path(&dir, &manifest.id), &page).map_err(io_error)?;
    set_off(&dir, &manifest.id, false).map_err(io_error)?;
    Ok(Json(PluginView { manifest, on: true }))
}

pub async fn switch(
    State(state): State<Shared>,
    UrlPath(id): UrlPath<String>,
    Json(request): Json<Switch>,
) -> Result<Json<PluginView>, ApiError> {
    let dir = folder(&state)?;
    let (_, manifest) = installed(&dir, &id)?;
    set_off(&dir, &id, !request.on).map_err(io_error)?;
    Ok(Json(PluginView {
        manifest,
        on: request.on,
    }))
}

/// The page and the plugin's store go; nothing else of bacnetio's data is touched.
pub async fn remove(
    State(state): State<Shared>,
    UrlPath(id): UrlPath<String>,
) -> Result<Json<Value>, ApiError> {
    let dir = folder(&state)?;
    installed(&dir, &id)?;
    fs::remove_file(page_path(&dir, &id)).map_err(io_error)?;
    match fs::remove_file(dir.join(format!("{id}{STORE_SUFFIX}"))) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(io_error(error)),
    }
    set_off(&dir, &id, false).map_err(io_error)?;
    Ok(Json(Value::Null))
}

/// The plugin's page for its frame, only while it is on.
pub async fn page(
    State(state): State<Shared>,
    UrlPath(id): UrlPath<String>,
) -> Result<impl IntoResponse, ApiError> {
    let dir = folder(&state)?;
    let (page, _) = installed(&dir, &id)?;
    if switched_off(&dir).contains(&id) {
        return Err(ApiError::conflict(format!("plugin {id} is switched off")));
    }
    Ok((
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::CONTENT_SECURITY_POLICY, PAGE_POLICY),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        page,
    ))
}

pub async fn store(
    State(state): State<Shared>,
    UrlPath(id): UrlPath<String>,
) -> Result<Json<Value>, ApiError> {
    let dir = folder(&state)?;
    installed(&dir, &id)?;
    let kept = fs::read_to_string(dir.join(format!("{id}{STORE_SUFFIX}")))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(Value::Null);
    Ok(Json(kept))
}

pub async fn save_store(
    State(state): State<Shared>,
    UrlPath(id): UrlPath<String>,
    Json(data): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let dir = folder(&state)?;
    installed(&dir, &id)?;
    let text = serde_json::to_string(&data).map_err(ApiError::internal)?;
    replace(&dir.join(format!("{id}{STORE_SUFFIX}")), &text).map_err(io_error)?;
    Ok(Json(Value::Null))
}
