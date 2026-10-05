import type { ServiceErrorReport } from "@destack/schema";
import { expect, test } from "@destack/test";
import { ObjectError, type ObjectErrorCode } from "./error.ts";

test("map every object failure code to the service error its caller receives", () => {
    // convert a failure of each code
    const codes: readonly ObjectErrorCode[] = [
        "INVALID_DECLARATION",
        "UNSUPPORTED_DECLARATION",
        "CYCLIC_DECLARATION",
    ];
    const received = Object.fromEntries(
        codes.map((code) => [code, new ObjectError(code, "failed").toServiceError()]),
    );

    // receive each code's service error with the failure's message
    const expected: Readonly<Record<ObjectErrorCode, ServiceErrorReport>> = {
        INVALID_DECLARATION: { code: "INTERNAL_SERVER_ERROR", message: "failed" },
        UNSUPPORTED_DECLARATION: { code: "INTERNAL_SERVER_ERROR", message: "failed" },
        CYCLIC_DECLARATION: { code: "INTERNAL_SERVER_ERROR", message: "failed" },
    };
    expect(received).toEqual(expected);
});
