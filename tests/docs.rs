//! The docs keep up with the code: every route of the web API is described in docs/api.md, and
//! every simulator in examples/ in the README. A route or simulator added without its description
//! fails here.

use std::fs;

/// "/api/devices/{device}/points" with any name in the braces, as the docs may name it otherwise.
fn described(route: &str, docs: &str) -> bool {
    let parts: Vec<&str> = route.split('/').collect();
    docs.match_indices(parts[1]).any(|(start, _)| {
        let mut rest = &docs[start - 1..];
        for part in &parts[1..] {
            let Some(after) = rest.strip_prefix('/') else {
                return false;
            };
            rest = if part.starts_with('{') {
                // a placeholder: any name in braces, or a number given as an example
                let end = after.find(['/', '`', ' ', '?']).unwrap_or(after.len());
                &after[end..]
            } else {
                match after.strip_prefix(part) {
                    Some(next) => next,
                    None => return false,
                }
            };
        }
        // the route ends here, not inside a longer one
        !rest.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '/' || c == '.')
    })
}

#[test]
fn every_route_is_in_the_api_docs() {
    let code = fs::read_to_string("src/server/mod.rs").unwrap();
    let docs = fs::read_to_string("docs/api.md").unwrap();
    let routes: Vec<&str> = code
        .split('"')
        .filter(|text| text.starts_with("/api/"))
        .collect();
    assert!(routes.len() > 40, "found {} routes", routes.len());
    let missing: Vec<&&str> = routes
        .iter()
        .filter(|route| !described(route, &docs))
        .collect();
    assert_eq!(
        missing,
        Vec::<&&str>::new(),
        "describe these in docs/api.md"
    );
}

#[test]
fn every_simulator_is_in_the_readme() {
    let readme = fs::read_to_string("README.md").unwrap();
    let missing: Vec<String> = fs::read_dir("examples")
        .unwrap()
        .flatten()
        .filter_map(|file| {
            let name = file.file_name().to_string_lossy().into_owned();
            name.strip_suffix(".rs").map(str::to_string)
        })
        .filter(|name| !readme.contains(name.as_str()))
        .collect();
    assert_eq!(missing, Vec::<String>::new(), "describe these in README.md");
}

#[test]
fn a_route_counts_only_where_it_stands_whole() {
    let docs = "`GET /api/devices/{device}/points` and `POST /api/devices/1001/read`";
    assert!(described("/api/devices/{device}/points", docs));
    assert!(described("/api/devices/{d}/read", docs));
    assert!(!described(
        "/api/devices/{device}",
        "`GET /api/devices/1/points`"
    ));
    assert!(!described(
        "/api/recording/mode",
        "`POST /api/recording/modes`"
    ));
}
