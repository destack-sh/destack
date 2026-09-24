use crate::tests::{DirRows, TestSession};

/// Check test registration and matchers through the public test API.
#[test]
fn test_register_suites_and_matchers_through_the_test_api() {
    let session = TestSession::single(
        r#"
import { beforeEach, describe, expect, test } from "destack:test";

describe("arithmetic", () => {
    beforeEach(() => {});

    test("adds values", () => {
        expect(1 + 1).toEqual(2);
        expect(2).toBe(2);
        expect.soft("pineapple").toContain("apple");
    });

    test.only("focused", () => {});
    test.skip("skipped", () => {});
    test.todo("pending");
});
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none(),
        r#"
=== annotated ===
import { beforeEach, describe, expect, test } from "destack:test";

describe("arithmetic", ((): void => {
    beforeEach((): BodyResult => {});

    test("adds values", ((): BodyResult => {
        expect<2, "frame">((1 + 1) as &'frame immutable 2).toEqual<"frame", 2, int64, "frame">(
            2 as &'frame immutable int64,
        );
        expect<int64, "frame">(2 as &'frame immutable int64).toBe<"frame", int64, "frame">(
            2 as &'frame immutable int64,
        );
        expect.soft<string, "managed">("pineapple" as &'managed immutable string).toContain<
            "managed",
            string
        >("apple");
    }) as Body<TestContext<{}, {}, {}>> | undefined);

    test.only("focused", ((): BodyResult => {}) as Body<TestContext<{}, {}, {}>> | undefined);
    test.skip("skipped", ((): BodyResult => {}) as Body<TestContext<{}, {}, {}>> | undefined);
    test.todo("pending");
}) as SuiteBody | undefined);

=== dir ===
import { beforeEach, describe, expect, test } from "destack:test";

describe("arithmetic", () => {
    beforeEach(() => {});

    test("adds values", () => {
        expect(1 + 1).toEqual(2);
        expect(2).toBe(2);
        expect.soft("pineapple").toContain("apple");
    });

    test.only("focused", () => {});
    test.skip("skipped", () => {});
    test.todo("pending");
});
"#,
        r#"
"#,
    );
}
