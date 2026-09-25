use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

const READ: i64 = 0x0001;
const WRITE: i64 = 0x0002;
const CREATE: i64 = 0x0004;
const EXCLUSIVE: i64 = 0x0008;
const TRUNCATE: i64 = 0x0010;
const APPEND: i64 = 0x0020;

declare_lint! {
    /// Disallow contradictory or ineffective file open options.
    pub NONSENSICAL_OPEN_OPTIONS {
        id: "nonsensical-open-options",
        summary: "Disallow contradictory or ineffective file open options",
        explanation: r#"
Contradictory file open flags cannot express a coherent operation, while dependent flags without their prerequisite have no useful effect.
Instead, you SHOULD select at least one access mode and include every prerequisite flag.
"#,
        example: {
            reported: r#"
import {
    FileMode,
    FileOpenFlags,
    FileOpenOptions,
    FileResolveFlags,
} from "tspp:fs/binding";

const OPTIONS = FileOpenOptions {
    flags: FileOpenFlags(0x0011),
    mode: FileMode(0),
    resolve: FileResolveFlags(0),
};
"#,
            accepted: r#"
import {
    FileMode,
    FileOpenFlags,
    FileOpenOptions,
    FileResolveFlags,
} from "tspp:fs/binding";

const OPTIONS = FileOpenOptions {
    flags: FileOpenFlags(0x0012),
    mode: FileMode(0),
    resolve: FileResolveFlags(0),
};
"#,
        },
        provenance: [Clippy("nonsensical_open_options")],
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// One contradictory or ineffective FileOpenOptions selection.
#[derive(Debug, Clone, Copy)]
enum OpenOptionConflict {
    /// No access mode.
    MissingAccess,
    /// Exclusive creation without creation.
    ExclusiveWithoutCreate,
    /// Truncation without write access.
    TruncateWithoutWrite,
    /// Append without write access.
    AppendWithoutWrite,
    /// Simultaneous append and truncation.
    AppendWithTruncate,
    /// A creation mode without creation.
    ModeWithoutCreate,
}

impl OpenOptionConflict {
    /// Iterate over invalid relationships among exact open flags and mode bits.
    fn detect(flags: i64, mode: i64) -> impl Iterator<Item = Self> {
        let has = |flag| flags & flag != 0;

        [
            (!has(READ) && !has(WRITE)).then_some(Self::MissingAccess),
            (has(EXCLUSIVE) && !has(CREATE)).then_some(Self::ExclusiveWithoutCreate),
            (has(TRUNCATE) && !has(WRITE)).then_some(Self::TruncateWithoutWrite),
            (has(APPEND) && !has(WRITE)).then_some(Self::AppendWithoutWrite),
            (mode != 0 && !has(CREATE)).then_some(Self::ModeWithoutCreate),
            (has(APPEND) && has(TRUNCATE)).then_some(Self::AppendWithTruncate),
        ]
        .into_iter()
        .flatten()
    }

    /// Return the option value that causes this conflict.
    fn expression(
        self,
        flags: dir::LocalNodeId<dir::Expression>,
        mode: dir::LocalNodeId<dir::Expression>,
    ) -> dir::LocalNodeId<dir::Expression> {
        match self {
            Self::ModeWithoutCreate => mode,
            Self::MissingAccess
            | Self::ExclusiveWithoutCreate
            | Self::TruncateWithoutWrite
            | Self::AppendWithoutWrite
            | Self::AppendWithTruncate => flags,
        }
    }

    /// Return the diagnostic message.
    fn message(self) -> &'static str {
        match self {
            Self::MissingAccess => "open options select no access mode",
            Self::ExclusiveWithoutCreate => "exclusive open has no create flag",
            Self::TruncateWithoutWrite => "truncate open has no write flag",
            Self::AppendWithoutWrite => "append open has no write flag",
            Self::AppendWithTruncate => "open options request append and truncate together",
            Self::ModeWithoutCreate => "open mode has no create flag",
        }
    }

    /// Return the correction guidance.
    fn help(self) -> &'static str {
        match self {
            Self::MissingAccess => "add the read or write flag",
            Self::ExclusiveWithoutCreate => "add the create flag or remove exclusive",
            Self::TruncateWithoutWrite => "add the write flag or remove truncate",
            Self::AppendWithoutWrite => "add the write flag or remove append",
            Self::AppendWithTruncate => "choose append or truncate",
            Self::ModeWithoutCreate => "add the create flag or use a zero mode",
        }
    }
}

