//! Analysis profiles for schematic-scale vs large extracted netlists.

/// How the server chooses between full and extracted analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnalysisMode {
    /// Use [`AnalysisProfile::Extracted`] when the buffer reaches the size threshold.
    #[default]
    Auto,
    /// Always index instances, outline children, and duplicate-name checks.
    Full,
    /// Definitions-only indexing (see [`AnalysisProfile::Extracted`]).
    Extracted,
}

impl AnalysisMode {
    /// Parse a settings string (`auto` / `full` / `extracted`). Unknown → `Auto`.
    pub fn parse_or_default(raw: &str) -> (Self, bool) {
        match raw.trim().to_ascii_lowercase().as_str() {
            "auto" => (Self::Auto, true),
            "full" => (Self::Full, true),
            "extracted" => (Self::Extracted, true),
            _ => (Self::Auto, false),
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Full => "full",
            Self::Extracted => "extracted",
        }
    }
}

/// Concrete analysis strategy applied to one buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnalysisProfile {
    /// Full instance symbols, outline children, and duplicate-name diagnostics.
    #[default]
    Full,
    /// Index `.subckt` / `.model` / `.param` only; sparse model refs for unknown-model.
    Extracted,
}

impl AnalysisProfile {
    pub fn id(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Extracted => "extracted",
        }
    }
}

/// Default size gate for [`AnalysisMode::Auto`] (16 MiB).
pub const DEFAULT_EXTRACTED_BYTE_THRESHOLD: usize = 16 * 1024 * 1024;

/// Resolve the profile for a buffer of `source_len` bytes.
pub fn resolve_profile(
    mode: AnalysisMode,
    source_len: usize,
    threshold: usize,
) -> AnalysisProfile {
    match mode {
        AnalysisMode::Full => AnalysisProfile::Full,
        AnalysisMode::Extracted => AnalysisProfile::Extracted,
        AnalysisMode::Auto => {
            if source_len >= threshold {
                AnalysisProfile::Extracted
            } else {
                AnalysisProfile::Full
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_respects_threshold() {
        assert_eq!(
            resolve_profile(AnalysisMode::Auto, 100, 1000),
            AnalysisProfile::Full
        );
        assert_eq!(
            resolve_profile(AnalysisMode::Auto, 1000, 1000),
            AnalysisProfile::Extracted
        );
    }

    #[test]
    fn parse_mode() {
        assert_eq!(AnalysisMode::parse_or_default("extracted").0, AnalysisMode::Extracted);
        assert_eq!(AnalysisMode::parse_or_default("nope").0, AnalysisMode::Auto);
    }
}
