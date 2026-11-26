//! The texts revmesh shows the user, in English (the default) and German.
//!
//! Each language is a flat JSON catalog of message id → text in `crates/revmesh-core/locales/`
//! (the layout SolidCraft uses for its own catalogs). A text may hold `{name}` placeholders that
//! [`Locale::format`] fills in. A message missing from a catalog falls back to English, and an
//! id missing from English too is shown as the id itself.
//!
//! SolidCraft shows a plugin's manifest texts as the manifest has them and tells the plugin
//! nothing about the app's language, so the language is chosen when the plugin is packaged:
//! `cargo xtask dist` builds one package per locale, with [`localize_manifest`] applied to
//! `plugin.json` and the plugin component built with `REVMESH_LOCALE` set to the locale's code.

use std::collections::BTreeMap;
use std::fmt::Display;
use std::sync::OnceLock;

type Catalog = BTreeMap<String, String>;

/// A language revmesh speaks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Locale {
    /// English, the default and the fallback for every missing message.
    #[default]
    En,
    /// German.
    De,
}

impl Locale {
    /// Every locale, English first.
    pub const ALL: [Self; 2] = [Self::En, Self::De];

    /// The locale's language code, as in `locales/<code>.json`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::De => "de",
        }
    }

    /// The locale for a language tag such as `de` or `de-AT` (exact, then by base language);
    /// `None` for a language revmesh does not speak.
    #[must_use]
    pub fn from_code(tag: &str) -> Option<Self> {
        let base = tag.split(['-', '_']).next().unwrap_or_default().to_ascii_lowercase();
        Self::ALL.into_iter().find(|l| l.code() == base)
    }

    fn catalog(self) -> &'static Catalog {
        static EN: OnceLock<Catalog> = OnceLock::new();
        static DE: OnceLock<Catalog> = OnceLock::new();
        let (cell, json) = match self {
            Self::En => (&EN, include_str!("../locales/en.json")),
            Self::De => (&DE, include_str!("../locales/de.json")),
        };
        // The catalogs are compiled in and checked by the tests, so they always parse; an
        // empty catalog (every text falls back) is the safe answer if one ever did not.
        cell.get_or_init(|| serde_json::from_str(json).unwrap_or_default())
    }

    /// The text of message `id`: this locale's, else English, else `id` itself.
    #[must_use]
    pub fn text(self, id: &str) -> &str {
        self.catalog().get(id).or_else(|| Self::En.catalog().get(id)).map_or(id, String::as_str)
    }

    /// The text of message `id` with each `{name}` replaced by its value.
    #[must_use]
    pub fn format(self, id: &str, args: &[(&str, &dyn Display)]) -> String {
        let mut text = self.text(id).to_owned();
        for (name, value) in args {
            text = text.replace(&format!("{{{name}}}"), &value.to_string());
        }
        text
    }

    /// A number already formatted with a decimal point (`2.5`), written the way this locale
    /// writes decimals (`2,5` in German).
    #[must_use]
    pub fn decimal(self, number: &str) -> String {
        match self {
            Self::En => number.to_owned(),
            Self::De => number.replace('.', ","),
        }
    }
}

