import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The update package's failures, each reason with the call error code it answers. */
export const UpdateError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    CHANNEL_UNKNOWN: { code: "BAD_REQUEST" },
    ENVIRONMENT_INVALID: { code: "BAD_REQUEST" },
    METADATA_INVALID: { code: "BAD_REQUEST" },
    OPTIONS_INVALID: { code: "BAD_REQUEST" },
    REPOSITORY_INVALID: { code: "BAD_REQUEST" },
    SIGNING_KEY_INVALID: { code: "BAD_REQUEST" },
    TARGET_UNSUPPORTED: { code: "BAD_REQUEST" },

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
    DESKTOP_UNAVAILABLE: { code: "PRECONDITION_FAILED" },
    DEVELOPMENT_BUILD: { code: "PRECONDITION_FAILED" },
    INSTALLATION_METHOD_MISMATCH: { code: "PRECONDITION_FAILED" },
    INSTALLATION_MISMATCH: { code: "PRECONDITION_FAILED" },

    // internal server error
    ACTIVATION_FAILED: { code: "INTERNAL_SERVER_ERROR" },
    COMMAND_FAILED: { code: "INTERNAL_SERVER_ERROR" },
    FILE_SYSTEM_FAILED: { code: "INTERNAL_SERVER_ERROR" },
    INSTALL_FAILED: { code: "INTERNAL_SERVER_ERROR" },
    RECORD_INVALID: { code: "INTERNAL_SERVER_ERROR" },

    // bad gateway
    DOWNLOAD_FAILED: { code: "BAD_GATEWAY" },
    RELEASE_INVALID: { code: "BAD_GATEWAY" },
    UNSAFE_ARCHIVE: { code: "BAD_GATEWAY" },
    VERIFICATION_FAILED: { code: "BAD_GATEWAY" },

    // service unavailable
    SESSION_CLOSED: { code: "SERVICE_UNAVAILABLE" },
});

/** A stable cause of update failures. */
export type UpdateReason = ReasonOf<typeof UpdateError>;
