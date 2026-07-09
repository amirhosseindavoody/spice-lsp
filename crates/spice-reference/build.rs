use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let reference_root = manifest_dir.join("../../reference");
    println!("cargo:rerun-if-changed={}", reference_root.display());

    let mut entries = Vec::new();
    collect_json(&reference_root.join("_shared"), "shared", &mut entries);
    for dialect in ["hspice", "ngspice", "ltspice"] {
        collect_json(&reference_root.join(dialect), dialect, &mut entries);
    }

    entries.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let dest = out_dir.join("embedded_entries.rs");
    let mut code = String::from("vec![\n");
    for (dialect, kind, name, json) in &entries {
        let escaped = escape_raw(json);
        code.push_str(&format!(
            "    EmbeddedRaw {{ dialect: \"{dialect}\", kind: \"{kind}\", name: \"{name}\", json: r###\"{escaped}\"### }},\n"
        ));
    }
    code.push_str("]\n");
    fs::write(dest, code).expect("write embedded_entries.rs");
}

fn collect_json(dir: &Path, dialect: &str, out: &mut Vec<(String, String, String, String)>) {
    if !dir.is_dir() {
        return;
    }
    visit(dir, dialect, out);
}

fn visit(dir: &Path, dialect: &str, out: &mut Vec<(String, String, String, String)>) {
    let Ok(read) = fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            visit(&path, dialect, out);
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if path.file_name().and_then(|n| n.to_str()) == Some("schema.json") {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!("failed to read {}: {e}", path.display());
        });
        let value: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|e| {
            panic!("invalid JSON {}: {e}", path.display());
        });
        let kind = value
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let name = value
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let id = value
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if kind.is_empty() || name.is_empty() || id.is_empty() {
            panic!("{} missing required id/kind/name", path.display());
        }
        for field in ["summary", "syntax"] {
            if value
                .get(field)
                .and_then(|v| v.as_str())
                .map(|s| s.is_empty())
                .unwrap_or(true)
            {
                panic!("{} missing required {field}", path.display());
            }
        }
        out.push((dialect.to_string(), kind, name, text));
    }
}

fn escape_raw(s: &str) -> String {
    // Avoid terminating the r###"..."### delimiter if present in JSON (unlikely).
    s.replace("###", "##\\#")
}
