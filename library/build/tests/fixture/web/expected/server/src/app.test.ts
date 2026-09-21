import { describe, test } from "@destack/test";

// fail if static collection evaluates the module
(() => {
    throw new Error("static inspection must not execute tests");
})();

describe("application", () => {
    test("renders", () => {});
    test.each([1, 2])("renders %i", () => {});
});
