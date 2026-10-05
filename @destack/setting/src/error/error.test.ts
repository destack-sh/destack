import type { ServiceErrorReport } from "@destack/schema";
import { expect, test } from "@destack/test";
import { SettingError, type SettingErrorCode } from "./error.ts";

test("map every setting failure code to the service error its caller receives", () => {
    // convert a failure of each code
    const codes: readonly SettingErrorCode[] = [
        "UNDECLARED",
        "INVALID_VALUE",
        "INVALID_PLACEMENT",
        "CONFLICT",
    ];
    const received = Object.fromEntries(
        codes.map((code) => [code, new SettingError(code, "failed").toServiceError()]),
    );

    // receive each code's service error with the failure's message
    const expected: Readonly<Record<SettingErrorCode, ServiceErrorReport>> = {
        UNDECLARED: { code: "NOT_FOUND", message: "failed" },
        INVALID_VALUE: { code: "BAD_REQUEST", message: "failed" },
        INVALID_PLACEMENT: { code: "BAD_REQUEST", message: "failed" },
        CONFLICT: { code: "CONFLICT", message: "failed" },
    };
    expect(received).toEqual(expected);
});
