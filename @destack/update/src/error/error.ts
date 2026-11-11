import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The update package's failures, each reason with the call error code it answers. */
export const UpdateError = defineErrorFamily(import.meta.destack.package, {
    // not found
    BLOB_NOT_FOUND: {},
    METADATA_NOT_FOUND: {},
    RELEASE_NOT_FOUND: {},

    // exists
    RELEASE_EXISTS: {},

    // conflict
    INSTALLATION_CONFLICT: {},
    RELEASE_CONFLICT: {},
    UPDATE_CONFLICT: {},

    // invalid
    ENVIRONMENT_INVALID: {},
    METADATA_INVALID: {},
    OPTIONS_INVALID: {},
    REPOSITORY_INVALID: {},
    SIGNING_KEY_INVALID: {},
    STAGE_INVALID: {},
    TARGET_INVALID: {},

    // blocked
    CACHE_BLOCKED: {},
    DESKTOP_BLOCKED: {},
    DEVELOPMENT_BUILD_BLOCKED: {},
    INSTALLATION_BLOCKED: {},
    INSTALLATION_METHOD_BLOCKED: {},
    PLATFORM_BLOCKED: {},
    RELEASE_BLOCKED: {},
    TARGET_BLOCKED: {},

    // unavailable
    SESSION_UNAVAILABLE: {},

    // unreachable
    DOWNLOAD_UNREACHABLE: {},

    // broken
    ACTIVATION_BROKEN: {},
    CLIENT_BROKEN: {},
    COMMAND_BROKEN: {},
    FILE_SYSTEM_BROKEN: {},
    INSTALL_BROKEN: {},
    MANIFEST_BROKEN: {},
    RECORD_BROKEN: {},
    RELEASE_BROKEN: {},
    VERIFICATION_BROKEN: {},
});

/** A stable cause of update failures. */
export type UpdateReason = ReasonOf<typeof UpdateError>;
