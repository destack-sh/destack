import type { ServiceErrorReport } from "@destack/schema";
import { expect, test } from "@destack/test";
import { HostError, type HostErrorCode } from "./error.ts";

test("map every host key failure code to the service error its caller receives", () => {
    // convert a failure of each code
    const codes: readonly HostErrorCode[] = ["KEY_UNAVAILABLE", "DECRYPTION_FAILED", "INVALID_KEY"];
    const received = Object.fromEntries(
        codes.map((code) => [code, new HostError(code, "failed").toServiceError()]),
    );

    // receive each code's service error with the failure's message
    const expected: Readonly<Record<HostErrorCode, ServiceErrorReport>> = {
        KEY_UNAVAILABLE: { code: "INTERNAL_SERVER_ERROR", message: "failed" },
        DECRYPTION_FAILED: { code: "INTERNAL_SERVER_ERROR", message: "failed" },
        INVALID_KEY: { code: "INTERNAL_SERVER_ERROR", message: "failed" },
    };
    expect(received).toEqual(expected);
});
