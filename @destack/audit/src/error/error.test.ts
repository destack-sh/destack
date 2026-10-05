import type { ServiceErrorReport } from "@destack/schema";
import { expect, test } from "@destack/test";
import { AuditError, type AuditErrorCode } from "./error.ts";

test("map every audit failure code to the service error its caller receives", () => {
    // convert a failure of each code
    const codes: readonly AuditErrorCode[] = [
        "INVALID_EVENT",
        "CONFLICT",
        "FORBIDDEN",
        "NOT_FOUND",
        "UNAVAILABLE",
    ];
    const received = Object.fromEntries(
        codes.map((code) => [code, new AuditError(code, "failed").toServiceError()]),
    );

    // receive each code's service error with the failure's message
    const expected: Readonly<Record<AuditErrorCode, ServiceErrorReport>> = {
        INVALID_EVENT: { code: "BAD_REQUEST", message: "failed" },
        CONFLICT: { code: "CONFLICT", message: "failed" },
        FORBIDDEN: { code: "FORBIDDEN", message: "failed" },
        NOT_FOUND: { code: "NOT_FOUND", message: "failed" },
        UNAVAILABLE: { code: "UNAVAILABLE", message: "failed" },
    };
    expect(received).toEqual(expected);
});
