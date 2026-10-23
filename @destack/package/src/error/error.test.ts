import type { Failure } from "@destack/error";
import { expect, test } from "@destack/test";
import { PackageError, type PackageReason, SandboxError } from "./error.ts";

test("map every package failure reason to the call error its caller receives", () => {
    // convert a failure of each reason
    const reasons: readonly PackageReason[] = [
        "INVALID_DEFINITION",
        "UNSUPPORTED_LANGUAGE",
        "UNSUPPORTED_TARGET",
        "INVALID_EXPORT",
        "INVALID_DEPENDENCY",
        "INVALID_FILE",
        "INVALID_INSPECTION",
        "INVALID_BUILD",
    ];
    const received = Object.fromEntries(
        reasons.map((reason) => [reason, new PackageError(reason, "failed").toCallError()]),
    );

    // receive each reason's call error with the failure's message
    const expected: Readonly<Record<PackageReason, Failure>> = {
        INVALID_DEFINITION: {
            code: "BAD_REQUEST",
            reason: "INVALID_DEFINITION",
            domain: "@destack/package",
            message: "failed",
        },
        UNSUPPORTED_LANGUAGE: {
            code: "UNPROCESSABLE_CONTENT",
            reason: "UNSUPPORTED_LANGUAGE",
            domain: "@destack/package",
            message: "failed",
        },
        UNSUPPORTED_TARGET: {
            code: "UNPROCESSABLE_CONTENT",
            reason: "UNSUPPORTED_TARGET",
            domain: "@destack/package",
            message: "failed",
        },
        INVALID_EXPORT: {
            code: "BAD_REQUEST",
            reason: "INVALID_EXPORT",
            domain: "@destack/package",
            message: "failed",
        },
        INVALID_DEPENDENCY: {
            code: "BAD_REQUEST",
            reason: "INVALID_DEPENDENCY",
            domain: "@destack/package",
            message: "failed",
        },
        INVALID_FILE: {
            code: "BAD_REQUEST",
            reason: "INVALID_FILE",
            domain: "@destack/package",
            message: "failed",
        },
        INVALID_INSPECTION: {
            code: "UNPROCESSABLE_CONTENT",
            reason: "INVALID_INSPECTION",
            domain: "@destack/package",
            message: "failed",
        },
        INVALID_BUILD: {
            code: "BAD_REQUEST",
            reason: "INVALID_BUILD",
            domain: "@destack/package",
            message: "failed",
        },
    };
    expect(received).toEqual(expected);
});

test("report a sandbox ability no host grants as content the host cannot process", () => {
    const failure = new SandboxError(
        "ABILITY_UNENFORCEABLE",
        "network",
        "the host cannot limit the network",
    );

    expect(failure.toCallError()).toEqual({
        code: "UNPROCESSABLE_CONTENT",
        reason: "ABILITY_UNENFORCEABLE",
        domain: "@destack/package",
        message: "the host cannot limit the network",
    });
});
