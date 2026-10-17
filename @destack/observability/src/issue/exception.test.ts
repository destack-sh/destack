import { expect, test } from "@destack/test";
import type { StackFrame } from "@destack/package/stack";
import { Exception } from "./exception.ts";

/** An in-app frame running a symbol. */
const frame = (moniker: string, line = 1): StackFrame => ({
    file: "a.js",
    line,
    column: 1,
    path: "src/note.ts",
    isInApp: true,
    moniker,
});

/** A range error on a page with frames and a requested grouping. */
const exception = (
    frames: readonly StackFrame[],
    message = "page 9",
    grouping?: readonly string[],
): Exception =>
    new Exception({
        errorType: "RangeError",
        type: "RangeError",
        message,
        stacktrace: undefined,
        frames,
        isEscaped: false,
        level: "error",
        grouping,
    });

test("fingerprint by error type and in-app symbols across lines and recursion, else by message, unless the capture asks for a grouping", async () => {
    const symbols = [frame("#Note.rename"), frame("#check")];
    const moved = [frame("#Note.rename", 40), frame("#Note.rename", 41), frame("#check", 80)];
    const dependency = [{ file: "b.js", line: 1, column: 1, isInApp: false }];

    // keep one group across moved lines and recursion, split on another symbol or message
    const [same, recursed, other, byMessage, otherMessage] = await Promise.all([
        exception(symbols).fingerprint(),
        exception(moved).fingerprint(),
        exception([frame("#archive")]).fingerprint(),
        exception(dependency).fingerprint(),
        exception(dependency, "page 10").fingerprint(),
    ]);
    expect([recursed === same, other === same, otherMessage === byMessage]).toEqual([
        true,
        false,
        false,
    ]);

    // replace or extend the default grouping as the capture asks
    const [requested, extended, tenant] = await Promise.all([
        exception(symbols, "page 9", ["quota"]).fingerprint(),
        exception(symbols, "page 9", ["{{ default }}", "tenant-a"]).fingerprint(),
        exception(symbols, "page 9", ["{{ default }}", "tenant-b"]).fingerprint(),
    ]);
    expect([requested === same, extended === same, extended === tenant]).toEqual([
        false,
        false,
        false,
    ]);
});
