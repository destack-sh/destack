use tspp_dir as dir;
use tspp_source::Patch;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow empty static and compile-time initialization blocks.
    pub NO_EMPTY_STATIC_BLOCK {
        id: "no-empty-static-block",
        summary: "Disallow empty static and compile-time initialization blocks",
        explanation: r#"
An uncommented empty static or compile-time block performs no initialization.
Instead, you SHOULD remove the block or explain why it is intentionally empty with a comment.
"#,
        example: {
            reported: r#"
class Registry {
    static {}
    value(): int32 {
        return 1;
    }
}
"#,
            accepted: r#"
class Registry {
    value(): int32 {
        return 1;
    }
}
"#,
        },
        provenance: [Eslint("no-empty-static-block")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report empty static blocks without comments.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect static and compile-time initialization members
    for (member, node) in view.iter_nodes::<dir::Member>() {
        let (body, kind) = match node {
            dir::Member::StaticBlock { body } => (*body, "static"),
            dir::Member::ConstBlock { body } => (*body, "const"),
            _ => continue,
        };

        // require an empty body without retained comments
        let dir::Expression::Block(block) = view.get(body) else {
            continue;
        };
        if !view.get(*block).is_empty() {
            continue;
        }
        let span = module.source_extent(member.into_any())?;
        if module.has_unretained_comment(span, &[])? {
            continue;
        }

        // remove the complete empty member
        let removal = module.line_removal_span(span)?;
        let patch = Patch::delete(removal);
        let suggestion = lint.fix(format!("remove the empty {kind} block"), patch)?;
        let diagnostic = lint
            .diagnostic(format!("{kind} initialization block is empty"), span)
            .suggestion(suggestion);
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Retain an otherwise empty static block that contains a comment.
    #[test]
    fn test_accepts_commented_static_block() {
        let session = TestSession::dir(
            &NO_EMPTY_STATIC_BLOCK,
            r#"
class Registry {
    static {
        // initialized by the runtime
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Remove an empty static block written on the class line.
    #[test]
    fn test_removes_inline_empty_static_block() {
        let session = TestSession::dir(
            &NO_EMPTY_STATIC_BLOCK,
            r#"
class Registry { static {} }
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-empty-static-block]: static initialization block is empty
 ──▶ main.tspp:1:18
  │
1 │ class Registry { static {} }
  │                  ^^^^^^^^^
  │

 = fix: remove the empty static block
--- a/main.tspp
+++ b/main.tspp

-   1│ class Registry { static {} }
+   1│ class Registry { }
"#,
        );
        session.assert_fixes(
            r#"
class Registry { }
"#,
        );
    }

    /// Remove an empty compile-time initialization block.
    #[test]
    fn test_removes_empty_const_block() {
        let session = TestSession::dir(
            &NO_EMPTY_STATIC_BLOCK,
            r#"
struct Registry {
    const {}
}
"#,
        );

        session.assert_fixes(
            r#"
struct Registry {
}
"#,
        );
    }
}
