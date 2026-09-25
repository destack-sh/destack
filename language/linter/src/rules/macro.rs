macro_rules! lint_example_source {
    ($source:literal) => {
        $crate::LintExampleSource {
            path: std::borrow::Cow::Borrowed("main.tspp"),
            source: std::borrow::Cow::Borrowed($source),
        }
    };
    (($path:literal, $source:literal)) => {
        $crate::LintExampleSource {
            path: std::borrow::Cow::Borrowed($path),
            source: std::borrow::Cow::Borrowed($source),
        }
    };
}

macro_rules! declare_lint {
    (
        $(#[$attribute:meta])*
        $visibility:vis $name:ident {
            id: $id:literal,
            summary: $summary:literal,
            explanation: $explanation:literal,
            example: {
                reported: $reported:tt,
                accepted: $accepted:tt,
            },
            provenance: [
                $($source:ident($rule:literal)),* $(,)?
            ],
            category: $category:ident,
            level: $level:ident,
            fixable: $fixable:ident,
            $(indexes: [$($index:ident),* $(,)?],)?
            check: $check:ident($function:path),
        }
    ) => {
        const _: tspp_source::DiagnosticDefinition =
            tspp_source::DiagnosticDefinition::controllable_warning($id, $summary);

        $(#[$attribute])*
        $visibility static $name: $crate::Lint = $crate::Lint {
            id: std::borrow::Cow::Borrowed($id),
            summary: std::borrow::Cow::Borrowed($summary),
            explanation: std::borrow::Cow::Borrowed($explanation.trim_ascii()),
            example: $crate::LintExample {
                reported: $crate::rules::lint_example_source!($reported),
                accepted: $crate::rules::lint_example_source!($accepted),
            },
            provenance: &[
                $(
                    $crate::LintProvenance {
                        source: $crate::LintSource::$source,
                        rule: $rule,
                    }
                ),*
            ],
            category: $crate::LintCategory::$category,
            default_level: tspp_repository::LintLevel::$level,
            fixability: $crate::Fixability::$fixable,
            module_indexes: &[$($(tspp_artifact::IndexKind::$index),*)?],
            check: $crate::LintCheck::$check($function),
            source_path: file!(),
            source_line: line!(),
        };

        #[cfg(test)]
        use $crate::tests::TestSession;

        /// Validate the canonical lint example.
        #[cfg(test)]
        #[test]
        fn test_lint_example() {
            TestSession::assert_example(&$name);
        }
    };
}

pub(crate) use declare_lint;
pub(crate) use lint_example_source;
