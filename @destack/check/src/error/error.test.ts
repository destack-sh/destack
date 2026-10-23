import type { Failure } from "@destack/error";
import { expect, test } from "@destack/test";
import { CheckError, type CheckReason } from "./error.ts";

test("map every check failure reason to the call error its caller receives", () => {
    // convert a failure of each reason
    const reasons: readonly CheckReason[] = [
        "TOOL_FAILED",
        "INVALID_CONFIGURATION",
        "LANGUAGE_SERVICE_FAILED",
    ];
    const received = Object.fromEntries(
        reasons.map((reason) => [reason, new CheckError(reason, "failed").toCallError()]),
    );

    // receive each reason's call error with the failure's message
    const expected: Readonly<Record<CheckReason, Failure>> = {
        TOOL_FAILED: {
            code: "INTERNAL_SERVER_ERROR",
            reason: "TOOL_FAILED",
            domain: "@destack/check",
            message: "failed",
        },
        INVALID_CONFIGURATION: {
            code: "BAD_REQUEST",
            reason: "INVALID_CONFIGURATION",
            domain: "@destack/check",
            message: "failed",
        },
        LANGUAGE_SERVICE_FAILED: {
            code: "INTERNAL_SERVER_ERROR",
            reason: "LANGUAGE_SERVICE_FAILED",
            domain: "@destack/check",
            message: "failed",
        },
    };
    expect(received).toEqual(expected);
});
