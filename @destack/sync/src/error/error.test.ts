import type { ServiceErrorReport } from "@destack/schema";
import { expect, test } from "@destack/test";
import { SyncError, type SyncErrorCode } from "./error.ts";

test("map every sync failure code to the service error its caller receives", () => {
    // convert a failure of each code
    const codes: readonly SyncErrorCode[] = [
        "OVERLOADED",
        "OVER_CAPACITY",
        "STALE",
        "INVALID_STREAM",
        "INVALID_SCOPE",
        "NOT_FOUND",
        "CYCLE",
    ];
    const received = Object.fromEntries(
        codes.map((code) => [code, new SyncError(code, "failed").toServiceError()]),
    );

    // receive each code's service error with the failure's message
    const expected: Readonly<Record<SyncErrorCode, ServiceErrorReport>> = {
        OVERLOADED: { code: "SERVICE_UNAVAILABLE", message: "failed" },
        OVER_CAPACITY: { code: "UNPROCESSABLE_CONTENT", message: "failed" },
        STALE: { code: "SERVICE_UNAVAILABLE", message: "failed" },
        INVALID_STREAM: { code: "BAD_GATEWAY", message: "failed" },
        INVALID_SCOPE: { code: "UNPROCESSABLE_CONTENT", message: "failed" },
        NOT_FOUND: { code: "NOT_FOUND", message: "failed" },
        CYCLE: { code: "CONFLICT", message: "failed" },
    };
    expect(received).toEqual(expected);
});
