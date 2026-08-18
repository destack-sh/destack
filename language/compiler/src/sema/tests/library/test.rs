use crate::tests::{DirRows, TestSession};

/// Check test registration and matchers through the public test API.
#[test]
fn test_check_test_api() {
    let session = TestSession::single(
        r#"
import {
    beforeEach,
    describe,
    expect,
    expectSoft,
    test,
    testOnly,
    testSkip,
    testTodo,
} from "destack:test";

describe("arithmetic", () => {
    beforeEach(() => {});

    test("adds values", () => {
        expect(1 + 1).toEqual(2);
        expect(2).toBe(2);
        expectSoft("pineapple").toContain("apple");
    });

    testOnly("focused", () => {});
    testSkip("skipped", () => {});
    testTodo("pending");
});
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none(),
        r#"
=== annotated ===
import {
    beforeEach,
    describe,
    expect,
    expectSoft,
    test,
    testOnly,
    testSkip,
    testTodo,
} from "destack:test";

describe("arithmetic", ((): void => {
    beforeEach((): BodyResult => {});

    test("adds values", ((): BodyResult => {
        expect<float64>((1 + 1) as &'frame readonly float64).toEqual<float64, float64>(
            2 as &'frame readonly float64,
        );
        expect<float64>(2 as &'frame readonly float64).toBe<float64>(2 as &'frame readonly float64);
        expectSoft<string>("pineapple").toContain<string>("apple" as &'frame readonly string);
    }) as CaseArgument | undefined);

    testOnly("focused", ((): BodyResult => {}) as CaseArgument | undefined);
    testSkip("skipped", ((): BodyResult => {}) as CaseArgument | undefined);
    testTodo("pending");
}) as SuiteArgument | undefined);

=== dir ===
import {
    beforeEach,
    describe,
    expect,
    expectSoft,
    test,
    testOnly,
    testSkip,
    testTodo,
} from "destack:test";

describe("arithmetic", () => {
    beforeEach(() => {});

    test("adds values", () => {
        expect(1 + 1).toEqual(2);
        expect(2).toBe(2);
        expectSoft("pineapple").toContain("apple");
    });

    testOnly("focused", () => {});
    testSkip("skipped", () => {});
    testTodo("pending");
});
"#,
        "",
    );
}
