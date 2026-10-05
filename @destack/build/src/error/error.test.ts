import type { ServiceErrorReport } from "@destack/schema";
import { expect, test } from "@destack/test";
import { BuildError, BuildErrorCode } from "./error.ts";

test("map every build failure code to the service error its caller receives, with its status", () => {
    // convert a failure of each code
    const failures = BuildErrorCode.options.map((code) => new BuildError(code, "failed"));
    const received = Object.fromEntries(
        failures.map((failure) => [failure.code, failure.toServiceError()]),
    );

    // receive each code's service error with the failure's message, at 422 Unprocessable Content
    const expected: Readonly<Record<BuildErrorCode, ServiceErrorReport>> = {
        INSPECTION_FAILED: { code: "UNPROCESSABLE_CONTENT", message: "failed" },
        BUILD_FAILED: { code: "UNPROCESSABLE_CONTENT", message: "failed" },
    };
    expect([received, failures.map((failure) => failure.status)]).toEqual([expected, [422, 422]]);
});
