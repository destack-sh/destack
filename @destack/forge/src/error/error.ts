import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The forge package's failures, each reason with the call error code it answers. */
export const ForgeError = defineErrorFamily({
    reasons: {
        // not found
        BUILD_NOT_FOUND: {},
        PACKAGE_NOT_FOUND: {},
        RELEASE_NOT_FOUND: {},
        REPOSITORY_NOT_FOUND: {},
        TARGET_NOT_FOUND: {},
        UPLOAD_OUTSIDE_ACCOUNT_NOT_FOUND: {},

        // exists
        NPM_NAME_EXISTS: {},
        RELEASE_EXISTS: {},

        // conflict
        REVISION_CONFLICT: {},

        // invalid
        BUILD_INVALID: {},
        MACHINE_REPOSITORY_INVALID: {},
        PUSH_INVALID: {},
        REPOSITORY_INVALID: {},
        STORAGE_INVALID: {},

        // unauthenticated
        CREDENTIAL_UNAUTHENTICATED: {},

        // denied
        BUILTIN_DENIED: {},

        // blocked
        CREDENTIAL_BLOCKED: {},
        DEPENDENCY_BLOCKED: {},
        SECRET_TEXT_BLOCKED: {},
        VERSION_BLOCKED: {},

        // unavailable
        STORAGE_UNAVAILABLE: {},
    },
});

/** A stable cause of forge failures. */
export type ForgeReason = ReasonOf<typeof ForgeError>;
