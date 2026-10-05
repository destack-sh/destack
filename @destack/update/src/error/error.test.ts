import type { ServiceErrorReport } from "@destack/schema";
import { expect, test } from "@destack/test";
import { UpdateError, type UpdateErrorCode } from "./error.ts";

test("map every update failure code to the service error its caller receives", () => {
    // convert a failure of each code
    const codes: readonly UpdateErrorCode[] = [
        "BUSY",
        "CLOSED",
        "RELEASE",
        "REPOSITORY",
        "DOWNLOAD",
        "ACTIVATION",
        "INSTALL",
    ];
    const received = Object.fromEntries(
        codes.map((code) => [code, new UpdateError(code, "failed").toServiceError()]),
    );

    // receive each code's service error with the failure's message
    const expected: Readonly<Record<UpdateErrorCode, ServiceErrorReport>> = {
        BUSY: { code: "CONFLICT", message: "failed" },
        CLOSED: { code: "UNAVAILABLE", message: "failed" },
        RELEASE: { code: "BAD_GATEWAY", message: "failed" },
        REPOSITORY: { code: "BAD_GATEWAY", message: "failed" },
        DOWNLOAD: { code: "BAD_GATEWAY", message: "failed" },
        ACTIVATION: { code: "INTERNAL_SERVER_ERROR", message: "failed" },
        INSTALL: { code: "INTERNAL_SERVER_ERROR", message: "failed" },
    };
    expect(received).toEqual(expected);
});
