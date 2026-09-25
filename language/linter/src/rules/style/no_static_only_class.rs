use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow classes containing only static members.
    pub NO_STATIC_ONLY_CLASS {
        id: "no-static-only-class",
        summary: "Disallow classes containing only static members",
        explanation: r#"
A class containing only static members uses an instance-bearing declaration as a module namespace.
Instead, you SHOULD declare its constants, functions, and types directly in the source module.

Inheritance may supply instance behavior, so derived classes retain their declaration.
"#,
        example: {
            reported: r#"
class MathHelper {
    static double(value: int32): int32 {
        return value * 2;
    }
}
"#,
            accepted: r#"
function double(value: int32): int32 {
    return value * 2;
}
"#,
        },
        provenance: [Unicorn("no-static-only-class")],
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report classes containing only static members.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect class declarations
    for (node, declaration) in view.iter_nodes::<dir::Declaration>() {
        let dir::Declaration::Class(class) = declaration else {
            continue;
        };
        if class.members.is_empty() {
            continue;
        }

        // require at least one static member and no instance member beyond a constructor
        let mut has_static_member = false;
        let mut has_instance_member = false;
        for member in &class.members {
            match view.get(*member) {
                dir::Member::AssociatedType { .. }
                | dir::Member::AssociatedConst { .. }
                | dir::Member::StaticBlock { .. }
                | dir::Member::ConstBlock { .. }
                | dir::Member::Field {
                    is_static: true, ..
                }
                | dir::Member::Method {
                    is_static: true, ..
                } => has_static_member = true,
                dir::Member::Method {
                    signature,
                    visibility: Some(dir::Visibility::Private),
                    ..
                } if signature.role == Some(dir::FunctionRole::Constructor) => {}
                dir::Member::Field { .. } | dir::Member::Method { .. } => {
                    has_instance_member = true;
                }
                dir::Member::Error => {}
            }
        }
        if !has_static_member || has_instance_member {
            continue;
        }

        // retain classes with effective instance behavior from a base
        let symbol = module.declaration_symbol(node)?;
        if module.dir.inherits_instance_members(symbol)? {
            continue;
        }

        // identify the static-only class declaration
        let span = module.main_span(node.into_any())?;
        output.report(lint.diagnostic("class contains only static members", span));
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a private-constructor namespace class.
    #[test]
    fn test_reports_static_class_with_private_constructor() {
        let session = TestSession::dir(
            &NO_STATIC_ONLY_CLASS,
            r#"
class Values {
    private constructor() {}

    static DEFAULT = 1;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-static-only-class]: class contains only static members
 ──▶ main.tspp:1:7
  │
1 │ class Values {
  │       ^^^^^^
2 │     private constructor() {}
3 │
  │
"#,
        );
    }

    /// Accept a class with instance storage.
    #[test]
    fn test_accepts_instance_class() {
        let session = TestSession::dir(
            &NO_STATIC_ONLY_CLASS,
            r#"
class Counter {
    value: int32 = 0;

    static zero(): Counter {
        return new Counter();
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a publicly constructible class without instance fields.
    #[test]
    fn test_accepts_constructible_class() {
        let session = TestSession::dir(
            &NO_STATIC_ONLY_CLASS,
            r#"
class Token {
    constructor() {}

    static create(): Token {
        return new Token();
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a class inheriting instance behavior from its base.
    #[test]
    fn test_accepts_derived_static_class() {
        let session = TestSession::dir(
            &NO_STATIC_ONLY_CLASS,
            r#"
class Base {
    value: int32 = 0;
}

class Derived extends Base {
    static DEFAULT = 1;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report a derived class when its bases provide no instance members.
    #[test]
    fn test_reports_static_class_with_empty_base() {
        let session = TestSession::dir(
            &NO_STATIC_ONLY_CLASS,
            r#"
class Base {}

class Values extends Base {
    static DEFAULT = 1;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-static-only-class]: class contains only static members
 ──▶ main.tspp:3:7
  │
1 │ class Base {}
2 │
3 │ class Values extends Base {
  │       ^^^^^^
4 │     static DEFAULT = 1;
5 │ }
  │
"#,
        );
    }
}
