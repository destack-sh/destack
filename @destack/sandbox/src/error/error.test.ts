import type { ServiceErrorReport } from "@destack/error";
import { expect, test } from "@destack/test";
import { SandboxError, type SandboxErrorCode } from "./error.ts";

test("map every sandbox failure code to the service error its caller receives", () => {
    // convert a failure of each code
    const codes: readonly SandboxErrorCode[] = ["UNSUPPORTED", "START_FAILED", "STOP_FAILED"];
    const received = Object.fromEntries(
        codes.map((code) => [code, new SandboxError(code, "failed").toServiceError()]),
    );

    // receive each code's service error with the failure's message
    const expected: Readonly<Record<SandboxErrorCode, ServiceErrorReport>> = {
        UNSUPPORTED: { code: "NOT_IMPLEMENTED", message: "failed" },
        START_FAILED: { code: "INTERNAL_SERVER_ERROR", message: "failed" },
        STOP_FAILED: { code: "INTERNAL_SERVER_ERROR", message: "failed" },
    };
    expect(received).toEqual(expected);
});
