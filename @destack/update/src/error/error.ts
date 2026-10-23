import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The update package's failures, each reason with the call error code it answers. */
export const UpdateError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    INVALID_METADATA: { code: "BAD_REQUEST" },
    INVALID_OPTIONS: { code: "BAD_REQUEST" },
    INVALID_REPOSITORY: { code: "BAD_REQUEST" },
    INVALID_SIGNING_KEY: { code: "BAD_REQUEST" },
    UNSUPPORTED_TARGET: { code: "BAD_REQUEST" },

    // not found
    RELEASE_NOT_FOUND: { code: "NOT_FOUND" },

    // conflict
    ALREADY_INSTALLED: { code: "CONFLICT" },
    CACHE_MISMATCH: { code: "CONFLICT" },
    DOWNGRADE_REFUSED: { code: "CONFLICT" },
    INSTALLATION_CHANGED: { code: "CONFLICT" },
    PLATFORM_CHANGED: { code: "CONFLICT" },
    RELEASE_CHANGED: { code: "CONFLICT" },
    TARGET_MISMATCH: { code: "CONFLICT" },
    UPDATE_RUNNING: { code: "CONFLICT" },

    // precondition failed
    DEVELOPMENT_BUILD: { code: "PRECONDITION_FAILED" },
    INSTALLATION_METHOD_MISMATCH: { code: "PRECONDITION_FAILED" },

    // internal server error
    ACTIVATION_FAILED: { code: "INTERNAL_SERVER_ERROR" },
    INSTALL_FAILED: { code: "INTERNAL_SERVER_ERROR" },
    INVALID_RECORD: { code: "INTERNAL_SERVER_ERROR" },

    // bad gateway
    DOWNLOAD_FAILED: { code: "BAD_GATEWAY" },
    INVALID_RELEASE: { code: "BAD_GATEWAY" },
    UNSAFE_ARCHIVE: { code: "BAD_GATEWAY" },
    VERIFICATION_FAILED: { code: "BAD_GATEWAY" },

    // service unavailable
    SESSION_CLOSED: { code: "SERVICE_UNAVAILABLE" },
});

/** A stable cause of update failures. */
export type UpdateReason = ReasonOf<typeof UpdateError>;
