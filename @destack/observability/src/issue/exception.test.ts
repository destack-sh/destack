import { testHost } from "@destack/test/host";
import { expect, refusal, test } from "@destack/test";
import type { StackFrame } from "@destack/package/stack";
import { FRAMES_ATTRIBUTE } from "@destack/telemetry/otlp";
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
        exception(symbols).fingerprint(testHost().crypto),
        exception(moved).fingerprint(testHost().crypto),
        exception([frame("#archive")]).fingerprint(testHost().crypto),
        exception(dependency).fingerprint(testHost().crypto),
        exception(dependency, "page 10").fingerprint(testHost().crypto),
    ]);
    expect([recursed === same, other === same, otherMessage === byMessage]).toEqual([
        true,
        false,
        false,
    ]);

    // replace or extend the default grouping as the capture asks
    const [requested, extended, tenant] = await Promise.all([
        exception(symbols, "page 9", ["quota"]).fingerprint(testHost().crypto),
        exception(symbols, "page 9", ["{{ default }}", "tenant-a"]).fingerprint(testHost().crypto),
        exception(symbols, "page 9", ["{{ default }}", "tenant-b"]).fingerprint(testHost().crypto),
    ]);
    expect([requested === same, extended === same, extended === tenant]).toEqual([
        false,
        false,
        false,
    ]);
});

/** Read an exception whose capture asks for a grouping. */
const grouped = (fingerprint: string) => () =>
    Exception.read({ "exception.message": "page 9", "exception.fingerprint": fingerprint }, 17);

/** Read an exception with an emitter's symbolicated frames as text. */
const framed = (frames: string) => () =>
    Exception.read({ "exception.message": "page 9", [FRAMES_ATTRIBUTE]: frames }, 17);

test("refuse symbolicated frames that are not JSON, or JSON but not a list of frames", async () => {
    expect([await refusal(framed("frames")), await refusal(framed('[{"line":"one"}]'))]).toEqual([
        ["BAD_REQUEST", "FRAMES_INVALID", "the exception's frames are not JSON"],
        ["BAD_REQUEST", "FRAMES_INVALID", "the exception's frames are not a list of frames"],
    ]);
});

test("refuse a requested grouping that is not a list", async () => {
    // read a key's grouping that is not JSON, and one that is JSON but not a list
    expect([await refusal(grouped("quota")), await refusal(grouped('{"quota":1}'))]).toEqual([
        ["BAD_REQUEST", "FINGERPRINT_INVALID", "exception.fingerprint is not valid JSON"],
        ["BAD_REQUEST", "FINGERPRINT_INVALID", "exception.fingerprint is not a list"],
    ]);
});
