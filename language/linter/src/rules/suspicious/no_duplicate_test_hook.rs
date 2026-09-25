use tspp_core::FxIndexSet;
use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow duplicate lifecycle hooks in one test suite.
    pub NO_DUPLICATE_TEST_HOOK {
        id: "no-duplicate-test-hook",
        summary: "Disallow duplicate lifecycle hooks in one test suite",
        explanation: r#"
Repeated hooks of the same kind make suite setup and cleanup order depend on registration order.
Instead, you SHOULD combine each suite's repeated hook bodies into one hook.
"#,
        example: {
            reported: r#"
import { beforeEach } from "tspp:test";

beforeEach(() => {
    // ...
});
beforeEach(() => {
    // ...
});
"#,
            accepted: r#"
import { beforeEach } from "tspp:test";

beforeEach(() => {
    // ...
    // ...
});
"#,
        },
        provenance: [Jest("no-duplicate-hooks"), Playwright("no-duplicate-hooks")],
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// The suite or Test value that receives one hook.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum HookReceiver {
    /// The current suite.
    Suite,
    /// One scoped Test value.
    Test(dir::GlobalSymbolId),
}

/// Report duplicate canonical test hooks within each lexical suite.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();
    let mut hooks = FxIndexSet::default();

    // inspect canonical test hook registrations
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.test_hook(expression)? else {
            continue;
        };
        let Some(suite) = module.test_suite_scope(expression.into_any())? else {
            continue;
        };
        let receiver = match call.receiver {
            None => HookReceiver::Suite,
            Some(receiver) => {
                let Some(symbol) = module.selected_symbol(receiver)? else {
                    continue;
                };

                HookReceiver::Test(symbol)
            }
        };

        // report every hook after the first equal registration
        if !hooks.insert((suite, receiver, call.hook)) {
            let dir::Expression::Call { left, .. } = module.view().get(expression) else {
                continue;
            };
            let span = module.main_span(left.into_any())?;
            let message = format!("duplicate {} hook in one test suite", call.hook.name());
            output.report(lint.diagnostic(message, span));
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report duplicate root and nested suite hooks independently.
    #[test]
    fn test_reports_duplicate_hooks_by_suite() {
        let session = TestSession::dir(
            &NO_DUPLICATE_TEST_HOOK,
            r#"
import { beforeEach as prepare, describe } from "tspp:test";

prepare(() => {
    // first root hook
});
prepare(() => {
    // duplicate root hook
});

describe("nested", () => {
    prepare(() => {
        // first nested hook
    });
    prepare(() => {
        // duplicate nested hook
    });
});
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-duplicate-test-hook]: duplicate beforeEach hook in one test suite
 ──▶ main.tspp:6:1
  │
4 │     // first root hook
5 │ });
6 │ prepare(() => {
  │ ^^^^^^^
7 │     // duplicate root hook
8 │ });
  │

warning[no-duplicate-test-hook]: duplicate beforeEach hook in one test suite
  ──▶ main.tspp:14:5
   │
12 │         // first nested hook
13 │     });
14 │     prepare(() => {
   │     ^^^^^^^
15 │         // duplicate nested hook
16 │     });
   │
"#,
        );
    }

    /// Keep each distinct lifecycle hook kind in one suite.
    #[test]
    fn test_accepts_distinct_hook_kinds() {
        let session = TestSession::dir(
            &NO_DUPLICATE_TEST_HOOK,
            r#"
import {
    afterAll,
    afterEach,
    aroundAll,
    aroundEach,
    beforeAll,
    beforeEach,
} from "tspp:test";

beforeEach(() => {
    // root setup
});
afterEach(() => {
    // root cleanup
});
beforeAll(() => {
    // suite setup
});
afterAll(() => {
    // suite cleanup
});
aroundEach((run, _context) => run());
aroundAll((run, _context) => run());
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Keep equal hooks in different suites.
    #[test]
    fn test_accepts_equal_hooks_in_different_suites() {
        let session = TestSession::dir(
            &NO_DUPLICATE_TEST_HOOK,
            r#"
import { beforeEach, describe } from "tspp:test";

describe("first", () => {
    beforeEach(() => {
        // first suite setup
    });
});
describe("second", () => {
    beforeEach(() => {
        // second suite setup
    });
});
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Keep equal global and scoped Test hooks in separate registries.
    #[test]
    fn test_accepts_equal_global_and_scoped_hooks() {
        let session = TestSession::dir(
            &NO_DUPLICATE_TEST_HOOK,
            r#"
import { beforeEach, test } from "tspp:test";

beforeEach(() => {
    // root setup
});

test.beforeEach(() => {
    // scoped root setup
});
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report duplicate hooks on one scoped Test value only.
    #[test]
    fn test_reports_duplicate_scoped_hooks() {
        let session = TestSession::dir(
            &NO_DUPLICATE_TEST_HOOK,
            r#"
import { test } from "tspp:test";

const checked = test.extend("value", 1);
const other = test.extend("value", 2);

checked.beforeEach(() => {
    // first scoped hook
});
checked.beforeEach(() => {
    // duplicate scoped hook
});
other.beforeEach(() => {
    // distinct scoped hook
});
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-duplicate-test-hook]: duplicate beforeEach hook in one test suite
  ──▶ main.tspp:9:9
   │
 7 │     // first scoped hook
 8 │ });
 9 │ checked.beforeEach(() => {
   │         ^^^^^^^^^^
10 │     // duplicate scoped hook
11 │ });
   │
"#,
        );
    }

    /// Ignore hook-shaped calls inside ordinary functions.
    #[test]
    fn test_accepts_runtime_hook_calls() {
        let session = TestSession::dir(
            &NO_DUPLICATE_TEST_HOOK,
            r#"
import { beforeEach } from "tspp:test";

function register(): void {
    beforeEach(() => {
        // first runtime call
    });
    beforeEach(() => {
        // second runtime call
    });
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
