import type { Failure } from "@destack/error";
import { expect, test } from "@destack/test";
import { UpdateError, type UpdateReason } from "./error.ts";

test("map every update failure reason to the call error its caller receives", () => {
    // convert a failure of each reason
    const reasons: readonly UpdateReason[] = [
        "UPDATE_RUNNING",
        "SESSION_CLOSED",
        "INVALID_REPOSITORY",
        "CACHE_MISMATCH",
        "RELEASE_NOT_FOUND",
        "INVALID_RELEASE",
        "RELEASE_CHANGED",
        "INSTALLATION_CHANGED",
        "VERIFICATION_FAILED",
        "DOWNLOAD_FAILED",
        "UNSAFE_ARCHIVE",
        "DOWNGRADE_REFUSED",
        "ALREADY_INSTALLED",
        "PLATFORM_CHANGED",
        "TARGET_MISMATCH",
        "UNSUPPORTED_TARGET",
        "INVALID_INSTALLATION",
        "ACTIVATION_FAILED",
        "INSTALL_FAILED",
        "DEVELOPMENT_BUILD",
    ];
    const received = Object.fromEntries(
        reasons.map((reason) => [reason, new UpdateError(reason, "failed").toCallError()]),
    );

    // receive each reason's call error with the failure's message
    const expected: Readonly<Record<UpdateReason, Failure>> = {
        UPDATE_RUNNING: {
            code: "CONFLICT",
            reason: "UPDATE_RUNNING",
            domain: "@destack/update",
            message: "failed",
        },
        SESSION_CLOSED: {
            code: "SERVICE_UNAVAILABLE",
            reason: "SESSION_CLOSED",
            domain: "@destack/update",
            message: "failed",
        },
        INVALID_REPOSITORY: {
            code: "BAD_REQUEST",
            reason: "INVALID_REPOSITORY",
            domain: "@destack/update",
            message: "failed",
        },
        CACHE_MISMATCH: {
            code: "CONFLICT",
            reason: "CACHE_MISMATCH",
            domain: "@destack/update",
            message: "failed",
        },
        RELEASE_NOT_FOUND: {
            code: "NOT_FOUND",
            reason: "RELEASE_NOT_FOUND",
            domain: "@destack/update",
            message: "failed",
        },
        INVALID_RELEASE: {
            code: "BAD_GATEWAY",
            reason: "INVALID_RELEASE",
            domain: "@destack/update",
            message: "failed",
        },
        RELEASE_CHANGED: {
            code: "CONFLICT",
            reason: "RELEASE_CHANGED",
            domain: "@destack/update",
            message: "failed",
        },
        INSTALLATION_CHANGED: {
            code: "CONFLICT",
            reason: "INSTALLATION_CHANGED",
            domain: "@destack/update",
            message: "failed",
        },
        VERIFICATION_FAILED: {
            code: "BAD_GATEWAY",
            reason: "VERIFICATION_FAILED",
            domain: "@destack/update",
            message: "failed",
        },
        DOWNLOAD_FAILED: {
            code: "BAD_GATEWAY",
            reason: "DOWNLOAD_FAILED",
            domain: "@destack/update",
            message: "failed",
        },
        UNSAFE_ARCHIVE: {
            code: "BAD_GATEWAY",
            reason: "UNSAFE_ARCHIVE",
            domain: "@destack/update",
            message: "failed",
        },
        DOWNGRADE_REFUSED: {
            code: "CONFLICT",
            reason: "DOWNGRADE_REFUSED",
            domain: "@destack/update",
            message: "failed",
        },
        ALREADY_INSTALLED: {
            code: "CONFLICT",
            reason: "ALREADY_INSTALLED",
            domain: "@destack/update",
            message: "failed",
        },
        PLATFORM_CHANGED: {
            code: "CONFLICT",
            reason: "PLATFORM_CHANGED",
            domain: "@destack/update",
            message: "failed",
        },
        TARGET_MISMATCH: {
            code: "CONFLICT",
            reason: "TARGET_MISMATCH",
            domain: "@destack/update",
            message: "failed",
        },
        UNSUPPORTED_TARGET: {
            code: "BAD_REQUEST",
            reason: "UNSUPPORTED_TARGET",
            domain: "@destack/update",
            message: "failed",
        },
        INVALID_INSTALLATION: {
            code: "PRECONDITION_FAILED",
            reason: "INVALID_INSTALLATION",
            domain: "@destack/update",
            message: "failed",
        },
        ACTIVATION_FAILED: {
            code: "INTERNAL_SERVER_ERROR",
            reason: "ACTIVATION_FAILED",
            domain: "@destack/update",
            message: "failed",
        },
        DEVELOPMENT_BUILD: {
            code: "PRECONDITION_FAILED",
            reason: "DEVELOPMENT_BUILD",
            domain: "@destack/update",
            message: "failed",
        },
        INSTALL_FAILED: {
            code: "INTERNAL_SERVER_ERROR",
            reason: "INSTALL_FAILED",
            domain: "@destack/update",
            message: "failed",
        },
    };
    expect(received).toEqual(expected);
});
