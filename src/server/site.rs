//! The site remembered on disk: the devices of the last search and the points of each device read,
//! as the interface saw them. After a restart they show at once; a new search brings them live
//! again (concept-explorer step D).

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use axum::Json;
use axum::extract::{Query, State};
use axum::http::{StatusCode, header};
use axum::response::IntoResponse;

use super::ApiError;
use super::json::PointView;
use super::session::{AppState, ScanStatus, Shared};

const FILE: &str = "site.json";
const SITES: &str = "sites";
/// The state of a finished search, the only one a kept site can hold (`GET /api/scan`).
const FINISHED: &str = "done";
/// More devices than a site has; the tree draws a node per device, so a file with a hundred
/// thousand of them would freeze the page on one click.
const MOST_DEVICES: usize = 5_000;
/// The longest name a site can be given; the field in the page stops there too.
const LONGEST_TITLE: usize = 80;
/// The longest grouping pattern; the field in the page stops there too.
const LONGEST_PATTERN: usize = 200;

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Site {
    /// What was searched: a subnet, BBMD or adapter. Each name is kept as a site of its own.
    #[serde(default)]
    pub name: String,
    /// The name the technician gave it (*Sample School*); none until renamed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// When it was last saved, milliseconds since 1970.
    #[serde(default)]
    pub saved_ms: u64,
    /// The scan as `GET /api/scan` answered it when done.
    #[serde(default)]
    pub scan: Option<Value>,
    /// The points of each device as `GET /api/devices/{instance}/points/last` answers them.
    #[serde(default)]
    pub points: Map<String, Value>,
    /// The technician's own pattern that groups the points of every device of the site; none
    /// groups them by itself (docs/concept-toolbox.md part 4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grouping: Option<String>,
}

/// What the technician chose for a site: kept when the same network is searched again.
#[derive(Default)]
pub struct Choices {
    title: Option<String>,
    grouping: Option<String>,
}

impl Choices {
    fn of(site: &Site) -> Self {
        Choices {
            title: site.title.clone(),
            grouping: site.grouping.clone(),
        }
    }

    pub fn apply(self, site: &mut Site) {
        site.title = self.title;
        site.grouping = self.grouping;
    }
}

/// The points of every device read, by device: read since the start, else as remembered. For the
/// site's problems and the search across the site.
pub async fn points(State(state): State<Shared>) -> Json<Map<String, Value>> {
    let mut all = state
        .site
        .lock()
        .expect("site lock")
        .as_ref()
        .map(|site| site.points.clone())
        .unwrap_or_default();
    for (device, points) in state.readings.lock().expect("readings lock").iter() {
        let points: Vec<PointView> = points.iter().map(PointView::new).collect();
        all.insert(device.to_string(), serde_json::json!({ "points": points }));
    }
    Json(all)
}

pub fn load(dir: &Path) -> io::Result<Option<Site>> {
    match fs::read_to_string(dir.join(FILE)) {
        Ok(text) => serde_json::from_str(&text)
            .map(Some)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn save(dir: &Path, site: &Site) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    keep(dir, site)?;
    let json = serde_json::to_string(site).map_err(io::Error::other)?;
    replace(&dir.join(FILE), &json)
}

/// Writes the site into the list of kept sites, without making it the one shown.
fn keep(dir: &Path, site: &Site) -> io::Result<()> {
    if site.name.is_empty() {
        return Ok(());
    }
    fs::create_dir_all(dir.join(SITES))?;
    let json = serde_json::to_string(site).map_err(io::Error::other)?;
    replace(&kept_path(dir, &site.name), &json)
}

fn kept_path(dir: &Path, name: &str) -> PathBuf {
    dir.join(SITES).join(format!("{}.json", file_stem(name)))
}

/// The kept site of that name, if there is one.
fn kept_site(dir: &Path, name: &str) -> Option<Site> {
    let text = fs::read_to_string(kept_path(dir, name)).ok()?;
    serde_json::from_str::<Site>(&text)
        .ok()
        .filter(|site| site.name == name)
}

/// Written beside and renamed, so a crash never leaves half a file.
fn replace(path: &Path, json: &str) -> io::Result<()> {
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, json)?;
    fs::rename(temporary, path)
}

