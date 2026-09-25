use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow aliasing the receiver into a binding.
    pub NO_THIS_ALIAS {
        id: "no-this-alias",
        summary: "Disallow aliasing the receiver into a binding",
        explanation: r#"
Binding `this` introduces a second name for the current receiver even though lexical lambdas retain that receiver directly.
Instead, you SHOULD use `this` directly or replace a receiver-capturing function with a lexical lambda.
"#,
        example: {
            reported: r#"
class Service {
    run(): void {
        const self = this;
        self.flush();
    }

    flush(): void {}
}
"#,
            accepted: r#"
class Service {
    run(): void {
        this.flush();
    }

    flush(): void {}
}
"#,
        },
        provenance: [TypeScriptEslint("no-this-alias")],
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report direct bindings of the current receiver.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect declarators initialized directly from this
    for (_, declarator) in view.iter_nodes::<dir::Declarator>() {
        let Some(value) = declarator.value else {
            continue;
        };

        // require a direct this value and a simple binding pattern
        if !matches!(view.get(value), dir::Expression::This) {
            continue;
        }
        if !matches!(view.get(declarator.pattern), dir::Pattern::Binding { .. }) {
            continue;
        }

        // report the introduced alias
        let span = module.main_span(declarator.pattern.into_any())?;
        output.report(lint.diagnostic("binding aliases the current receiver", span));
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Accept destructuring properties from the receiver.
    #[test]
    fn test_accepts_receiver_destructuring() {
        let session = TestSession::dir(
            &NO_THIS_ALIAS,
            r#"
class Service {
    isReady: boolean = true;

    ready(): boolean {
        const { isReady } = this;
        return isReady;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report a mutable binding that aliases the receiver.
    #[test]
    fn test_reports_mutable_receiver_alias() {
        let session = TestSession::dir(
            &NO_THIS_ALIAS,
            r#"
class Service {
    run(): void {
        let context = this;
        context.flush();
    }

    flush(): void {}
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-this-alias]: binding aliases the current receiver
 ──▶ main.tspp:3:13
  │
1 │ class Service {
2 │     run(): void {
3 │         let context = this;
  │             ^^^^^^^
4 │         context.flush();
5 │     }
  │
"#,
        );
    }
}
