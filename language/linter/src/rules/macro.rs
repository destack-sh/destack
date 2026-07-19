macro_rules! declare_lint {
    (
        $(#[$attribute:meta])*
        $visibility:vis $name:ident {
            id: $id:literal,
            description: $description:literal,
            category: $category:ident,
            level: $level:ident,
            fixable: $fixable:ident,
            check: $check:ident($function:path),
        }
    ) => {
        const _: destack_source::DiagnosticDefinition =
            destack_source::DiagnosticDefinition::controllable_warning($id, $description);

        $(#[$attribute])*
        $visibility static $name: $crate::Lint = $crate::Lint {
            id: std::borrow::Cow::Borrowed($id),
            description: std::borrow::Cow::Borrowed($description),
            category: $crate::LintCategory::$category,
            default_level: destack_repository::LintLevel::$level,
            fixability: $crate::Fixability::$fixable,
            check: $crate::LintCheck::$check($function),
        };
    };
}

pub(crate) use declare_lint;
