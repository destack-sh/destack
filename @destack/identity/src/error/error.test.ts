import type { Failure } from "@destack/error";
import { expect, test } from "@destack/test";
import { IdentityError, type IdentityReason } from "./error.ts";

test("map every identity failure reason to the call error its caller receives", () => {
    // convert a failure of each reason
    const reasons: readonly IdentityReason[] = [
        "INVALID_PUBLIC_KEY",
        "INVALID_PROOF",
        "INVALID_OPERATION",
        "INVALID_ROOT_KEY",
        "KEY_UNAVAILABLE",
        "DECRYPTION_FAILED",
    ];
    const received = Object.fromEntries(
        reasons.map((reason) => [reason, new IdentityError(reason, "failed").toCallError()]),
    );

    // receive each reason's call error with the failure's message
    const expected: Readonly<Record<IdentityReason, Failure>> = {
        INVALID_PUBLIC_KEY: {
            code: "BAD_REQUEST",
            reason: "INVALID_PUBLIC_KEY",
            domain: "@destack/identity",
            message: "failed",
        },
        INVALID_PROOF: {
            code: "BAD_REQUEST",
            reason: "INVALID_PROOF",
            domain: "@destack/identity",
            message: "failed",
        },
        INVALID_OPERATION: {
            code: "BAD_REQUEST",
            reason: "INVALID_OPERATION",
            domain: "@destack/identity",
            message: "failed",
        },
        INVALID_ROOT_KEY: {
            code: "INTERNAL_SERVER_ERROR",
            reason: "INVALID_ROOT_KEY",
            domain: "@destack/identity",
            message: "failed",
        },
        KEY_UNAVAILABLE: {
            code: "INTERNAL_SERVER_ERROR",
            reason: "KEY_UNAVAILABLE",
            domain: "@destack/identity",
            message: "failed",
        },
        DECRYPTION_FAILED: {
            code: "INTERNAL_SERVER_ERROR",
            reason: "DECRYPTION_FAILED",
            domain: "@destack/identity",
            message: "failed",
        },
    };
    expect(received).toEqual(expected);
});
