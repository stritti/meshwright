//! The plugin package (`plugin/revmesh/`), as far as `cargo test` can check it natively: the
//! manifest's `x-solidcraft-icon` annotations must name an SVG inside the package that passes
//! SolidCraft's icon subset (the check `solidcraft-cli plugin test` runs on the real host), so a
//! broken icon fails here first.

use std::path::Path;

const PACKAGE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../plugin/revmesh");

/// SolidCraft's icon palette source colours (crates/look/icons/design/palette.toml); `currentColor`
/// is allowed on top of these.
const PALETTE: &[&str] = &[
    "#34465b", "#e2e8ef", "#b8c3cf", "#8e9faf", "#ffffff", "#6f879c", "#b9ddf8", "#328bd3", "#236ead", "#f1d2af", "#c06f00", "#a45000",
    "#e5d99b", "#a28200", "#826700",
];

fn manifest() -> serde_json::Value {
    let path = format!("{PACKAGE_DIR}/plugin.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    serde_json::from_str(&text).expect("plugin.json parses")
}

/// Every `x-solidcraft-icon` value anywhere in `value` (an entry's input schema names the entry's
/// icon at its top and pictures on `oneOf` options).
fn icon_paths(value: &serde_json::Value, found: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, v) in map {
                if key == "x-solidcraft-icon" {
                    found.push(v.as_str().expect("x-solidcraft-icon is a path string").to_owned());
                }
                icon_paths(v, found);
            }
        }
        serde_json::Value::Array(items) => items.iter().for_each(|v| icon_paths(v, found)),
        _ => {}
    }
}

/// The attribute values of `attr` (for example ` fill="`) in `svg`.
fn attribute_values<'a>(svg: &'a str, attr: &str) -> impl Iterator<Item = &'a str> {
    svg.match_indices(attr).map(move |(i, _)| {
        let rest = &svg[i + attr.len()..];
        rest.split('"').next().unwrap_or_default()
    })
}

#[test]
fn every_manifest_icon_names_an_svg_inside_the_package() {
    let manifest = manifest();
    let mut paths = Vec::new();
    icon_paths(&manifest["runtime"]["entries"], &mut paths);
    assert!(!paths.is_empty(), "the manifest names at least one x-solidcraft-icon");
    for path in paths {
        assert!(Path::new(&path).extension().is_some_and(|ext| ext.eq_ignore_ascii_case("svg")), "{path}: not an .svg");
        assert!(
            !path.starts_with('/') && !path.contains('\\') && !path.split('/').any(|component| component.is_empty() || component == ".."),
            "{path}: not a plain relative path inside the package"
        );
        let file = format!("{PACKAGE_DIR}/{path}");
        assert!(Path::new(&file).is_file(), "{file}: missing");
        let bytes = std::fs::read(&file).unwrap_or_else(|e| panic!("{file}: {e}"));
        assert!(bytes.len() <= 64 * 1024, "{path}: over 64 KiB ({} bytes)", bytes.len());
        let svg = String::from_utf8(bytes).expect("the icon is UTF-8 SVG");
        check_icon_subset(&path, &svg);
    }
}

/// The subset SolidCraft's icon parser accepts (the lint its own icons pass,
/// `crates/look/icons/design/tools/lint_icons.py`): viewBox "0 0 64 64", no width/height on
/// `<svg>`, no foreign elements or transform/style/class, absolute M L C Q A Z path data,
/// palette colours or currentColor, and strokes with a width and round caps and joins.
fn check_icon_subset(path: &str, svg: &str) {
    let svg_tag = svg.split('>').next().unwrap_or_default();
    assert!(svg_tag.contains("viewBox=\"0 0 64 64\""), "{path}: viewBox must be \"0 0 64 64\"");
    assert!(!svg_tag.contains("width=") && !svg_tag.contains("height="), "{path}: no width/height on <svg>");
    for forbidden in ["<image", "<use", "<script", "<style", "<text", "transform=", "style=", "class="] {
        assert!(!svg.contains(forbidden), "{path}: {forbidden} is not allowed");
    }
    for attr in [" fill=\"", " stroke=\""] {
        for value in attribute_values(svg, attr) {
            let v = value.to_lowercase();
            assert!(
                v == "none" || v == "currentcolor" || v.starts_with("url(") || PALETTE.contains(&v.as_str()),
                "{path}: {attr}{value}\" is not a palette colour"
            );
        }
    }
    for d in attribute_values(svg, " d=\"") {
        assert!(
            !d.is_empty() && d.chars().all(|c| "MLCQAZ0123456789 .-".contains(c)),
            "{path}: path data may use only absolute M L C Q A Z: {d:?}"
        );
    }
    for element in svg.split('<').skip(1) {
        let element = element.split('>').next().unwrap_or_default();
        if element.starts_with("path ") || element.starts_with("circle ") || element == "path" || element == "circle" {
            let stroked = element.contains(" stroke=\"") && !element.contains(" stroke=\"none\"");
            if stroked {
                assert!(element.contains(" stroke-width="), "{path}: stroke without stroke-width in <{element}");
                assert!(
                    element.contains(" stroke-linejoin=\"round\"") && element.contains(" stroke-linecap=\"round\""),
                    "{path}: strokes need round joins and caps in <{element}"
                );
            }
            assert!(element.contains(" fill=") || element.contains(" stroke="), "{path}: <{element} without paint");
        }
    }
}
