import type { ServiceErrorReport } from "@destack/schema";
import { expect, test } from "@destack/test";
import { MimeError, type MimeErrorCode } from "./error.ts";

test("map every message composition failure code to the service error its caller receives", () => {
    // convert a failure of each code
    const codes: readonly MimeErrorCode[] = [
        "INVALID_ADDRESS",
        "INVALID_HEADER",
        "INVALID_KEY",
        "INVALID_DATE",
    ];
    const received = Object.fromEntries(
        codes.map((code) => [code, new MimeError(code, "failed").toServiceError()]),
    );

    // receive each code's service error with the failure's message
    const expected: Readonly<Record<MimeErrorCode, ServiceErrorReport>> = {
        INVALID_ADDRESS: { code: "BAD_REQUEST", message: "failed" },
        INVALID_HEADER: { code: "BAD_REQUEST", message: "failed" },
        INVALID_KEY: { code: "BAD_REQUEST", message: "failed" },
        INVALID_DATE: { code: "BAD_REQUEST", message: "failed" },
    };
    expect(received).toEqual(expected);
});
