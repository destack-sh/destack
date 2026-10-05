import type { ServiceErrorReport } from "@destack/schema";
import { expect, test } from "@destack/test";
import { SmtpError, type SmtpErrorCode } from "./error.ts";

test("map every SMTP failure code to the service error its caller receives", () => {
    // convert a failure of each code
    const codes: readonly SmtpErrorCode[] = [
        "INVALID_OPTIONS",
        "INSECURE",
        "INVALID_ENVELOPE",
        "CONNECTION",
        "TIMEOUT",
        "PROTOCOL",
        "UNSUPPORTED",
        "REJECTED",
    ];
    const received = Object.fromEntries(
        codes.map((code) => [code, new SmtpError(code, "failed").toServiceError()]),
    );

    // receive each code's service error with the failure's message
    const expected: Readonly<Record<SmtpErrorCode, ServiceErrorReport>> = {
        INVALID_OPTIONS: { code: "INTERNAL_SERVER_ERROR", message: "failed" },
        INSECURE: { code: "INTERNAL_SERVER_ERROR", message: "failed" },
        INVALID_ENVELOPE: { code: "BAD_REQUEST", message: "failed" },
        CONNECTION: { code: "BAD_GATEWAY", message: "failed" },
        TIMEOUT: { code: "GATEWAY_TIMEOUT", message: "failed" },
        PROTOCOL: { code: "BAD_GATEWAY", message: "failed" },
        UNSUPPORTED: { code: "BAD_GATEWAY", message: "failed" },
        REJECTED: { code: "UNPROCESSABLE_CONTENT", message: "failed" },
    };
    expect(received).toEqual(expected);
});
