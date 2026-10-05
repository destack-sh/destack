import type { ServiceErrorReport } from "@destack/schema";
import { expect, test } from "@destack/test";
import { CheckError, type CheckErrorCode } from "./error.ts";

test("map every check failure code to the service error its caller receives", () => {
    // convert a failure of each code
    const codes: readonly CheckErrorCode[] = ["TOOL", "CONFIGURATION", "LANGUAGE"];
    const received = Object.fromEntries(
        codes.map((code) => [code, new CheckError(code, "failed").toServiceError()]),
    );

    // receive each code's service error with the failure's message
    const expected: Readonly<Record<CheckErrorCode, ServiceErrorReport>> = {
        TOOL: { code: "INTERNAL_SERVER_ERROR", message: "failed" },
        CONFIGURATION: { code: "BAD_REQUEST", message: "failed" },
        LANGUAGE: { code: "UNPROCESSABLE_CONTENT", message: "failed" },
    };
    expect(received).toEqual(expected);
});
