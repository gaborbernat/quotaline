//! Regenerates the desktop copy modules from the shared catalog and fails when a checked-in module was stale.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use super::*;

fn repository_file(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(relative)
}

fn quoted(text: &str, quote: char) -> String {
    let escaped = text.replace('\\', "\\\\").replace(quote, &format!("\\{quote}"));
    format!("{quote}{escaped}{quote}")
}

fn javascript() -> String {
    let entries = ENTRIES
        .iter()
        .map(|(key, text)| format!("    {key}: {},\n", quoted(text, '\'')))
        .collect::<String>();
    format!(
        "// Generated from crates/presentation/text/src/ui_copy.rs by its unit test; do not edit.\n\nexport const COPY = Object.freeze({{\n{entries}}});\n"
    )
}

fn swift() -> String {
    let entries = ENTRIES
        .iter()
        .map(|(key, text)| {
            let line = format!("  public static let {key} = {}\n", quoted(text, '"'));
            // swift-format breaks after `=` once a declaration passes its 120-column limit.
            if line.chars().count() > 121 {
                format!("  public static let {key} =\n    {}\n", quoted(text, '"'))
            } else {
                line
            }
        })
        .collect::<String>();
    format!(
        "// Generated from crates/presentation/text/src/ui_copy.rs by its unit test; do not edit.\n\npublic enum SharedCopy {{\n{entries}}}\n"
    )
}

#[test]
fn keys_are_unique_and_sorted() {
    let keys = ENTRIES.iter().map(|(key, _)| *key).collect::<Vec<_>>();
    let unique = keys.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(keys, unique.into_iter().collect::<Vec<_>>());
}

#[test]
fn generated_modules_match_the_catalog() {
    let stale = [
        ("gnome/extension/src/shared-copy.ts", javascript()),
        ("macos/src/QuotalineCore/SharedCopy.swift", swift()),
    ]
    .into_iter()
    .filter(|(relative, source)| {
        let path = repository_file(relative);
        let current = fs::read_to_string(&path).unwrap_or_default();
        fs::write(&path, source).unwrap();
        current != *source
    })
    .map(|(relative, _)| relative)
    .collect::<Vec<_>>();
    assert!(stale.is_empty(), "regenerated stale copy modules: {stale:?}");
}
