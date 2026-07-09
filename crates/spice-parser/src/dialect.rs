//! SPICE dialect identifiers and syntax/semantics profiles.

use std::fmt;
use std::str::FromStr;

/// Supported SPICE dialects (issue #16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Dialect {
    #[default]
    Hspice,
    Ngspice,
    Ltspice,
}

impl Dialect {
    pub const ALL: [Dialect; 3] = [Dialect::Hspice, Dialect::Ngspice, Dialect::Ltspice];

    pub fn id(self) -> &'static str {
        match self {
            Dialect::Hspice => "hspice",
            Dialect::Ngspice => "ngspice",
            Dialect::Ltspice => "ltspice",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Dialect::Hspice => "HSPICE",
            Dialect::Ngspice => "Ngspice",
            Dialect::Ltspice => "LTspice",
        }
    }

    pub fn profile(self) -> DialectProfile {
        DialectProfile::for_dialect(self)
    }

    /// Parse a dialect id; unknown values fall back to HSPICE.
    pub fn parse_or_default(value: &str) -> (Dialect, bool) {
        match Dialect::from_str(value) {
            Ok(d) => (d, false),
            Err(_) => (Dialect::Hspice, true),
        }
    }
}

impl fmt::Display for Dialect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

impl FromStr for Dialect {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "hspice" => Ok(Dialect::Hspice),
            "ngspice" => Ok(Dialect::Ngspice),
            "ltspice" => Ok(Dialect::Ltspice),
            _ => Err(()),
        }
    }
}

/// Syntax / semantics knobs that differ by dialect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DialectProfile {
    pub dialect: Dialect,
    /// Line-comment prefixes recognized for documentation / future toggle.
    pub line_comment_prefixes: &'static [&'static str],
}

impl DialectProfile {
    pub fn for_dialect(dialect: Dialect) -> Self {
        match dialect {
            Dialect::Hspice => Self {
                dialect,
                line_comment_prefixes: &["*"],
            },
            Dialect::Ngspice => Self {
                dialect,
                line_comment_prefixes: &["*", ";", "$"],
            },
            Dialect::Ltspice => Self {
                dialect,
                line_comment_prefixes: &["*", "$"],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_hspice() {
        assert_eq!(Dialect::default(), Dialect::Hspice);
    }

    #[test]
    fn parse_known_and_fallback() {
        assert_eq!(Dialect::from_str("NGSPICE"), Ok(Dialect::Ngspice));
        let (d, fell_back) = Dialect::parse_or_default("nope");
        assert_eq!(d, Dialect::Hspice);
        assert!(fell_back);
    }
}
