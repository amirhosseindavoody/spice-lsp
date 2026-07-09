//! Generate or check mdBook dialect catalog pages from the embedded corpus.

use std::env;
use std::path::PathBuf;
use std::process;

use spice_reference::{generate_catalog_files, CatalogMode};

fn main() {
    let mut args = env::args().skip(1);
    let mode = match args.next().as_deref() {
        Some("write") | None => CatalogMode::Write,
        Some("check") => CatalogMode::Check,
        Some(other) => {
            eprintln!("usage: spice-reference-catalog [write|check] [repo-root]");
            eprintln!("unknown mode: {other}");
            process::exit(2);
        }
    };

    let repo_root = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    match generate_catalog_files(&repo_root, mode) {
        Ok(()) => {}
        Err(err) => {
            eprintln!("spice-reference-catalog: {err}");
            process::exit(1);
        }
    }
}