/// Report contradictory or ineffective FileOpenOptions values.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect statically written FileOpenOptions objects
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::StructExpression { properties, .. } = node else {
            continue;
        };
        if module.representation_item(expression.into_any())?
            != Some(dir::LanguageItem::FileOpenOptions)
        {
            continue;
        }

        // read the flag and mode fields
        let flags = dir::StaticKey::Name(dir::StringId::for_text("flags"));
        let Some((_, flags_expression)) = module.direct_field(properties, flags) else {
            continue;
        };
        let mode = dir::StaticKey::Name(dir::StringId::for_text("mode"));
        let Some((_, mode_expression)) = module.direct_field(properties, mode) else {
            continue;
        };

        // evaluate exact newtype values
        let Some((flags, _)) =
            module.newtype_integral(flags_expression, dir::LanguageItem::FileOpenFlags)?
        else {
            continue;
        };
        let Some((mode, _)) =
            module.newtype_integral(mode_expression, dir::LanguageItem::FileMode)?
        else {
            continue;
        };

        // report every independent invalid selection
        for conflict in OpenOptionConflict::detect(flags, mode) {
            let expression = conflict.expression(flags_expression, mode_expression);
            let span = module.source_extent(expression.into_any())?;
            let diagnostic = lint
                .diagnostic(conflict.message(), span)
                .help(conflict.help());
            output.report(diagnostic);
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report truncation without write access.
    #[test]
    fn test_reports_truncate_without_write() {
        TestSession::assert_example(&NONSENSICAL_OPEN_OPTIONS);
    }

    /// Report every independent contradiction in one options value.
    #[test]
    fn test_reports_multiple_conflicts() {
        let session = TestSession::dir(
            &NONSENSICAL_OPEN_OPTIONS,
            r#"
import {
    FileMode,
    FileOpenFlags,
    FileOpenOptions,
    FileResolveFlags,
} from "tspp:fs/binding";

const OPTIONS = FileOpenOptions {
    flags: FileOpenFlags(0x0038),
    mode: FileMode(0o600),
    resolve: FileResolveFlags(0),
};
"#,
        );

        session.assert_diagnostics(
            r#"warning[nonsensical-open-options]: open options select no access mode
  ──▶ main.tspp:9:12
   │
 7 │
 8 │ const OPTIONS = FileOpenOptions {
 9 │     flags: FileOpenFlags(0x0038),
   │            ^^^^^^^^^^^^^^^^^^^^^
10 │     mode: FileMode(0o600),
11 │     resolve: FileResolveFlags(0),
   │

 = help: add the read or write flag
warning[nonsensical-open-options]: exclusive open has no create flag
  ──▶ main.tspp:9:12
   │
 7 │
 8 │ const OPTIONS = FileOpenOptions {
 9 │     flags: FileOpenFlags(0x0038),
   │            ^^^^^^^^^^^^^^^^^^^^^
10 │     mode: FileMode(0o600),
11 │     resolve: FileResolveFlags(0),
   │

 = help: add the create flag or remove exclusive
warning[nonsensical-open-options]: truncate open has no write flag
  ──▶ main.tspp:9:12
   │
 7 │
 8 │ const OPTIONS = FileOpenOptions {
 9 │     flags: FileOpenFlags(0x0038),
   │            ^^^^^^^^^^^^^^^^^^^^^
10 │     mode: FileMode(0o600),
11 │     resolve: FileResolveFlags(0),
   │

 = help: add the write flag or remove truncate
warning[nonsensical-open-options]: append open has no write flag
  ──▶ main.tspp:9:12
   │
 7 │
 8 │ const OPTIONS = FileOpenOptions {
 9 │     flags: FileOpenFlags(0x0038),
   │            ^^^^^^^^^^^^^^^^^^^^^
10 │     mode: FileMode(0o600),
11 │     resolve: FileResolveFlags(0),
   │

 = help: add the write flag or remove append
warning[nonsensical-open-options]: open mode has no create flag
  ──▶ main.tspp:10:11
   │
 8 │ const OPTIONS = FileOpenOptions {
 9 │     flags: FileOpenFlags(0x0038),
10 │     mode: FileMode(0o600),
   │           ^^^^^^^^^^^^^^^
11 │     resolve: FileResolveFlags(0),
12 │ };
   │

 = help: add the create flag or use a zero mode
warning[nonsensical-open-options]: open options request append and truncate together
  ──▶ main.tspp:9:12
   │
 7 │
 8 │ const OPTIONS = FileOpenOptions {
 9 │     flags: FileOpenFlags(0x0038),
   │            ^^^^^^^^^^^^^^^^^^^^^
10 │     mode: FileMode(0o600),
11 │     resolve: FileResolveFlags(0),
   │

 = help: choose append or truncate
"#,
        );
    }

    /// Accept coherent truncating write options.
    #[test]
    fn test_accepts_coherent_options() {
        let session = TestSession::dir(
            &NONSENSICAL_OPEN_OPTIONS,
            NONSENSICAL_OPEN_OPTIONS.example.accepted.source(),
        );

        session.assert_no_diagnostics();
    }

    /// Accept options whose runtime flags remain unknown.
    #[test]
    fn test_accepts_runtime_flags() {
        let session = TestSession::dir(
            &NONSENSICAL_OPEN_OPTIONS,
            r#"
import {
    FileMode,
    FileOpenFlags,
    FileOpenOptions,
    FileResolveFlags,
} from "tspp:fs/binding";

function options(flags: FileOpenFlags): FileOpenOptions {
    return FileOpenOptions {
        flags,
        mode: FileMode(0),
        resolve: FileResolveFlags(0),
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an arbitrary function returning FileOpenFlags.
    #[test]
    fn test_accepts_file_open_flags_function() {
        let session = TestSession::dir(
            &NONSENSICAL_OPEN_OPTIONS,
            r#"
import {
    FileMode,
    FileOpenFlags,
    FileOpenOptions,
    FileResolveFlags,
} from "tspp:fs/binding";

declare function flags(value: uint32): FileOpenFlags;

const OPTIONS = FileOpenOptions {
    flags: flags(0x0038),
    mode: FileMode(0),
    resolve: FileResolveFlags(0),
};
"#,
        );

        session.assert_no_diagnostics();
    }
}
