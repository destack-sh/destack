import type { Failure } from "@destack/error";
import { expect, test } from "@destack/test";
import { SandboxError, type SandboxReason } from "./error.ts";

test("map every sandbox failure reason to the call error its caller receives", () => {
    // convert a failure of each reason
    const reasons: readonly SandboxReason[] = ["UNSUPPORTED", "START_FAILED", "STOP_FAILED"];
    const received = Object.fromEntries(
        reasons.map((reason) => [reason, new SandboxError(reason, "failed").toCallError()]),
    );

    // receive each reason's call error with the failure's message
    const expected: Readonly<Record<SandboxReason, Failure>> = {
        UNSUPPORTED: {
            code: "NOT_IMPLEMENTED",
            reason: "UNSUPPORTED",
            domain: "@destack/sandbox",
            message: "failed",
        },
        START_FAILED: {
            code: "INTERNAL_SERVER_ERROR",
            reason: "START_FAILED",
            domain: "@destack/sandbox",
            message: "failed",
        },
        STOP_FAILED: {
            code: "INTERNAL_SERVER_ERROR",
            reason: "STOP_FAILED",
            domain: "@destack/sandbox",
            message: "failed",
        },
    };
    expect(received).toEqual(expected);
});
