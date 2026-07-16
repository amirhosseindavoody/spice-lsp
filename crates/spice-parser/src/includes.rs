//! Cross-file resolution of `.include` / `.inc` and HSPICE `.lib` sections.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::analyze::{analyze_lines, collect_classified_lines};
use crate::diagnostic::{Diagnostic, Severity, Span};
use crate::dialect::Dialect;
use crate::symbols::{build_index, Index, LineKind, SymbolKind};

/// Default nesting limit for include / `.lib` chains.
pub const DEFAULT_MAX_INCLUDE_DEPTH: usize = 16;

/// A file load request extracted from the root (or nested) netlist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeRef {
    pub path: String,
    pub path_span: Span,
    /// When set, only the matching `.LIB entry` … `.ENDL` section is imported.
    pub lib_entry: Option<String>,
    pub entry_span: Option<Span>,
}

impl IncludeRef {
    pub fn from_line(kind: &LineKind) -> Option<Self> {
        match kind {
            LineKind::Include { path, path_span } => Some(Self {
                path: path.clone(),
                path_span: *path_span,
                lib_entry: None,
                entry_span: None,
            }),
            LineKind::LibCall {
                path,
                path_span,
                entry,
                entry_span,
            } => Some(Self {
                path: path.clone(),
                path_span: *path_span,
                lib_entry: Some(entry.clone()),
                entry_span: Some(*entry_span),
            }),
            _ => None,
        }
    }
}

/// Options for resolving include and library paths.
#[derive(Debug, Clone)]
pub struct ResolveOptions {
    /// Directory of the file that owns the include directives (usually the open document).
    pub base_dir: PathBuf,
    /// Extra search directories (`spiceLsp.libraryPaths`).
    pub library_paths: Vec<PathBuf>,
    pub max_depth: usize,
    pub dialect: Dialect,
}

impl Default for ResolveOptions {
    fn default() -> Self {
        Self {
            base_dir: PathBuf::from("."),
            library_paths: Vec::new(),
            max_depth: DEFAULT_MAX_INCLUDE_DEPTH,
            dialect: Dialect::default(),
        }
    }
}

/// One successfully loaded include / library contribution.
#[derive(Debug, Clone)]
pub struct IncludedFile {
    pub path: PathBuf,
    pub text: String,
    pub index: Index,
    /// Section filter applied, if this came from a `.lib` call.
    pub lib_entry: Option<String>,
}

/// A root-level `.include` / `.lib` directive that resolved to a file.
///
/// Used by go-to-definition when the cursor is on the path or library entry name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedInclude {
    pub path_span: Span,
    pub entry_span: Option<Span>,
    pub resolved_path: PathBuf,
    pub lib_entry: Option<String>,
}

/// Result of walking the include / `.lib` graph from a root buffer.
#[derive(Debug, Clone, Default)]
pub struct IncludeResolution {
    pub files: Vec<IncludedFile>,
    pub diagnostics: Vec<Diagnostic>,
    /// Successfully resolved include/lib calls from the root buffer (for navigation).
    pub root_includes: Vec<ResolvedInclude>,
}

impl IncludeResolution {
    pub fn find_definition(
        &self,
        kind: SymbolKind,
        name: &str,
    ) -> Option<(&IncludedFile, Span)> {
        for file in &self.files {
            if let Some(span) = file.index.definition_span(kind, name) {
                return Some((file, span));
            }
        }
        None
    }

    pub fn find_model_or_subckt(&self, name: &str) -> Option<(&IncludedFile, SymbolKind, Span)> {
        self.find_definition(SymbolKind::Model, name)
            .map(|(f, s)| (f, SymbolKind::Model, s))
            .or_else(|| {
                self.find_definition(SymbolKind::Subckt, name)
                    .map(|(f, s)| (f, SymbolKind::Subckt, s))
            })
    }

    pub fn defines_model_or_subckt(&self, name: &str) -> bool {
        self.find_model_or_subckt(name).is_some()
    }

    /// If `offset` is on a resolved include/lib path or entry, return that target location.
    ///
    /// - Cursor on the path → start of the included / library file
    /// - Cursor on a `.lib` entry name → the matching `.lib entry` section header
    pub fn definition_at_include_offset(&self, offset: usize) -> Option<(&IncludedFile, Span)> {
        for inc in &self.root_includes {
            let on_entry = inc
                .entry_span
                .is_some_and(|span| offset_in_span(offset, span));
            let on_path = offset_in_span(offset, inc.path_span);

            if !on_entry && !on_path {
                continue;
            }

            let file = self
                .files
                .iter()
                .find(|f| f.path == inc.resolved_path)?;

            if on_entry {
                if let Some(entry) = &inc.lib_entry {
                    let span = find_lib_section_span(&file.text, entry)
                        .unwrap_or(Span { start: 0, end: 0 });
                    return Some((file, span));
                }
            }

            return Some((file, Span { start: 0, end: 0 }));
        }
        None
    }
}

