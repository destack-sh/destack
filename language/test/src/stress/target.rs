use std::time::Duration;

use super::{StressFixture, materialize_formatter_fixtures, materialize_parser_fixtures};

/// One generated stress target.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(super) enum StressTarget {
    /// Parser stress target.
    Parser,
    /// Formatter stress target.
    Formatter,
}

impl StressTarget {
    /// Return the suite name.
    pub(super) const fn suite_name(self) -> &'static str {
        match self {
            Self::Parser => "stress-parser",
            Self::Formatter => "stress-formatter",
        }
    }

    /// Return the hidden worker command flag.
    pub(super) const fn worker_flag(self) -> &'static str {
        match self {
            Self::Parser => "--run-parser-case",
            Self::Formatter => "--run-formatter-case",
        }
    }

    /// Return the generated fixture category.
    pub(super) const fn category(self) -> &'static str {
        match self {
            Self::Parser => StressFixture::parser_category(),
            Self::Formatter => StressFixture::formatter_category(),
        }
    }

    /// Return the worker timeout.
    pub(super) const fn worker_timeout(self) -> Duration {
        match self {
            Self::Parser => Duration::from_secs(30),
            Self::Formatter => Duration::from_secs(60),
        }
    }

    /// Return the harness timeout.
    pub(super) const fn harness_timeout(self) -> Duration {
        match self {
            Self::Parser => Duration::from_secs(35),
            Self::Formatter => Duration::from_secs(65),
        }
    }

    /// Materialize target stress fixtures.
    pub(super) fn materialize_fixtures(self) -> Result<Vec<StressFixture>, String> {
        match self {
            Self::Parser => materialize_parser_fixtures(),
            Self::Formatter => materialize_formatter_fixtures(),
        }
    }
}
