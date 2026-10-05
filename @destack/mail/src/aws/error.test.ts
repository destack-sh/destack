import type { ServiceErrorReport } from "@destack/schema";
import { expect, test } from "@destack/test";
import { SesError, type SesErrorCode } from "./error.ts";

test("map every SES failure code to the service error its caller receives", () => {
    // convert a failure of each code
    const codes: readonly SesErrorCode[] = [
        "INVALID_OPTIONS",
        "CONNECTION",
        "THROTTLED",
        "UNAVAILABLE",
        "EXPIRED",
        "UNAUTHORIZED",
        "REJECTED",
        "UNVERIFIED",
        "SUSPENDED",
        "INVALID_REQUEST",
        "INVALID_RESPONSE",
    ];
    const received = Object.fromEntries(
        codes.map((code) => [code, new SesError(code, "failed").toServiceError()]),
    );

    // receive each code's service error with the failure's message
    const expected: Readonly<Record<SesErrorCode, ServiceErrorReport>> = {
        INVALID_OPTIONS: { code: "INTERNAL_SERVER_ERROR", message: "failed" },
        CONNECTION: { code: "BAD_GATEWAY", message: "failed" },
        THROTTLED: { code: "TOO_MANY_REQUESTS", message: "failed" },
        UNAVAILABLE: { code: "UNAVAILABLE", message: "failed" },
        EXPIRED: { code: "UNAVAILABLE", message: "failed" },
        UNAUTHORIZED: { code: "INTERNAL_SERVER_ERROR", message: "failed" },
        REJECTED: { code: "UNPROCESSABLE_CONTENT", message: "failed" },
        UNVERIFIED: { code: "UNPROCESSABLE_CONTENT", message: "failed" },
        SUSPENDED: { code: "UNAVAILABLE", message: "failed" },
        INVALID_REQUEST: { code: "BAD_REQUEST", message: "failed" },
        INVALID_RESPONSE: { code: "BAD_GATEWAY", message: "failed" },
    };
    expect(received).toEqual(expected);
});
