use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow focused tests and suites.
    pub NO_FOCUSED_TEST {
        id: "no-focused-test",
        summary: "Disallow focused tests and suites",
        explanation: r#"
A focused test or suite prevents unrelated tests from running and can conceal regressions.
Instead, you MUST remove the focus modifier before committing the test.
"#,
        example: {
            reported: r#"
import { test } from "tspp:test";

test.only("adds values", () => {
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
        provenance: [Jest("no-focused-tests"), Playwright("no-focused-test")],
        category: Suspicious,
        level: Error,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report focused canonical test registrations.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // report every canonical only modifier
    for (modifier, _) in module.view().iter_nodes::<dir::Expression>() {
        let Some(receiver) = module.test_modifier(modifier, "only")? else {
            continue;
        };
        let span = module.main_span(modifier.into_any())?;
        let mut diagnostic = lint.diagnostic("focused registration can exclude other tests", span);
        if let Some(suggestion) = remove_modifier(module, lint, modifier, receiver)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    // inspect configured test and suite registrations
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(registration) = module.test_registration(expression)? else {
            continue;
        };

        // report an enabled only option
        let Some((property, value)) = module.test_option(registration, "only") else {
            continue;
        };
        let span = module.main_span(property.into_any())?;
        let patch = Patch::replace(module.source_extent(value.into_any())?, "false");
        let suggestion = lint.fix("disable test focus", patch)?;
        let diagnostic = lint
            .diagnostic("focused registration can exclude other tests", span)
            .suggestion(suggestion);
        output.report(diagnostic);
    }

    Ok(output)
}

/// Remove one test registration modifier without discarding comments.
fn remove_modifier(
    module: &DirModule<'_>,
    lint: &Lint,
    modifier: dir::LocalNodeId<dir::Expression>,
    receiver: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    // preserve optional access behavior
    let dir::Expression::Member {
        is_optional: false, ..
    } = module.view().get(modifier)
    else {
        return Ok(None);
    };

    // remove only the final member suffix
    let extent = module.source_extent(modifier.into_any())?;
    let receiver = module.source_extent(receiver.into_any())?;
    let removed = Span::new(extent.file, receiver.end, extent.end);
    if module.has_unretained_comment(removed, &[])? {
        return Ok(None);
    }

    // preserve independent fixes for repeated modifiers
    let patch = Patch::replace(removed, "");
    let suggestion = lint.fix("remove test focus", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Remove direct, aliased, and repeated focus modifiers.
    #[test]
    fn test_removes_focused_modifiers() {
        let session = TestSession::dir(
            &NO_FOCUSED_TEST,
            r#"
import { test as check } from "tspp:test";

const focused = check.only;

focused("aliased", () => {
    // empty
});
check.only("direct", () => {
    // empty
});
check.only.only("repeated", () => {
    // empty
});
"#,
        );

        session.assert_fixes(
            r#"
import { test as check } from "tspp:test";

const focused = check;

focused("aliased", () => {
    // empty
});
check("direct", () => {
    // empty
});
check("repeated", () => {
    // empty
});
"#,
        );
    }

    /// Remove focus from suites and nested test registrations.
    #[test]
    fn test_removes_focused_suites() {
        let session = TestSession::dir(
            &NO_FOCUSED_TEST,
            r#"
import { describe, test } from "tspp:test";

describe.only("suite", () => {
    test.concurrent.only("nested", () => {
        // empty
    });
});
"#,
        );

        session.assert_fixes(
            r#"
import { describe, test } from "tspp:test";

describe("suite", () => {
    test.concurrent("nested", () => {
        // empty
    });
});
"#,
        );
    }

    /// Disable focus options on direct registrations.
    #[test]
    fn test_disables_focus_options() {
        let session = TestSession::dir(
            &NO_FOCUSED_TEST,
            r#"
import { describe } from "tspp:test";

const options = { only: false };

describe("configured", { only: true }, () => {
    // empty
});
describe("spread configured", { ...options, only: true }, () => {
    // empty
});
"#,
        );

        session.assert_fixes(
            r#"
import { describe } from "tspp:test";

const options = { only: false };

describe("configured", { only: false }, () => {
    // empty
});
describe("spread configured", { ...options, only: false }, () => {
    // empty
});
"#,
        );
    }

    /// Remove focus from parameterized test and suite registrations.
    #[test]
    fn test_removes_parameterized_focus() {
        let session = TestSession::dir(
            &NO_FOCUSED_TEST,
            r#"
import { describe, test } from "tspp:test";

test.each([(1,)]).only("parameterized case", (value: int32) => {
    // empty
});
test.for([1]).only("table case", (value: &readonly int64) => {
    // empty
});
describe.each([(1,)]).only("parameterized suite", (value: int32) => {
    // empty
});
describe.for([1]).only("table suite", (value: int64) => {
    // empty
});
"#,
        );

        session.assert_fixes(
            r#"
import { describe, test } from "tspp:test";

test.each([(1,)])("parameterized case", (value: int32) => {
    // empty
});
test.for([1])("table case", (value: &readonly int64) => {
    // empty
});
describe.each([(1,)])("parameterized suite", (value: int32) => {
    // empty
});
describe.for([1])("table suite", (value: int64) => {
    // empty
});
"#,
        );
    }

    /// Preserve comments when removing the focus modifier would discard one.
    #[test]
    fn test_reports_commented_modifier_without_fix() {
        let session = TestSession::dir(
            &NO_FOCUSED_TEST,
            r#"
import { test } from "tspp:test";

test /* retain */ .only("focused", () => {
    // empty
});
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-focused-test]: focused registration can exclude other tests
 ──▶ main.tspp:3:20
  │
1 │ import { test } from "tspp:test";
2 │
3 │ test /* retain */ .only("focused", () => {
  │                    ^^^^
4 │     // empty
5 │ });
  │
"#,
        );
    }

    /// Report optional focus without changing its conditional evaluation.
    #[test]
    fn test_reports_optional_modifier_without_fix() {
        let session = TestSession::dir(
            &NO_FOCUSED_TEST,
            r#"
import { Test } from "tspp:test";

function register(selected: Test | undefined): void {
    selected?.only("focused", () => {
        // empty
    });
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-focused-test]: focused registration can exclude other tests
 ──▶ main.tspp:4:15
  │
2 │
3 │ function register(selected: Test | undefined): void {
4 │     selected?.only("focused", () => {
  │               ^^^^
5 │         // empty
6 │     });
  │
"#,
        );
    }

    /// Accept disabled focus options and later spread overrides.
    #[test]
    fn test_accepts_disabled_focus_options() {
        let session = TestSession::dir(
            &NO_FOCUSED_TEST,
            r#"
import { test } from "tspp:test";

const options = { only: false };

test("ordinary", { only: false }, () => {
    // empty
});
test("spread override", { only: true, ...options }, () => {
    // empty
});
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept conditional modifiers and unrelated member calls.
    #[test]
    fn test_accepts_other_modifiers() {
        let session = TestSession::dir(
            &NO_FOCUSED_TEST,
            r#"
import { test } from "tspp:test";

test.skipIf(true)("conditional", () => {
    // empty
});

interface Runner {
    only(name: string, body: () => void): void;
}

function register(runner: Runner): void {
    runner.only("unrelated", () => {
        // empty
    });
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
