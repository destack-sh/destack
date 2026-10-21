import { describe, test } from "@destack/test";

/** Fail when static collection evaluates the module. */
function refuseEvaluation(): void {
    throw new Error("static inspection must not execute tests");
}
refuseEvaluation();

describe("application", () => {
    test("renders", () => {});
    test.each([1, 2])("renders %i", () => {});
});