fn offset_in_span(offset: usize, span: Span) -> bool {
    // Match Index::symbol_at_offset: allow the cursor on either edge of the token.
    offset >= span.start && offset <= span.end
}

/// Collect include / lib-call directives from classified lines.
pub fn collect_include_refs(lines: &[(Span, LineKind)]) -> Vec<IncludeRef> {
    lines
        .iter()
        .filter_map(|(_, kind)| IncludeRef::from_line(kind))
        .collect()
}

/// Resolve `raw_path` against the including file directory and library search paths.
pub fn resolve_include_path(
    raw_path: &str,
    base_dir: &Path,
    library_paths: &[PathBuf],
) -> Option<PathBuf> {
    let path = PathBuf::from(raw_path);
    if path.is_absolute() {
        return if path.exists() {
            Some(canonicalize_best_effort(&path))
        } else {
            None
        };
    }

    let candidate = base_dir.join(&path);
    if candidate.exists() {
        return Some(canonicalize_best_effort(&candidate));
    }

    for lib in library_paths {
        let candidate = lib.join(&path);
        if candidate.exists() {
            return Some(canonicalize_best_effort(&candidate));
        }
    }

    None
}

fn canonicalize_best_effort(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// Load text for a path: prefer `open_buffers`, else read from disk.
pub type FileLoader<'a> = dyn Fn(&Path) -> Option<String> + 'a;

/// Walk includes reachable from `source`, using `loader` to obtain file text.
pub fn resolve_includes(
    source: &str,
    options: &ResolveOptions,
    loader: &FileLoader<'_>,
) -> IncludeResolution {
    let lines = collect_classified_lines(source);
    let refs = collect_include_refs(&lines);
    let mut resolution = IncludeResolution::default();
    let mut stack = HashSet::new();
    let root_key = options.base_dir.join("__root__");
    stack.insert(root_key);

    for include in refs {
        let before = resolution.files.len();
        resolve_one(
            &include,
            &options.base_dir,
            options,
            loader,
            &mut resolution,
            &mut stack,
            0,
        );
        if let Some(file) = resolution.files.get(before) {
            resolution.root_includes.push(ResolvedInclude {
                path_span: include.path_span,
                entry_span: include.entry_span,
                resolved_path: file.path.clone(),
                lib_entry: include.lib_entry.clone(),
            });
        }
    }

    resolution
}

fn resolve_one(
    include: &IncludeRef,
    base_dir: &Path,
    options: &ResolveOptions,
    loader: &FileLoader<'_>,
    resolution: &mut IncludeResolution,
    stack: &mut HashSet<PathBuf>,
    depth: usize,
) {
    if depth >= options.max_depth {
        resolution.diagnostics.push(Diagnostic {
            message: format!(
                "include depth exceeded (max {}) for '{}'",
                options.max_depth, include.path
            ),
            severity: Severity::Warning,
            span: include.path_span,
            code: Some("spice/include-depth".into()),
        });
        return;
    }

    let Some(path) = resolve_include_path(&include.path, base_dir, &options.library_paths) else {
        resolution.diagnostics.push(Diagnostic {
            message: format!("include file not found: '{}'", include.path),
            severity: Severity::Warning,
            span: include.path_span,
            code: Some("spice/include-not-found".into()),
        });
        return;
    };

    if !stack.insert(path.clone()) {
        resolution.diagnostics.push(Diagnostic {
            message: format!("include cycle involving '{}'", path.display()),
            severity: Severity::Warning,
            span: include.path_span,
            code: Some("spice/include-cycle".into()),
        });
        return;
    }

    let Some(text) = loader(&path) else {
        stack.remove(&path);
        resolution.diagnostics.push(Diagnostic {
            message: format!("include file not readable: '{}'", path.display()),
            severity: Severity::Warning,
            span: include.path_span,
            code: Some("spice/include-not-found".into()),
        });
        return;
    };

    let all_lines = collect_classified_lines(&text);
    let (section_lines, section_ok) = match &include.lib_entry {
        Some(entry) => {
            let filtered = lines_for_lib_section(&all_lines, entry);
            let ok = !filtered.is_empty()
                || all_lines.iter().any(|(_, k)| {
                    matches!(
                        k,
                        LineKind::LibSection { name, .. }
                            if name.eq_ignore_ascii_case(entry)
                    )
                });
            (filtered, ok)
        }
        None => (all_lines.clone(), true),
    };

    if let Some(entry) = &include.lib_entry {
        if !section_ok {
            resolution.diagnostics.push(Diagnostic {
                message: format!(
                    "library section '{entry}' not found in '{}'",
                    include.path
                ),
                severity: Severity::Warning,
                span: include.entry_span.unwrap_or(include.path_span),
                code: Some("spice/lib-section-not-found".into()),
            });
            stack.remove(&path);
            return;
        }
    }

    let (index, _) = build_index(&text, &section_lines);
    let nested_refs = collect_include_refs(&section_lines);
    let parent_dir = path.parent().unwrap_or(base_dir).to_path_buf();

    resolution.files.push(IncludedFile {
        path: path.clone(),
        text,
        index,
        lib_entry: include.lib_entry.clone(),
    });

    for nested in nested_refs {
        resolve_one(
            &nested,
            &parent_dir,
            options,
            loader,
            resolution,
            stack,
            depth + 1,
        );
    }

    stack.remove(&path);
}

/// Locate the `.lib entry` section header name span in a library file.
pub fn find_lib_section_span(source: &str, entry: &str) -> Option<Span> {
    for (_, kind) in collect_classified_lines(source) {
        if let LineKind::LibSection { name, name_span } = kind {
            if name.eq_ignore_ascii_case(entry) {
                return Some(name_span);
            }
        }
    }
    None
}

/// Keep classified lines that fall inside `.LIB entry` … `.ENDL` (case-insensitive).
pub fn lines_for_lib_section(
    lines: &[(Span, LineKind)],
    entry: &str,
) -> Vec<(Span, LineKind)> {
    let mut out = Vec::new();
    let mut in_section = false;

    for (span, kind) in lines {
        match kind {
            LineKind::LibSection { name, .. } if !in_section => {
                if name.eq_ignore_ascii_case(entry) {
                    in_section = true;
                }
            }
            LineKind::Endl { name } if in_section => {
                let matches = name
                    .as_ref()
                    .map(|n| n.eq_ignore_ascii_case(entry))
                    .unwrap_or(true);
                if matches {
                    in_section = false;
                } else {
                    out.push((*span, kind.clone()));
                }
            }
            _ if in_section => {
                out.push((*span, kind.clone()));
            }
            _ => {}
        }
    }

    out
}

/// Drop `spice/unknown-model` diagnostics that resolve via the include graph.
pub fn filter_unknown_models(
    diagnostics: Vec<Diagnostic>,
    local: &Index,
    resolution: &IncludeResolution,
) -> Vec<Diagnostic> {
    diagnostics
        .into_iter()
        .filter(|d| {
            if d.code.as_deref() != Some("spice/unknown-model") {
                return true;
            }
            let Some(name) = unknown_model_name(&d.message) else {
                return true;
            };
            if local.has_model_or_subckt(name) {
                return false;
            }
            !resolution.defines_model_or_subckt(name)
        })
        .collect()
}

fn unknown_model_name(message: &str) -> Option<&str> {
    // "'name' is not defined as a model or subcircuit"
    let start = message.find('\'')? + 1;
    let end = message[start..].find('\'')? + start;
    Some(&message[start..end])
}

/// Analyze `source` and merge include/lib resolution into diagnostics.
pub fn analyze_with_includes(
    source: &str,
    options: &ResolveOptions,
    loader: &FileLoader<'_>,
) -> (crate::ParseResult, IncludeResolution) {
    let lines = collect_classified_lines(source);
    let mut result = analyze_lines(source, options.dialect, &lines);
    let resolution = resolve_includes(source, options, loader);
    result.diagnostics =
        filter_unknown_models(std::mem::take(&mut result.diagnostics), &result.index, &resolution);
    result.diagnostics.extend(resolution.diagnostics.iter().cloned());
    result
        .diagnostics
        .sort_by_key(|d| (d.span.start, d.span.end));
    result
        .diagnostics
        .dedup_by(|a, b| a.span == b.span && a.message == b.message);
    (result, resolution)
}

/// Disk-backed loader with optional open-buffer overrides (path → text).
pub fn disk_loader_with_overrides(
    overrides: HashMap<PathBuf, String>,
) -> impl Fn(&Path) -> Option<String> {
    move |path: &Path| {
        let key = canonicalize_best_effort(path);
        if let Some(text) = overrides.get(&key).cloned() {
            return Some(text);
        }
        if let Some(text) = overrides.get(path).cloned() {
            return Some(text);
        }
        std::fs::read_to_string(path).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbols::classify_line;

    fn fixture_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-data/valid/with-include")
    }

    fn span_line(source: &str, line: &str) -> (Span, LineKind) {
        let start = source.find(line).expect("line");
        let span = Span {
            start,
            end: start + line.len(),
        };
        (span, classify_line(source, span))
    }

    #[test]
    fn classifies_include_and_lib_call() {
        let inc = ".include 'models.inc'\n";
        let (_, kind) = span_line(inc, ".include 'models.inc'");
        assert!(matches!(
            kind,
            LineKind::Include { ref path, .. } if path == "models.inc"
        ));

        let lib = ".lib 'corners.lib' TT\n";
        let (_, kind) = span_line(lib, ".lib 'corners.lib' TT");
        assert!(matches!(
            kind,
            LineKind::LibCall { ref path, ref entry, .. }
                if path == "corners.lib" && entry == "TT"
        ));

        let section = ".lib TT\n";
        let (_, kind) = span_line(section, ".lib TT");
        assert!(matches!(
            kind,
            LineKind::LibSection { ref name, .. } if name == "TT"
        ));
    }

    #[test]
    fn lib_section_filters_models() {
        let source = "\
.lib TT
.model nmos_tt nmos level=1
.endl TT
.lib FF
.model nmos_ff nmos level=1
.endl FF
";
        let lines = collect_classified_lines(source);
        let tt = lines_for_lib_section(&lines, "TT");
        let (index, _) = build_index(source, &tt);
        assert!(index.has_definition(SymbolKind::Model, "nmos_tt"));
        assert!(!index.has_definition(SymbolKind::Model, "nmos_ff"));
    }

    #[test]
    fn include_resolves_model_and_suppresses_unknown() {
        let dir = fixture_dir();
        let source = std::fs::read_to_string(dir.join("top.cir")).expect("top.cir");
        let options = ResolveOptions {
            base_dir: dir,
            library_paths: Vec::new(),
            max_depth: 8,
            dialect: Dialect::Hspice,
        };
        let loader = disk_loader_with_overrides(HashMap::new());
        let (result, resolution) = analyze_with_includes(&source, &options, &loader);

        assert!(
            resolution.defines_model_or_subckt("nch"),
            "expected nch from include"
        );
        assert!(
            resolution.defines_model_or_subckt("buffer"),
            "expected buffer from include"
        );
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|d| d.code.as_deref() == Some("spice/unknown-model")),
            "unexpected diagnostics: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn lib_call_resolves_selected_section_only() {
        let dir = fixture_dir();
        let source = std::fs::read_to_string(dir.join("top-lib.cir")).expect("top-lib.cir");
        let options = ResolveOptions {
            base_dir: dir,
            library_paths: Vec::new(),
            max_depth: 8,
            dialect: Dialect::Hspice,
        };
        let loader = disk_loader_with_overrides(HashMap::new());
        let (result, resolution) = analyze_with_includes(&source, &options, &loader);

        assert!(resolution.defines_model_or_subckt("nch_tt"));
        assert!(!resolution.defines_model_or_subckt("nch_ff"));
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|d| d.code.as_deref() == Some("spice/unknown-model")),
            "unexpected diagnostics: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn lib_path_and_entry_offer_goto_targets() {
        let dir = fixture_dir();
        let source = std::fs::read_to_string(dir.join("top-lib.cir")).expect("top-lib.cir");
        let options = ResolveOptions {
            base_dir: dir.clone(),
            library_paths: Vec::new(),
            max_depth: 8,
            dialect: Dialect::Hspice,
        };
        let loader = disk_loader_with_overrides(HashMap::new());
        let (_, resolution) = analyze_with_includes(&source, &options, &loader);

        assert_eq!(resolution.root_includes.len(), 1);

        let path_offset = source.find("corners.lib").expect("path");
        let (file, span) = resolution
            .definition_at_include_offset(path_offset)
            .expect("path target");
        assert!(file.path.ends_with("corners.lib"));
        assert_eq!(span, Span { start: 0, end: 0 });

        let entry_offset = source.find(" TT").expect("entry") + 1;
        let (file, span) = resolution
            .definition_at_include_offset(entry_offset)
            .expect("entry target");
        assert!(file.path.ends_with("corners.lib"));
        let section = find_lib_section_span(&file.text, "TT").expect("TT section");
        assert_eq!(span, section);
        assert_eq!(&file.text[span.start..span.end], "TT");
    }

    #[test]
    fn missing_include_emits_diagnostic() {
        let source = ".include 'missing.inc'\nX1 a b ghost\n";
        let options = ResolveOptions {
            base_dir: PathBuf::from("/tmp"),
            ..ResolveOptions::default()
        };
        let loader = disk_loader_with_overrides(HashMap::new());
        let (result, _) = analyze_with_includes(source, &options, &loader);
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("spice/include-not-found")));
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("spice/unknown-model")));
    }
}
