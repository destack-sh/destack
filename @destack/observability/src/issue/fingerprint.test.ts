import { expect, test } from "@destack/test";
import { fingerprint } from "./fingerprint.ts";

/** An in-app frame running a symbol. */
const frame = (moniker: string, line = 1) => ({
    file: "a.js",
    line,
    column: 1,
    path: "src/note.ts",
    isInApp: true,
    moniker,
});

test("group by error type and in-app symbols across lines and recursion, else by message, unless the capture names the grouping", async () => {
    const exception = { errorType: "RangeError", message: "page 9" };
    const symbols = [frame("#Note.rename"), frame("#check")];
    const moved = [frame("#Note.rename", 40), frame("#Note.rename", 41), frame("#check", 80)];
    const dependency = [{ file: "b.js", line: 1, column: 1, isInApp: false }];

    // keep one group across moved lines and recursion, split on another symbol or message
    const [same, recursed, other, byMessage, otherMessage] = await Promise.all([
        fingerprint(exception, symbols),
        fingerprint(exception, moved),
        fingerprint(exception, [frame("#archive")]),
        fingerprint(exception, dependency),
        fingerprint({ ...exception, message: "page 10" }, dependency),
    ]);
    expect([recursed === same, other === same, otherMessage === byMessage]).toEqual([
        true,
        false,
        false,
    ]);

    // replace or extend the default grouping as the capture asks
    const [requested, extended, tenant] = await Promise.all([
        fingerprint({ ...exception, fingerprint: ["quota"] }, symbols),
        fingerprint({ ...exception, fingerprint: ["{{ default }}", "tenant-a"] }, symbols),
        fingerprint({ ...exception, fingerprint: ["{{ default }}", "tenant-b"] }, symbols),
    ]);
    expect([requested === same, extended === same, extended === tenant]).toEqual([
        false,
        false,
        false,
    ]);
});
