import { expect, test } from "@destack/test";
import * as linear from "./linear.ts";
import * as native from "./native.ts";

test("match declared patterns alike, the linear one refusing backreferences RE2 lacks", () => {
    // match a plain pattern with both, then compile a backreference with each
    const linearPattern = new linear.Pattern("^[a-z]+-[0-9]+$");
    const nativePattern = new native.Pattern("^[a-z]+-[0-9]+$");
    const refused = (() => {
        try {
            return new linear.Pattern("^(a)\\1$");
        } catch (error) {
            return error instanceof TypeError ? error.message : "other";
        }
    })();

    // expect the same matches, and the backreference refused by RE2 alone
    expect({
        linear: [linearPattern.test("ada-1"), linearPattern.test("Ada 1")],
        native: [nativePattern.test("ada-1"), nativePattern.test("Ada 1")],
        refused,
        accepted: new native.Pattern("^(a)\\1$").test("aa"),
    }).toEqual({
        linear: [true, false],
        native: [true, false],
        refused: "json schema pattern '^(a)\\1$' is not declarable",
        accepted: true,
    });
});