/// A file name from a site's name, which may hold slashes and anything else from a file brought in.
fn file_stem(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Every site kept in the data folder, the latest first. A file that cannot be read is left out.
fn saved_sites(dir: &Path) -> Vec<Site> {
    let Ok(files) = fs::read_dir(dir.join(SITES)) else {
        return Vec::new();
    };
    let mut sites: Vec<Site> = files
        .flatten()
        .filter_map(|file| fs::read_to_string(file.path()).ok())
        .filter_map(|text| serde_json::from_str(&text).ok())
        .collect();
    sites.sort_by_key(|site: &Site| std::cmp::Reverse(site.saved_ms));
    sites
}

#[derive(Serialize)]
pub struct SiteView {
    /// The network searched, which the site is kept by.
    name: String,
    title: Option<String>,
    grouping: Option<String>,
    /// Whether it is the site shown now.
    open: bool,
    saved_ms: u64,
    devices: usize,
    /// Devices whose points are kept.
    read: usize,
}

/// The sites kept, to go back to one without searching.
pub async fn list(State(state): State<Shared>) -> Json<Vec<SiteView>> {
    let shown = state
        .site
        .lock()
        .expect("site lock")
        .as_ref()
        .map(|site| site.name.clone());
    let sites = state
        .data_dir
        .as_deref()
        .map(saved_sites)
        .unwrap_or_default();
    Json(
        sites
            .into_iter()
            .map(|site| SiteView {
                devices: site
                    .scan
                    .as_ref()
                    .and_then(|scan| scan["devices"].as_array())
                    .map_or(0, Vec::len),
                read: site.points.len(),
                open: shown.as_ref() == Some(&site.name),
                title: site.title,
                grouping: site.grouping,
                name: site.name,
                saved_ms: site.saved_ms,
            })
            .collect(),
    )
}

/// Shows a site as remembered, instead of what was on the screen.
fn show(state: &AppState, site: Site) {
    *state.site.lock().expect("site lock") = Some(site);
    state.set_scan(ScanStatus::Idle);
    state.readings.lock().expect("readings lock").clear();
    state.reads.lock().expect("reads lock").clear();
    state.devices.lock().expect("devices lock").clear();
    super::remembered::restore(state);
}

#[derive(Deserialize)]
pub struct OpenRequest {
    name: String,
}

pub async fn open(
    State(state): State<Shared>,
    Json(request): Json<OpenRequest>,
) -> Result<StatusCode, ApiError> {
    let site = state
        .data_dir
        .as_deref()
        .map(saved_sites)
        .unwrap_or_default()
        .into_iter()
        .find(|site| site.name == request.name)
        .ok_or_else(|| ApiError::bad_request(format!("no site named {}", request.name)))?;
    show(&state, site);
    Ok(StatusCode::NO_CONTENT)
}

/// Forgets a kept site. Only sites the list knows can go, whatever the request says.
pub async fn delete(
    State(state): State<Shared>,
    Json(request): Json<OpenRequest>,
) -> Result<StatusCode, ApiError> {
    let dir = state
        .data_dir
        .as_deref()
        .ok_or_else(|| ApiError::conflict("bacnetio has no data folder to save to"))?;
    if !saved_sites(dir)
        .iter()
        .any(|site| site.name == request.name)
    {
        return Err(ApiError::bad_request(format!(
            "no site named {}",
            request.name
        )));
    }
    fs::remove_file(kept_path(dir, &request.name))
        .map_err(|error| ApiError::internal(error.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct RenameRequest {
    name: String,
    title: String,
}

/// Gives a kept site a name of the technician's; an empty one goes back to the network.
pub async fn rename(
    State(state): State<Shared>,
    Json(request): Json<RenameRequest>,
) -> Result<StatusCode, ApiError> {
    let title: String = request.title.trim().chars().take(LONGEST_TITLE).collect();
    let title = (!title.is_empty()).then_some(title);
    change_site(&state, &request.name, |site| site.title = title.clone())
}

#[derive(Deserialize)]
pub struct GroupingRequest {
    name: String,
    /// The pattern, a JavaScript regular expression the page applies; empty or none groups by
    /// itself again.
    pattern: Option<String>,
}

/// Gives a kept site its own pattern to group points by. The page checks the pattern, since it is
/// the page that applies it; here only its length.
pub async fn grouping(
    State(state): State<Shared>,
    Json(request): Json<GroupingRequest>,
) -> Result<StatusCode, ApiError> {
    // spaces count in a pattern: only one of nothing else is none
    let pattern = request.pattern.unwrap_or_default();
    if pattern.chars().count() > LONGEST_PATTERN {
        return Err(ApiError::bad_request(format!(
            "a grouping pattern has at most {LONGEST_PATTERN} characters"
        )));
    }
    let pattern = (!pattern.trim().is_empty()).then_some(pattern);
    change_site(&state, &request.name, |site| {
        site.grouping = pattern.clone();
    })
}

/// Changes a kept site, the one shown or one in the data folder.
fn change_site(
    state: &AppState,
    name: &str,
    change: impl Fn(&mut Site),
) -> Result<StatusCode, ApiError> {
    let shown = state
        .site
        .lock()
        .expect("site lock")
        .as_ref()
        .is_some_and(|site| site.name == name);
    if shown {
        state.remember(change);
        return Ok(StatusCode::NO_CONTENT);
    }
    let dir = state
        .data_dir
        .as_deref()
        .ok_or_else(|| ApiError::conflict("bacnetio has no data folder to save to"))?;
    let mut site = kept_site(dir, name)
        .ok_or_else(|| ApiError::bad_request(format!("no site named {name}")))?;
    change(&mut site);
    keep(dir, &site).map_err(|error| ApiError::internal(error.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct ExportQuery {
    /// A kept site; without it the site shown now.
    name: Option<String>,
}

/// A site as a file, to keep it or to hand it to a colleague.
pub async fn export(
    State(state): State<Shared>,
    Query(query): Query<ExportQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let shown = state.site.lock().expect("site lock").clone();
    let site = match query.name {
        Some(name) if shown.as_ref().is_none_or(|site| site.name != name) => state
            .data_dir
            .as_deref()
            .and_then(|dir| kept_site(dir, &name))
            .ok_or_else(|| ApiError::bad_request(format!("no site named {name}")))?,
        _ => shown.ok_or_else(|| ApiError::conflict("search for devices first"))?,
    };
    let disposition = format!(
        "attachment; filename=\"bacnetio-site-{}.json\"",
        file_stem(&site.name)
    );
    Ok(([(header::CONTENT_DISPOSITION, disposition)], Json(site)))
}

/// The numbers of a device in a site file; everything else in it is shown as escaped text.
#[derive(Deserialize)]
struct CheckedDevice {
    #[serde(rename = "instance")]
    _instance: u32,
    #[serde(rename = "network")]
    _network: Option<u16>,
    #[serde(default, rename = "objects")]
    _objects: Option<CheckedObjects>,
}

#[derive(Deserialize)]
struct CheckedObjects {
    #[serde(rename = "read")]
    _read: usize,
    #[serde(rename = "total")]
    _total: Option<usize>,
    #[serde(default, rename = "types")]
    _types: BTreeMap<u32, usize>,
}

#[derive(Deserialize)]
struct CheckedRouter {
    #[serde(rename = "networks")]
    _networks: Vec<u16>,
}

/// What the interface reads as text or walks as a list. A number in `value` or `status` throws in
/// the page (`value.endsWith`, `status.includes`) and takes the whole redraw with it.
#[derive(Deserialize)]
struct CheckedPoint {
    #[serde(rename = "type")]
    _object_type: u32,
    #[serde(rename = "instance")]
    _instance: u32,
    #[serde(default, rename = "value")]
    _value: String,
    #[serde(default, rename = "status")]
    _status: String,
    #[serde(default, rename = "units")]
    _units: Option<String>,
    #[serde(default, rename = "state_texts")]
    _state_texts: Vec<String>,
    #[serde(default, rename = "binary_texts")]
    _binary_texts: Vec<String>,
    #[serde(default, rename = "notification_class")]
    _notification_class: Option<u32>,
}

#[derive(Deserialize)]
struct CheckedPoints {
    #[serde(rename = "points")]
    _points: Vec<CheckedPoint>,
}

/// A file from outside is checked for what the interface puts into its pages as it is, numbers
/// above all: a script in place of a device ID would run in the page that can write to the plant.
/// Beyond that it must not be able to wedge the page: a search that never ends, a value the page
/// reads as text, or more devices than the tree can draw.
fn well_formed(site: &Site) -> bool {
    fn fits<T: serde::de::DeserializeOwned>(value: &Value) -> bool {
        serde_json::from_value::<T>(value.clone()).is_ok()
    }
    let Some(scan) = &site.scan else {
        return false;
    };
    // a kept site is a finished search; "running" would set the page polling for a scan that is
    // not there, with no way out of "Searching …"
    scan["state"] == FINISHED
        && scan["devices"]
            .as_array()
            .is_some_and(|devices| devices.len() <= MOST_DEVICES)
        && fits::<Vec<CheckedDevice>>(&scan["devices"])
        && (scan["routers"].is_null() || fits::<Vec<CheckedRouter>>(&scan["routers"]))
        && site
            .points
            .iter()
            .all(|(device, points)| device.parse::<u32>().is_ok() && fits::<CheckedPoints>(points))
}

/// A site file brought back in. It comes from outside, so its form is checked before it is kept.
pub async fn import(
    State(state): State<Shared>,
    Json(mut site): Json<Site>,
) -> Result<StatusCode, ApiError> {
    if !well_formed(&site) {
        return Err(ApiError::bad_request("this is not a bacnetio site file"));
    }
    if site.name.is_empty() {
        site.name = "imported".to_string();
    }
    if let Some(dir) = &state.data_dir {
        save(dir, &site).map_err(|error| ApiError::internal(error.to_string()))?;
    }
    show(&state, site);
    Ok(StatusCode::NO_CONTENT)
}

impl AppState {
    /// What was chosen for the site kept for that network, so a new search keeps it.
    pub(super) fn kept_choices(&self, name: &str) -> Choices {
        if let Some(site) = self.site.lock().expect("site lock").as_ref()
            && site.name == name
        {
            return Choices::of(site);
        }
        self.data_dir
            .as_deref()
            .and_then(|dir| kept_site(dir, name))
            .map(|site| Choices::of(&site))
            .unwrap_or_default()
    }

    /// Changes the remembered site and writes it to the data folder, if there is one.
    pub(super) fn remember(&self, change: impl FnOnce(&mut Site)) {
        let mut site = self.site.lock().expect("site lock");
        let site = site.get_or_insert_with(Site::default);
        change(site);
        site.saved_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_millis() as u64);
        if let Some(dir) = &self.data_dir
            && let Err(reason) = save(dir, site)
        {
            // the site memory is a convenience; working on goes on without it
            eprintln!(
                "Warning: cannot save the site to {}: {reason}",
                dir.display()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn a_site_is_saved_and_loaded_again() {
        let dir = std::env::temp_dir().join(format!("bacnetio-site-{}", std::process::id()));
        assert!(load(&dir).unwrap().is_none());
        let mut site = Site {
            saved_ms: 7,
            scan: Some(serde_json::json!({ "state": "done", "devices": [] })),
            ..Site::default()
        };
        site.points.insert(
            "21".into(),
            serde_json::json!({ "points": [], "notes": [] }),
        );
        save(&dir, &site).unwrap();
        let loaded = load(&dir).unwrap().unwrap();
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(loaded.saved_ms, 7);
        assert_eq!(loaded.scan, site.scan);
        assert_eq!(loaded.points, site.points);
    }

    #[test]
    fn a_site_file_with_anything_but_numbers_where_numbers_belong_is_refused() {
        let site = |scan: Value, points: Value| Site {
            scan: Some(scan),
            points: serde_json::from_value(points).unwrap(),
            ..Site::default()
        };
        let device = serde_json::json!({ "instance": 21, "network": null, "name": "<b>any text</b>",
            "objects": { "read": 3, "total": 3, "done": true, "error": null, "types": { "0": 2 } } });
        let good =
            serde_json::json!({ "21": { "points": [{ "type": 0, "instance": 1, "name": "x" }] } });
        assert!(well_formed(&site(
            serde_json::json!({ "state": "done", "devices": [device] }),
            good.clone()
        )));
        let script = "<img src=x onerror=alert(1)>";
        for bad in [
            serde_json::json!({ "state": "done", "devices": "none" }),
            serde_json::json!({ "state": "done", "devices": [{ "instance": script, "network": null }] }),
            serde_json::json!({ "state": "done", "devices": [{ "instance": 1, "network": script }] }),
            serde_json::json!({ "state": "done", "devices": [{ "instance": 1, "network": null, "objects": { "read": 1, "total": script } }] }),
            serde_json::json!({ "state": "done", "devices": [], "routers": [{ "networks": [script] }] }),
        ] {
            assert!(!well_formed(&site(bad, good.clone())));
        }
        let scan = serde_json::json!({ "state": "done", "devices": [] });
        assert!(!well_formed(&site(
            scan.clone(),
            serde_json::json!({ "21": { "points": [{ "type": script, "instance": 1 }] } })
        )));
        assert!(!well_formed(&site(
            scan,
            serde_json::json!({ script: { "points": [] } })
        )));
        assert!(!well_formed(&Site::default()));
    }

    /// A file that would leave the page unusable: a search that never ends, a value the page reads
    /// as text, or more devices than the tree can draw.
    #[test]
    fn a_site_file_that_would_wedge_the_page_is_refused() {
        let site = |scan: Value, points: Value| Site {
            scan: Some(scan),
            points: serde_json::from_value(points).unwrap(),
            ..Site::default()
        };
        let empty = serde_json::json!({ "21": { "points": [] } });
        for state in ["running", "idle", "failed"] {
            let scan = serde_json::json!({ "state": state, "devices": [] });
            assert!(
                !well_formed(&site(scan, empty.clone())),
                "a site file with the state {state} was taken in"
            );
        }
        let done = serde_json::json!({ "state": "done", "devices": [] });
        for bad in [
            serde_json::json!({ "type": 0, "instance": 1, "value": 21.5 }),
            serde_json::json!({ "type": 0, "instance": 1, "status": ["fault"] }),
            serde_json::json!({ "type": 0, "instance": 1, "units": 62 }),
            serde_json::json!({ "type": 0, "instance": 1, "state_texts": "Off" }),
        ] {
            let points = serde_json::json!({ "21": { "points": [bad] } });
            assert!(!well_formed(&site(done.clone(), points)));
        }
        let many: Vec<Value> = (0..=MOST_DEVICES)
            .map(|instance| serde_json::json!({ "instance": instance, "network": null }))
            .collect();
        let crowd = serde_json::json!({ "state": "done", "devices": many });
        assert!(!well_formed(&site(crowd, empty)));
    }

    #[test]
    fn each_name_is_kept_as_a_site_of_its_own_and_its_file_name_is_safe() {
        let dir = std::env::temp_dir().join(format!("bacnetio-sites-{}", std::process::id()));
        for (name, saved_ms) in [("10.0.0.0/24", 1), ("../../evil", 2), ("10.0.0.0/24", 3)] {
            let site = Site {
                name: name.into(),
                saved_ms,
                ..Site::default()
            };
            save(&dir, &site).unwrap();
        }
        let sites = saved_sites(&dir);
        let files: Vec<_> = fs::read_dir(dir.join(SITES))
            .unwrap()
            .flatten()
            .map(|file| file.file_name())
            .collect();
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(
            sites.iter().map(|site| site.saved_ms).collect::<Vec<_>>(),
            [3, 2]
        );
        assert_eq!(files.len(), 2);
        assert!(
            files
                .iter()
                .all(|file| !file.to_string_lossy().contains('/'))
        );
        assert_eq!(file_stem("../../evil"), "______evil");
    }

    #[test]
    fn a_name_given_to_a_kept_site_stays_with_it() {
        let dir = std::env::temp_dir().join(format!("bacnetio-title-{}", std::process::id()));
        let mut site = Site {
            name: "10.0.0.0/24".into(),
            ..Site::default()
        };
        save(&dir, &site).unwrap();
        site.title = Some("Sample School".into());
        keep(&dir, &site).unwrap();
        let kept = kept_site(&dir, "10.0.0.0/24");
        let other = kept_site(&dir, "10.0.0.0_24");
        let shown = load(&dir).unwrap().unwrap();
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(kept.unwrap().title.as_deref(), Some("Sample School"));
        assert!(
            other.is_none(),
            "a name that only shares the file is another site"
        );
        assert_eq!(
            shown.title, None,
            "keeping a site leaves the one shown as it is"
        );
    }

    /// The grouping pattern belongs to the site: set on the one shown or a kept one, kept when the
    /// same network is searched again, empty for the automatic grouping, and refused when too long.
    #[tokio::test]
    async fn a_grouping_pattern_stays_with_its_site() {
        let dir = std::env::temp_dir().join(format!("bacnetio-grouping-{}", std::process::id()));
        let mut state = AppState::default();
        state.data_dir = Some(dir.clone());
        let state = Arc::new(state);
        state.remember(|site| site.name = "10.0.0.0/24".into());
        let set = |name: &str, pattern: Option<&str>| {
            grouping(
                State(Arc::clone(&state)),
                Json(GroupingRequest {
                    name: name.into(),
                    pattern: pattern.map(Into::into),
                }),
            )
        };
        assert!(set("10.0.0.0/24", Some(r"^(\w+) ")).await.is_ok());
        assert_eq!(
            state.kept_choices("10.0.0.0/24").grouping.as_deref(),
            Some(r"^(\w+) "),
            "with its space, on the site shown"
        );
        assert_eq!(
            kept_site(&dir, "10.0.0.0/24").unwrap().grouping.as_deref(),
            Some(r"^(\w+) "),
            "and in its file"
        );
        let too_long = "a".repeat(LONGEST_PATTERN + 1);
        assert!(set("10.0.0.0/24", Some(&too_long)).await.is_err());
        assert!(
            set("192.168.1.0/24", Some("x")).await.is_err(),
            "no such site"
        );
        assert!(set("10.0.0.0/24", Some("  ")).await.is_ok());
        let cleared = state.kept_choices("10.0.0.0/24").grouping;
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(cleared, None, "empty groups by itself again");
    }
}