/// Puts `locale`'s texts into a parsed `plugin.json`: the description (`manifest.description`)
/// and each entry's option titles and descriptions (`manifest.<entry>.<option>.title` and
/// `.description`). Texts without a message in the catalog stay as they are; option values
/// (`enum`, `default`) are never touched, as the plugin parses them.
pub fn localize_manifest(manifest: &mut serde_json::Value, locale: Locale) {
    let translated = |id: String| {
        let text = locale.text(&id);
        (text != id).then(|| text.to_owned())
    };
    if let Some(text) = translated("manifest.description".into()) {
        manifest["description"] = text.into();
    }
    let Some(entries) = manifest.pointer_mut("/runtime/entries").and_then(serde_json::Value::as_array_mut) else {
        return;
    };
    for entry in entries {
        let name = entry["name"].as_str().unwrap_or_default().to_owned();
        let Some(properties) = entry.pointer_mut("/input_schema/properties").and_then(serde_json::Value::as_object_mut) else {
            continue;
        };
        for (option, schema) in properties {
            for field in ["title", "description"] {
                if schema.get(field).is_some()
                    && let Some(text) = translated(format!("manifest.{name}.{option}.{field}"))
                {
                    schema[field] = text.into();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placeholders(text: &str) -> Vec<&str> {
        let mut names: Vec<&str> = text.split('{').skip(1).filter_map(|s| s.split_once('}').map(|(name, _)| name)).collect();
        names.sort_unstable();
        names
    }

    #[test]
    fn every_catalog_parses_and_matches_english() {
        let en = Locale::En.catalog();
        assert!(!en.is_empty(), "the English catalog parses");
        for locale in Locale::ALL {
            let catalog = locale.catalog();
            for (id, text) in catalog {
                let english = en.get(id).unwrap_or_else(|| panic!("{}: {id} is not an English message", locale.code()));
                assert_eq!(placeholders(text), placeholders(english), "{}: {id} has other placeholders", locale.code());
                assert!(!text.trim().is_empty(), "{}: {id} is empty", locale.code());
            }
            for id in en.keys() {
                assert!(catalog.contains_key(id), "{}: {id} is not translated", locale.code());
            }
        }
    }

    #[test]
    fn texts_fall_back_to_english_then_to_the_id() {
        assert_eq!(Locale::De.text("progress.done"), "Fertig");
        assert_eq!(Locale::En.text("progress.done"), "Done");
        assert_eq!(Locale::De.text("no.such.message"), "no.such.message");
        assert_eq!(Locale::En.format("report.mesh.summary", &[("triangles", &12)]), "A mesh of 12 triangles");
        assert_eq!(Locale::De.format("report.mesh.summary", &[("triangles", &12)]), "Ein Netz aus 12 Dreiecken");
    }

    #[test]
    fn locales_come_from_language_tags() {
        assert_eq!(Locale::from_code("de"), Some(Locale::De));
        assert_eq!(Locale::from_code("de-AT"), Some(Locale::De));
        assert_eq!(Locale::from_code("EN_gb"), Some(Locale::En));
        assert_eq!(Locale::from_code("fr"), None);
        assert_eq!(Locale::from_code(""), None);
        assert_eq!(Locale::default(), Locale::En);
        assert_eq!(Locale::De.decimal("2.5"), "2,5");
        assert_eq!(Locale::En.decimal("2.5"), "2.5");
    }

    fn plugin_json() -> serde_json::Value {
        serde_json::from_str(include_str!("../../../plugin/revmesh/plugin.json")).expect("plugin.json parses")
    }

    #[test]
    fn the_english_catalog_holds_the_manifest_texts() {
        // plugin.json is the English manifest: localizing it to English changes nothing, so the
        // catalog and the manifest cannot drift apart.
        let mut manifest = plugin_json();
        localize_manifest(&mut manifest, Locale::En);
        assert_eq!(manifest, plugin_json());
    }

    #[test]
    fn the_german_manifest_translates_texts_but_not_values() {
        let mut manifest = plugin_json();
        localize_manifest(&mut manifest, Locale::De);
        let mode = &manifest["runtime"]["entries"][0]["input_schema"]["properties"]["mode"];
        assert_eq!(mode["title"], "Importieren als");
        assert!(mode["description"].as_str().expect("description").starts_with("auto: Features erkennen"));
        assert_eq!(mode["enum"], serde_json::json!(["auto", "mesh"]));
        assert_eq!(mode["default"], "auto");
        assert!(manifest["description"].as_str().expect("description").starts_with("Öffnet STL-, OBJ- und PLY-Netze"));
        assert_eq!(manifest["id"], "revmesh");
        // SolidCraft refuses descriptions over 240 characters.
        assert!(manifest["description"].as_str().expect("description").chars().count() <= 240);
    }

    #[test]
    fn manifests_without_entries_or_options_are_left_alone() {
        let mut bare = serde_json::json!({ "description": "x" });
        localize_manifest(&mut bare, Locale::De);
        assert!(bare["description"].as_str().expect("description").starts_with("Öffnet"));
        let mut no_schema = serde_json::json!({ "runtime": { "entries": [{ "name": "stl" }] } });
        localize_manifest(&mut no_schema, Locale::De);
        assert_eq!(no_schema["runtime"]["entries"][0], serde_json::json!({ "name": "stl" }));
        let mut unknown = serde_json::json!({ "runtime": { "entries": [{ "name": "obj", "input_schema": { "properties": { "mode": { "title": "Mode" } } } }] } });
        localize_manifest(&mut unknown, Locale::De);
        assert_eq!(unknown["runtime"]["entries"][0]["input_schema"]["properties"]["mode"]["title"], "Mode");
    }
}
