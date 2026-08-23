use destack_dir as dir;
use destack_source::{NodeSpanRegion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer this as the return type when a method only returns its receiver.
    pub PREFER_RETURN_THIS_TYPE {
        id: "prefer-return-this-type",
        summary: "Prefer this as the return type when a method only returns its receiver",
        explanation: r#"
A method that only returns its receiver preserves the receiver's concrete subtype.
Instead, you SHOULD declare its return type as `this`.
"#,
        example: {
            reported: r#"
class Builder {
    reset(): Builder {
        return this;
    }
}
"#,
            accepted: r#"
class Builder {
    reset(): this {
        return this;
    }
}
"#,
        },
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report instance methods whose returned values are all the receiver.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect concrete ordinary instance methods
    for (node, member) in view.iter_nodes::<dir::Member>() {
        let dir::Member::Method {
            signature,
            body: Some(body),
            is_static: false,
            ..
        } = member
        else {
            continue;
        };
        if signature.role.is_some() || signature.is_override {
            continue;
        }
        let Some(return_type) = signature.return_type else {
            continue;
        };
        if matches!(view.get(return_type), dir::TypeExpression::This) {
            continue;
        }

        // require every completed path to return this directly
        let Some(values) = module.callable_return_values(*body) else {
            continue;
        };
        if values.is_empty()
            || values
                .iter()
                .any(|value| !matches!(view.get(*value), dir::Expression::This))
        {
            continue;
        }

        // identify the redundant nominal return type
        let span = module.source_extent(return_type.into_any())?;

        // replace the complete authored return annotation
        let edit = module.source_region(node.into_any(), NodeSpanRegion::Type)?;
        let patch = Patch::replace(edit, ": this");
        let suggestion = lint.fix("return this", patch)?;
        let diagnostic = lint
            .diagnostic("method only returns its receiver", span)
            .suggestion(suggestion);
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a nominal return type when every path returns this.
    #[test]
    fn test_replaces_nominal_return_type() {
        let session = TestSession::dir(
            &PREFER_RETURN_THIS_TYPE,
            r#"
class Builder {
    clear(flag: boolean): Builder {
        if (flag) {
            return this;
        }

        return this;
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-return-this-type]: method only returns its receiver
 ──▶ main.ds:2:27
  │
1 │ class Builder {
2 │     clear(flag: boolean): Builder {
  │                           ^^^^^^^
3 │         if (flag) {
4 │             return this;
  │

 = fix: return this
--- a/main.ds
+++ b/main.ds

    1│ class Builder {
-   2│     clear(flag: boolean): Builder {
+   2│     clear(flag: boolean): this {
"#,
        );
        session.assert_fixes(
            r#"
class Builder {
    clear(flag: boolean): this {
        if (flag) {
            return this;
        }

        return this;
    }
}
"#,
        );
    }

    /// Accept a method that may return another instance.
    #[test]
    fn test_accepts_other_return_value() {
        let session = TestSession::dir(
            &PREFER_RETURN_THIS_TYPE,
            r#"
class Builder {
    choose(other: Builder, flag: boolean): Builder {
        return flag ? this : other;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
