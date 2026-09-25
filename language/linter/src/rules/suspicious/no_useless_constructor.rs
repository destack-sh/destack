use tspp_dir as dir;
use tspp_source::Patch;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow constructors that only repeat implicit construction behavior.
    pub NO_USELESS_CONSTRUCTOR {
        id: "no-useless-constructor",
        summary: "Disallow constructors that only repeat implicit construction behavior",
        explanation: r#"
An empty public constructor with the implicit signature performs the same initialization as an omitted constructor.
Instead, you SHOULD remove the constructor.

Constructors that change visibility, parameters, decorators, or initialization behavior remain meaningful.
"#,
        example: {
            reported: r#"
class Token {
    constructor() {}
}
"#,
            accepted: r#"
class Token {
}
"#,
        },
        provenance: [
            Eslint("no-useless-constructor"),
            TypeScriptEslint("no-useless-constructor"),
        ],
        category: Suspicious,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report empty public constructors with the implicit signature.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect concrete declaration constructors
    for (member, value) in view.iter_nodes::<dir::Member>() {
        let dir::Member::Method {
            signature,
            body: Some(body),
            visibility,
            ..
        } = value
        else {
            continue;
        };
        let Some(owner) = view.ancestor::<dir::Declaration>(member.into_any()) else {
            continue;
        };
        let has_implicit_constructor = matches!(
            view.get(owner),
            dir::Declaration::Struct(_)
                | dir::Declaration::Class(dir::ClassDeclaration {
                    extends_type: None,
                    ..
                })
        );
        if !has_implicit_constructor {
            continue;
        }

        // require the signature supplied by implicit construction
        if signature.role != Some(dir::FunctionRole::Constructor)
            || !signature.generic_parameters.is_empty()
            || !signature.where_clauses.is_empty()
            || signature.this_parameter.is_some()
            || !signature.parameters.is_empty()
            || !matches!(visibility, None | Some(dir::Visibility::Public))
        {
            continue;
        }

        // require an uncommented empty block and undecorated declaration
        let dir::Expression::Block(block) = view.get(*body) else {
            continue;
        };
        if !view.get(*block).is_empty() {
            continue;
        }
        let extent = module.source_extent(member.into_any())?;
        if module.has_unretained_comment(extent, &[])?
            || view.has_decorators_any(member.into_any())
            || view.has_decorators_any(owner.into_any())
        {
            continue;
        }

        // remove the complete constructor declaration
        let removal = module.line_removal_span(extent)?;
        let patch = Patch::delete(removal);
        let suggestion = lint.fix("remove the unnecessary constructor", patch)?;
        let span = module.main_span(member.into_any())?;
        let diagnostic = lint
            .diagnostic("constructor repeats implicit construction", span)
            .suggestion(suggestion);
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Remove an empty public constructor.
    #[test]
    fn test_removes_empty_constructor() {
        let session = TestSession::dir(
            &NO_USELESS_CONSTRUCTOR,
            r#"
class Token {
    constructor() {}
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-useless-constructor]: constructor repeats implicit construction
 ──▶ main.tspp:2:5
  │
1 │ class Token {
2 │     constructor() {}
  │     ^^^^^^^^^^^
3 │ }
  │

 = fix: remove the unnecessary constructor
--- a/main.tspp
+++ b/main.tspp

    1│ class Token {
-   2│     constructor() {}
    3│ }
"#,
        );
        session.assert_fixes(
            r#"
class Token {
}
"#,
        );
    }

    /// Accept a private constructor that restricts construction.
    #[test]
    fn test_accepts_private_constructor() {
        let session = TestSession::dir(
            &NO_USELESS_CONSTRUCTOR,
            r#"
class Token {
    private constructor() {}
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a constructor that establishes a typed call signature.
    #[test]
    fn test_accepts_parameterized_constructor() {
        let session = TestSession::dir(
            &NO_USELESS_CONSTRUCTOR,
            r#"
class Token {
    constructor(value: int32) {}
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a constructor with authored initialization.
    #[test]
    fn test_accepts_constructor_body() {
        let session = TestSession::dir(
            &NO_USELESS_CONSTRUCTOR,
            r#"
class Token {
    value: int32;

    constructor() {
        this.value = 1;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
