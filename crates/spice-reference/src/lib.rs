//! Curated SPICE dialect reference corpus for hover and completion.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;
use spice_parser::{Dialect, HoverKind, HoverToken};

#[derive(Debug, Clone, Deserialize)]
pub struct ReferenceParameter {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub units: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReferenceEntry {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub summary: String,
    pub syntax: String,
    #[serde(default)]
    pub parameters: Vec<ReferenceParameter>,
    #[serde(default)]
    pub examples: Vec<String>,
    #[serde(default, rename = "seeAlso")]
    pub see_also: Vec<String>,
    #[serde(default)]
    pub diagnostics: Vec<String>,
    #[serde(default)]
    pub since: Option<String>,
    #[serde(default)]
    pub deprecated: Option<String>,
    #[serde(default, rename = "dialectNotes")]
    pub dialect_notes: Option<String>,
}

impl ReferenceEntry {
    pub fn render_markdown(&self, dialect: Dialect) -> String {
        let mut out = String::new();
        out.push_str(&format!("### `{}` — {}\n\n", self.name, self.summary));
        out.push_str(&format!("**Dialect:** {}\n\n", dialect.label()));
        out.push_str("```\n");
        out.push_str(&self.syntax);
        out.push_str("\n```\n");

        if !self.parameters.is_empty() {
            out.push_str("\n| Parameter | Description | Units |\n");
            out.push_str("|-----------|-------------|-------|\n");
            for p in &self.parameters {
                let units = p.units.as_deref().unwrap_or("");
                out.push_str(&format!(
                    "| `{}` | {} | {} |\n",
                    p.name,
                    escape_cells(&p.description),
                    units
                ));
            }
        }

        if !self.examples.is_empty() {
            out.push_str("\n**Examples**\n");
            for ex in &self.examples {
                out.push_str(&format!("- `{}`\n", ex));
            }
        }

        if let Some(notes) = &self.dialect_notes {
            out.push_str(&format!("\n_{notes}_\n"));
        }

        out
    }
}

fn escape_cells(s: &str) -> String {
    s.replace('|', "\\|")
}

#[derive(Debug, Clone)]
struct EmbeddedRaw {
    dialect: &'static str,
    kind: &'static str,
    name: &'static str,
    json: &'static str,
}

fn embedded_raw() -> Vec<EmbeddedRaw> {
    include!(concat!(env!("OUT_DIR"), "/embedded_entries.rs"))
}

/// Index of reference entries with shared base + dialect overlays.
#[derive(Debug, Default)]
pub struct ReferenceIndex {
    /// (dialect_id or "shared", kind, normalized_name) -> entry
    entries: HashMap<(String, String, String), ReferenceEntry>,
}

impl ReferenceIndex {
    pub fn load_embedded() -> Self {
        let mut index = Self::default();
        for raw in embedded_raw() {
            let entry: ReferenceEntry = serde_json::from_str(raw.json).unwrap_or_else(|e| {
                panic!(
                    "invalid embedded entry {}/{}/{}: {e}",
                    raw.dialect, raw.kind, raw.name
                );
            });
            let key = (
                raw.dialect.to_string(),
                entry.kind.clone(),
                normalize_name(&entry.name),
            );
            index.entries.insert(key, entry);
        }
        index
    }

    pub fn global() -> &'static ReferenceIndex {
        static INDEX: OnceLock<ReferenceIndex> = OnceLock::new();
        INDEX.get_or_init(Self::load_embedded)
    }

    pub fn lookup(&self, dialect: Dialect, kind: HoverKind, name: &str) -> Option<&ReferenceEntry> {
        let kind_key = match kind {
            HoverKind::Directive => "directive",
            HoverKind::Element => "element",
        };
        let name_key = normalize_name(name);
        let dialect_key = dialect.id().to_string();

        self.entries
            .get(&(dialect_key, kind_key.to_string(), name_key.clone()))
            .or_else(|| {
                self.entries
                    .get(&("shared".to_string(), kind_key.to_string(), name_key))
            })
    }

    pub fn lookup_token(&self, dialect: Dialect, token: &HoverToken) -> Option<&ReferenceEntry> {
        self.lookup(dialect, token.kind, &token.name)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn normalize_name(name: &str) -> String {
    let trimmed = name.trim();
    if let Some(rest) = trimmed.strip_prefix('.') {
        format!(".{}", rest.to_ascii_lowercase())
    } else if trimmed.len() == 1 {
        trimmed.to_ascii_uppercase()
    } else {
        trimmed.to_ascii_lowercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spice_parser::HoverKind;

    #[test]
    fn embedded_corpus_loads() {
        let index = ReferenceIndex::load_embedded();
        assert!(!index.is_empty());
    }

    #[test]
    fn hspice_tran_overrides_shared() {
        let index = ReferenceIndex::load_embedded();
        let entry = index
            .lookup(Dialect::Hspice, HoverKind::Directive, ".tran")
            .expect("hspice .tran");
        assert!(entry.summary.contains("HSPICE"));
        assert_eq!(entry.id, "hspice.directive.tran");
    }

    #[test]
    fn ngspice_falls_back_to_shared_subckt() {
        let index = ReferenceIndex::load_embedded();
        let entry = index
            .lookup(Dialect::Ngspice, HoverKind::Directive, ".subckt")
            .expect("shared .subckt");
        assert_eq!(entry.id, "shared.directive.subckt");
    }

    #[test]
    fn element_r_from_shared() {
        let index = ReferenceIndex::load_embedded();
        let entry = index
            .lookup(Dialect::Hspice, HoverKind::Element, "R")
            .expect("R");
        assert_eq!(entry.name, "R");
    }
}
