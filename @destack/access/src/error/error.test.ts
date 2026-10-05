import type { ServiceErrorReport } from "@destack/schema";
import { expect, test } from "@destack/test";
import { AccessError, type AccessErrorCode } from "./error.ts";

test("map every access failure code to the service error its caller receives", () => {
    // convert a failure of each code
    const codes: readonly AccessErrorCode[] = [
        "INVALID_DECLARATION",
        "INVALID_CONTEXT",
        "NOT_FOUND",
        "FORBIDDEN",
        "INSUFFICIENT_AUTHENTICATION",
        "CONFLICT",
        "STALE",
    ];
    const received = Object.fromEntries(
        codes.map((code) => [code, new AccessError(code, "failed").toServiceError()]),
    );

    // receive each code's service error with the failure's message
    const expected: Readonly<Record<AccessErrorCode, ServiceErrorReport>> = {
        INVALID_DECLARATION: { code: "INTERNAL_SERVER_ERROR", message: "failed" },
        INVALID_CONTEXT: { code: "FORBIDDEN", message: "failed" },
        NOT_FOUND: { code: "NOT_FOUND", message: "failed" },
        FORBIDDEN: { code: "FORBIDDEN", message: "failed" },
        INSUFFICIENT_AUTHENTICATION: { code: "INSUFFICIENT_AUTHENTICATION", message: "failed" },
        CONFLICT: { code: "CONFLICT", message: "failed" },
        STALE: { code: "SERVICE_UNAVAILABLE", message: "failed" },
    };
    expect(received).toEqual(expected);
});

test("challenge a caller with the step-up an insufficient authentication names", () => {
    const refused = new AccessError("INSUFFICIENT_AUTHENTICATION", "authenticate again", {
        stepUp: { assurance: 3 },
    });
    expect(refused.toServiceError()).toEqual({
        code: "INSUFFICIENT_AUTHENTICATION",
        message: "authenticate again",
        data: { assurance: 3 },
    });
});
