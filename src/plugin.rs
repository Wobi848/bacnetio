//! A plugin file and the description it carries (docs/concept-plugins.md), without files or HTTP.

use serde::{Deserialize, Serialize};

/// The plugin API this bacnetio answers (docs/plugins.md).
pub const API_VERSION: u32 = 1;
/// A plugin is one page with its script and data inside; a few megabytes are plenty.
pub const MOST_PLUGIN_BYTES: usize = 8 * 1024 * 1024;

const DESCRIPTION_START: &str = r#"<script type="application/json" id="bacnetio-plugin">"#;
const MOST_ID_CHARS: usize = 40;
const MOST_NAME_CHARS: usize = 60;
const MOST_VERSION_CHARS: usize = 20;
const MOST_DESCRIPTION_CHARS: usize = 300;

/// What a plugin says about itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub api: u32,
    #[serde(default)]
    pub description: String,
}

/// The description of a plugin page, checked, or why it is not a plugin bacnetio can run.
pub fn manifest(page: &str) -> Result<Manifest, String> {
    let start = page
        .find(DESCRIPTION_START)
        .ok_or("not a bacnetio plugin: the description is missing")?
        + DESCRIPTION_START.len();
    let end = page[start..]
        .find("</script>")
        .ok_or("not a bacnetio plugin: the description is not closed")?
        + start;
    let manifest: Manifest = serde_json::from_str(&page[start..end])
        .map_err(|error| format!("the plugin's description cannot be read: {error}"))?;
    check(&manifest)?;
    Ok(manifest)
}

fn check(manifest: &Manifest) -> Result<(), String> {
    let id = &manifest.id;
    let id_ok = !id.is_empty()
        && id.chars().count() <= MOST_ID_CHARS
        && !id.starts_with('-')
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !id_ok {
        return Err(format!(
            "the plugin id \"{id}\" must be up to {MOST_ID_CHARS} small letters, digits and dashes"
        ));
    }
    if manifest.name.trim().is_empty() || manifest.name.chars().count() > MOST_NAME_CHARS {
        return Err(format!(
            "the plugin's name must be 1 to {MOST_NAME_CHARS} characters"
        ));
    }
    if manifest.version.trim().is_empty() || manifest.version.chars().count() > MOST_VERSION_CHARS {
        return Err(format!(
            "the plugin's version must be 1 to {MOST_VERSION_CHARS} characters"
        ));
    }
    if manifest.description.chars().count() > MOST_DESCRIPTION_CHARS {
        return Err(format!(
            "the plugin's description must be at most {MOST_DESCRIPTION_CHARS} characters"
        ));
    }
    if manifest.api != API_VERSION {
        return Err(format!(
            "the plugin needs plugin API version {}; this bacnetio has version {API_VERSION}",
            manifest.api
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(description: &str) -> String {
        [
            "<!doctype html><title>x</title>",
            DESCRIPTION_START,
            description,
            "</script><p>hi</p>",
        ]
        .concat()
    }

    #[test]
    fn a_plugin_names_itself() {
        let found = manifest(&page(
            r#"{"id":"check","name":"Site check","version":"1.0.0","api":1,"description":"Checks"}"#,
        ))
        .unwrap();
        assert_eq!(found.id, "check");
        assert_eq!(found.name, "Site check");
        assert_eq!(found.description, "Checks");
    }

    #[test]
    fn what_is_not_a_plugin_is_refused_in_words() {
        let refused = |description: &str| manifest(&page(description)).unwrap_err();
        assert!(
            manifest("<html>just a page</html>")
                .unwrap_err()
                .contains("not a bacnetio plugin")
        );
        assert!(refused("{not json").contains("cannot be read"));
        // the id becomes a file name: nothing that could leave the plugin folder
        assert!(refused(r#"{"id":"../x","name":"n","version":"1","api":1}"#).contains("id"));
        assert!(refused(r#"{"id":"Big","name":"n","version":"1","api":1}"#).contains("id"));
        assert!(refused(r#"{"id":"x","name":" ","version":"1","api":1}"#).contains("name"));
        assert!(refused(r#"{"id":"x","name":"n","version":"","api":1}"#).contains("version"));
        assert!(
            refused(r#"{"id":"x","name":"n","version":"1","api":2}"#).contains("API version 2")
        );
    }
}
