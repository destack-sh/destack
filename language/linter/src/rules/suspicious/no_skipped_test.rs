use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow skipped tests and suites.
    pub NO_SKIPPED_TEST {
        id: "no-skipped-test",
        summary: "Disallow skipped tests and suites",
        explanation: r#"
A skipped test or suite leaves expected behavior unverified while the test run can still pass.
Instead, you SHOULD restore the test or remove obsolete coverage.
"#,
        example: {
            reported: r#"
import { test } from "tspp:test";

test.skip("adds values", () => {
    // ...
});
"#,
            accepted: r#"
import { test } from "tspp:test";

test("adds values", () => {
    // ...
});
"#,
        },
        provenance: [Jest("no-disabled-tests"), Playwright("no-skipped-test")],
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report skipped canonical test registrations.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // report every canonical skip modifier
    for (modifier, _) in module.view().iter_nodes::<dir::Expression>() {
        if module.test_modifier(modifier, "skip")?.is_none() {
            continue;
        }
        let span = module.main_span(modifier.into_any())?;
        output.report(lint.diagnostic("skipped registration leaves coverage inactive", span));
    }

    // inspect configured test and suite registrations
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(registration) = module.test_registration(expression)? else {
            continue;
        };

        // report an enabled skip option
        let Some((property, _)) = module.test_option(registration, "skip") else {
            continue;
        };
        let span = module.main_span(property.into_any())?;
        output.report(lint.diagnostic("skipped registration leaves coverage inactive", span));
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report direct and aliased skip modifiers on tests and suites.
    #[test]
    fn test_reports_skip_modifiers() {
        let session = TestSession::dir(
            &NO_SKIPPED_TEST,
            r#"
import { describe as suite, test } from "tspp:test";

test.skip("direct", () => {
    // empty
});
suite.skip("nested", () => {
    // empty
});
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-skipped-test]: skipped registration leaves coverage inactive
 ──▶ main.tspp:3:6
  │
1 │ import { describe as suite, test } from "tspp:test";
2 │
3 │ test.skip("direct", () => {
  │      ^^^^
4 │     // empty
5 │ });
  │

warning[no-skipped-test]: skipped registration leaves coverage inactive
 ──▶ main.tspp:6:7
  │
4 │     // empty
5 │ });
6 │ suite.skip("nested", () => {
  │       ^^^^
7 │     // empty
8 │ });
  │
"#,
        );
    }

    /// Report enabled skip options on direct registrations.
    #[test]
    fn test_reports_skip_options() {
        let session = TestSession::dir(
            &NO_SKIPPED_TEST,
            r#"
import { test } from "tspp:test";

test("configured", { skip: true }, () => {
    // empty
});
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-skipped-test]: skipped registration leaves coverage inactive
 ──▶ main.tspp:3:22
  │
1 │ import { test } from "tspp:test";
2 │
3 │ test("configured", { skip: true }, () => {
  │                      ^^^^
4 │     // empty
5 │ });
  │
"#,
        );
    }

    /// Accept active, pending, and conditionally skipped registrations.
    #[test]
    fn test_accepts_other_test_states() {
        let session = TestSession::dir(
            &NO_SKIPPED_TEST,
            r#"
import { test } from "tspp:test";

test("active", { skip: false }, () => {
    // empty
});
test.todo("pending");
test.skipIf(true)("conditional", () => {
    // empty
});
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept unrelated skip member calls.
    #[test]
    fn test_accepts_unrelated_skip_members() {
        let session = TestSession::dir(
            &NO_SKIPPED_TEST,
            r#"
interface Runner {
    skip(name: string, body: () => void): void;
}

function register(runner: Runner): void {
    runner.skip("unrelated", () => {
        // empty
    });
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
