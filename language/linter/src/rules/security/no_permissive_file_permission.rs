use tspp_dir as dir;

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
import { FileMode } from "tspp:fs/binding";

const mode = FileMode(0o666);
"#,
            accepted: r#"
import { FileMode } from "tspp:fs/binding";

const mode = FileMode(0o640);
"#,
        },
        provenance: [
            Clippy("non_octal_unix_permissions"),
            Ruff("bad-file-permissions"),
        ],
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

    // inspect canonical FileMode constructions
    for (expression, _) in view.iter_nodes::<dir::Expression>() {
        let Some((value, literal)) =
            module.newtype_integral(expression, dir::LanguageItem::FileMode)?
        else {
            continue;
        };
        let span = module.source_extent(literal.into_any())?;
        let token = module.token(span)?;
        let is_decimal = value != 0
            && matches!(
                token.literal(),
                Some(dir::TokenLiteral::Int {
                    base: dir::NumberBase::Decimal,
                    ..
                })
            );
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
import { FileMode } from "tspp:fs/binding";

const mode = FileMode(416);
"#,
        );

        session.assert_diagnostics(
            r#"warning[no-permissive-file-permission]: file permission uses decimal notation
 ──▶ main.tspp:3:14
  │
1 │ import { FileMode } from "tspp:fs/binding";
2 │
3 │ const mode = FileMode(416);
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
import { FileMode } from "tspp:fs/binding";

const mode = FileMode(438);
"#,
        );

        session.assert_diagnostics(
            r#"warning[no-permissive-file-permission]: file permission is world-writable and uses decimal notation
 ──▶ main.tspp:3:14
  │
1 │ import { FileMode } from "tspp:fs/binding";
2 │
3 │ const mode = FileMode(438);
  │              ^^^^^^^^^^^^^
  │

 = help: remove other-write permission and use an octal literal
"#,
        );
    }

    /// Accept explicit safe mode values, mode components, and zero.
    #[test]
    fn test_accepts_explicit_mode_values() {
        let session = TestSession::dir(
            &NO_PERMISSIVE_FILE_PERMISSION,
            r#"
import { FileMode } from "tspp:fs/binding";

const empty = FileMode(0);

const restricted = FileMode(0o640);

const upperOctal = FileMode(0O640);

const typeMask = FileMode(0Xf000);

const readBit = FileMode(0B100);
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a runtime-computed permission value.
    #[test]
    fn test_accepts_runtime_mode() {
        let session = TestSession::dir(
            &NO_PERMISSIVE_FILE_PERMISSION,
            r#"
import { FileMode } from "tspp:fs/binding";

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
import { FileMode } from "tspp:fs/binding";

declare function mode(value: uint32): FileMode;

const fileMode = mode(438);
"#,
        );

        session.assert_no_diagnostics();
    }
}
