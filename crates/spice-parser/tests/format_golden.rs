//! Golden-file tests for the SPICE formatter.
//!
//! Each subdirectory of `tests/format/` contains `input.cir` and `expected.cir`.
//! The `wrap` case uses a narrower `max_line_width` (40).

use std::fs;
use std::path::PathBuf;

use spice_parser::{format_source, FormatOptions};

fn format_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/format")
}

fn options_for_case(name: &str) -> FormatOptions {
    if name == "wrap" {
        FormatOptions {
            max_line_width: 40,
            ..FormatOptions::default()
        }
    } else {
        FormatOptions::default()
    }
}

#[test]
fn golden_format_cases() {
    let root = format_root();
    let mut cases = fs::read_dir(&root)
        .expect("tests/format")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().ok().is_some_and(|t| t.is_dir()))
        .map(|e| e.path())
        .collect::<Vec<_>>();
    cases.sort();
    assert!(!cases.is_empty(), "expected at least one format case");

    for case_dir in cases {
        let name = case_dir.file_name().unwrap().to_string_lossy().to_string();
        let input = fs::read_to_string(case_dir.join("input.cir"))
            .unwrap_or_else(|e| panic!("{name}/input.cir: {e}"));
        let expected = fs::read_to_string(case_dir.join("expected.cir"))
            .unwrap_or_else(|e| panic!("{name}/expected.cir: {e}"));
        let opts = options_for_case(&name);
        let actual = format_source(&input, &opts);
        assert_eq!(
            actual, expected,
            "format mismatch for case '{name}'\n--- actual ---\n{actual}\n--- expected ---\n{expected}"
        );
        let again = format_source(&actual, &opts);
        assert_eq!(actual, again, "not idempotent for case '{name}'");
    }
}
