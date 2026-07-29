macro_rules! declare_lint_stub {
    (
        $(#[$attribute:meta])*
        $visibility:vis $name:ident {
            id: $id:literal,
            summary: $summary:literal,
            explanation: $explanation:literal,
            example: {
                reported: $reported:literal,
                accepted: $accepted:literal,
            },
            category: $category:ident,
            level: $level:ident,
            fixable: $fixable:ident,
            check: $check:ident($function:path),
        }
    ) => {
        const _: destack_source::DiagnosticDefinition =
            destack_source::DiagnosticDefinition::controllable_warning($id, $summary);

        $(#[$attribute])*
        $visibility static $name: $crate::Lint = $crate::Lint {
            id: std::borrow::Cow::Borrowed($id),
            summary: std::borrow::Cow::Borrowed($summary),
            explanation: std::borrow::Cow::Borrowed($explanation),
            example: $crate::LintExample {
                reported: std::borrow::Cow::Borrowed($reported),
                accepted: std::borrow::Cow::Borrowed($accepted),
            },
            category: $crate::LintCategory::$category,
            default_level: destack_repository::LintLevel::$level,
            fixability: $crate::Fixability::$fixable,
            check: $crate::LintCheck::$check($function),
        };
    };
    (
        $(#[$attribute:meta])*
        $visibility:vis $name:ident {
            id: $id:literal,
            summary: $summary:literal,
            category: $category:ident,
            level: $level:ident,
            fixable: $fixable:ident,
            check: $check:ident($function:path),
        }
    ) => {
        $crate::rules::declare_lint_stub! {
            $(#[$attribute])*
            $visibility $name {
                id: $id,
                summary: $summary,
                explanation: "",
                example: {
                    reported: "",
                    accepted: "",
                },
                category: $category,
                level: $level,
                fixable: $fixable,
                check: $check($function),
            }
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
                reported: $reported:literal,
                accepted: $accepted:literal,
            },
            category: $category:ident,
            level: $level:ident,
            fixable: $fixable:ident,
            check: $check:ident($function:path),
        }
    ) => {
        $crate::rules::declare_lint_stub! {
            $(#[$attribute])*
            $visibility $name {
                id: $id,
                summary: $summary,
                explanation: $explanation,
                example: {
                    reported: $reported,
                    accepted: $accepted,
                },
                category: $category,
                level: $level,
                fixable: $fixable,
                check: $check($function),
            }
        }

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
pub(crate) use declare_lint_stub;
