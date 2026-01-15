# Optimize Debug Plan

## Intent
- Build a repeatable process to identify and fix optimizer miscompilations.
- Keep results reproducible and make each failure point to a specific pass.
- Separate baseline runtime issues from optimizer induced issues.

## Setup
- Use `destack_test` optimize runner for the execute suite.
- Run with a high instruction limit to avoid false EM014 where possible.
- Record the exact command and inputs for each failure.

## Phase 1: Baseline validity at O0
- Purpose is to verify the program can run without optimization.
- Any failure at O0 is a runtime or fixture issue, not an optimization issue.
- Command per case:
  - `cargo test -p destack_test --test optimize -- --execute --level O0 --max-instructions 1000000000 --verbose <case>`
- Expected result is a clean baseline output for the case.
- If the baseline fails, fix the runtime or fixture first before debugging optimizer passes.

## Phase 2: Capture O2 failures
- Purpose is to confirm failures that only happen under optimization.
- Command per case:
  - `cargo test -p destack_test --test optimize -- --execute --level O2 --max-instructions 1000000000 --verbose <case>`
- Record the failure type and message for each case.

## Phase 3: Isolate the first bad pass
- Purpose is to identify the first pass that changes execution behavior.
- Command per case:
  - `cargo test -p destack_test --test optimize -- --execute --level O2 --max-instructions 1000000000 --matrix --diagnostic --verbose <case>`
- The diagnostic output should report the first mismatching pass.
- Map each case to the first bad pass and cluster the failures by pass.

## Phase 4: Fix and validate per pass
- For each pass cluster, create a minimal reproducer if possible.
- Fix the pass or its prerequisites and add tests where appropriate.
- Re run the case at O2 and confirm the failure is gone.

## Phase 5: Full suite regression
- Run the full O2 execute suite after each major fix.
- Command:
  - `cargo test -p destack_test --test optimize -- --execute --level O2 --max-instructions 1000000000 --verbose`
- Track the failure count over time and keep a simple changelog.

## Example failure patterns
- `EM003: type mismatch expected raw_pointer got ManagedReference(...)` often indicates a pass that changes value types without updating uses.
- `expected Int ... got Int ...` indicates semantic change or broken data flow.
- `expected Int ... got Void` indicates a dropped return value or invalid call rewrite.
- `EM014: execution step limit exceeded` can indicate an infinite loop or overly aggressive transform.

## Examples seen so far
- `loop_countdown` reported EM014 at O2 with a 10M cap.
- `json_like_scan` reported a reduced numeric result.
- `fib_recursive` reported width 0 on a return value.
- `matmul_32` reported raw pointer versus managed reference mismatch.

## Suggested error categories
- `optimizer_miscompile` for wrong results without runtime type errors.
- `optimizer_type_corruption` for raw pointer or aggregate mismatches.
- `optimizer_infinite_loop` for EM014 under optimization only.
- `fixture_or_runtime_issue` for baseline failures at O0.
