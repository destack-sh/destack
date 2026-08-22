use destack_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

const OTHER_WRITE: i64 = 0o002;

declare_lint! {
    /// Disallow world-writable and decimal file permission literals.
    pub NO_PERMISSIVE_FILE_PERMISSION {
        id: "no-permissive-file-permission",
        summary: "Disallow world-writable and decimal file permission literals",
        explanation: r#"
World-writable permissions let every local user modify the created entry, while decimal notation obscures the selected mode bits.
Instead, you SHOULD grant only the required permissions and write mode literals in octal notation.
"#,
        example: {
            reported: r#"
import { FileMode } from "destack:fs/binding";

const MODE = FileMode(0o666);
"#,
            accepted: r#"
import { FileMode } from "destack:fs/binding";

const MODE = FileMode(0o640);
"#,
        },
        category: Security,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report unsafe or opaque authored filesystem permission modes.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect expressions whose checked representation is FileMode
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        if !matches!(node, dir::Expression::Call { .. }) {
            continue;
        }
        if module.representation_item(expression.into_any())? != Some(dir::LanguageItem::FileMode) {
            continue;
        }
        let Some((value, literal)) =
            module.newtype_integral(expression, dir::LanguageItem::FileMode)?
        else {
            continue;
        };
        let source = module.source(module.source_extent(literal.into_any())?)?;
        let is_decimal = source.as_bytes().first().is_some_and(u8::is_ascii_digit)
            && !source.starts_with("0o")
            && !source.starts_with("0x")
            && !source.starts_with("0b");
        let is_world_writable = value & OTHER_WRITE != 0;
        if !is_decimal && !is_world_writable {
            continue;
        }

        // report the exact permission expression
        let span = module.source_extent(expression.into_any())?;
        let (message, help) = match (is_world_writable, is_decimal) {
            (true, true) => (
                "file permission is world-writable and uses decimal notation",
                "remove other-write permission and use an octal literal",
            ),
            (true, false) => (
                "file permission is world-writable",
                "remove other-write permission",
            ),
            (false, true) => (
                "file permission uses decimal notation",
                "use an octal permission literal",
            ),
            (false, false) => continue,
        };
        output.report(lint.diagnostic(message, span).help(help));
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report an octal world-writable mode.
    #[test]
    fn test_reports_world_writable_mode() {
        TestSession::assert_example(&NO_PERMISSIVE_FILE_PERMISSION);
    }

    /// Report a safe permission written in decimal notation.
    #[test]
    fn test_reports_decimal_mode() {
        let session = TestSession::dir(
            &NO_PERMISSIVE_FILE_PERMISSION,
            r#"
import { FileMode } from "destack:fs/binding";

const MODE = FileMode(416);
"#,
        );

        session.assert_diagnostics(
            r#"warning[no-permissive-file-permission]: file permission uses decimal notation
 ──▶ main.ds:3:14
  │
1 │ import { FileMode } from "destack:fs/binding";
2 │
3 │ const MODE = FileMode(416);
  │              ^^^^^^^^^^^^^
  │

 = help: use an octal permission literal
"#,
        );
    }

    /// Report a world-writable mode written in decimal notation.
    #[test]
    fn test_reports_decimal_world_writable_mode() {
        let session = TestSession::dir(
            &NO_PERMISSIVE_FILE_PERMISSION,
            r#"
import { FileMode } from "destack:fs/binding";

const MODE = FileMode(438);
"#,
        );

        session.assert_diagnostics(
            r#"warning[no-permissive-file-permission]: file permission is world-writable and uses decimal notation
 ──▶ main.ds:3:14
  │
1 │ import { FileMode } from "destack:fs/binding";
2 │
3 │ const MODE = FileMode(438);
  │              ^^^^^^^^^^^^^
  │

 = help: remove other-write permission and use an octal literal
"#,
        );
    }

    /// Accept a restricted octal permission.
    #[test]
    fn test_accepts_restricted_mode() {
        let session = TestSession::dir(
            &NO_PERMISSIVE_FILE_PERMISSION,
            NO_PERMISSIVE_FILE_PERMISSION.example.accepted.source(),
        );

        session.assert_no_diagnostics();
    }

    /// Accept a runtime-computed permission value.
    #[test]
    fn test_accepts_runtime_mode() {
        let session = TestSession::dir(
            &NO_PERMISSIVE_FILE_PERMISSION,
            r#"
import { FileMode } from "destack:fs/binding";

function preserve(mode: FileMode): FileMode {
    return mode;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an arbitrary function returning FileMode.
    #[test]
    fn test_accepts_file_mode_function() {
        let session = TestSession::dir(
            &NO_PERMISSIVE_FILE_PERMISSION,
            r#"
import { FileMode } from "destack:fs/binding";

declare function mode(value: uint32): FileMode;

const MODE = mode(438);
"#,
        );

        session.assert_no_diagnostics();
    }
}
