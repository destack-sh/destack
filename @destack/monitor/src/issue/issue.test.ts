import { expect, test } from "@destack/test";
import type { Entry } from "../entry/index.ts";
import { Exception } from "./exception.ts";
import { fingerprint } from "./fingerprint.ts";

/** An exception record with attributes. */
function record(attributes: Entry["attributes"], severity = 17): Entry {
    return {
        kind: "log",
        name: "exception",
        time: 1,
        source: { name: "@example/notes", version: "2026.10.0" },
        status: "unset",
        severity,
        attributes,
    };
}

/** An in-app frame running a symbol. */
const frame = (moniker: string, line = 1) => ({
    file: "a.js",
    line,
    column: 1,
    path: "src/note.ts",
    isInApp: true,
    moniker,
});

test("read exception records with their error type and level, and ignore other entries", () => {
    expect([
        Exception.of(
            record({
                "exception.type": "RangeError",
                "exception.message": "page 9",
                "exception.escaped": true,
            }),
        ),
        Exception.of(
            record(
                { "exception.message": "quota nearly reached", "exception.fingerprint": ["quota"] },
                13,
            ),
        ),
        Exception.of({ ...record({ "exception.message": "x" }), name: "request" }),
    ]).toEqual([
        {
            errorType: "RangeError",
            type: "RangeError",
            message: "page 9",
            isEscaped: true,
            level: "error",
        },
        {
            errorType: "Error",
            message: "quota nearly reached",
            isEscaped: false,
            level: "warning",
            fingerprint: ["quota"],
        },
        undefined,
    ]);
});

test("expect service errors refusing their caller, and count every other failure as unexpected", () => {
    expect(
        ["NOT_FOUND", "CONFLICT", "INTERNAL_SERVER_ERROR", "TypeError", "ENOENT"].map((errorType) =>
            Exception.isExpected({ errorType }),
        ),
    ).toEqual([true, true, false, false, false]);
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
