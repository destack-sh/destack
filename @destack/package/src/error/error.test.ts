import type { ServiceErrorReport } from "@destack/schema";
import { expect, test } from "@destack/test";
import { CapabilityError, PackageError, PackageErrorCode } from "./error.ts";

test("map every package failure code to the service error its caller receives", () => {
    // convert a failure of each code
    const received = Object.fromEntries(
        PackageErrorCode.options.map((code) => [
            code,
            new PackageError(code, "failed").toServiceError(),
        ]),
    );

    // receive each code's service error with the failure's message
    const expected: Readonly<Record<PackageErrorCode, ServiceErrorReport>> = {
        INVALID_DEFINITION: { code: "BAD_REQUEST", message: "failed" },
        UNSUPPORTED_LANGUAGE: { code: "UNPROCESSABLE_CONTENT", message: "failed" },
        UNSUPPORTED_TARGET: { code: "UNPROCESSABLE_CONTENT", message: "failed" },
        INVALID_EXPORT: { code: "BAD_REQUEST", message: "failed" },
        INVALID_DEPENDENCY: { code: "BAD_REQUEST", message: "failed" },
        INVALID_FILE: { code: "BAD_REQUEST", message: "failed" },
        INVALID_INSPECTION: { code: "UNPROCESSABLE_CONTENT", message: "failed" },
    };
    expect(received).toEqual(expected);
});

test("report a capability no host grants as unprocessable", () => {
    const refused = [
        new CapabilityError("UNSUPPORTED", "usb", "this host has no USB devices"),
        new CapabilityError("UNENFORCEABLE", "network", "this host cannot limit the network"),
    ];
    expect(refused.map((failure) => failure.toServiceError())).toEqual([
        { code: "UNPROCESSABLE_CONTENT", message: "this host has no USB devices" },
        { code: "UNPROCESSABLE_CONTENT", message: "this host cannot limit the network" },
    ]);
});
