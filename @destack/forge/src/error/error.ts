import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The forge package's failures, each reason with the call error code it answers. */
export const ForgeError = defineErrorFamily({
    reasons: {
        // not found
        BUILD_NOT_FOUND: {},
        GITHUB_WEBHOOK_NOT_FOUND: {},
        PACKAGE_NOT_FOUND: {},
        RELEASE_NOT_FOUND: {},
        TARGET_NOT_FOUND: {},
        UPLOAD_OUTSIDE_ACCOUNT_NOT_FOUND: {},

        // exists
        INSTALLATION_EXISTS: {},
        NPM_NAME_EXISTS: {},
        RELEASE_EXISTS: {},

        // conflict
        BUILD_CONFLICT: {},
        REVISION_CONFLICT: {},

        // invalid
        BUILD_INVALID: {},
        EVENT_INVALID: {},
        MACHINE_REPOSITORY_INVALID: {},
        MANIFEST_INVALID: {},
        PUSH_INVALID: {},
        RELEASE_INVALID: {},
        REPOSITORY_INVALID: {},
        STORAGE_INVALID: {},

        // required
        EVENT_TYPE_REQUIRED: {},
        GITHUB_REPOSITORY_REQUIRED: {},

        // unauthenticated
        CREDENTIAL_UNAUTHENTICATED: {},

        // denied
        BUILTIN_DENIED: {},
        INSTALLATION_DENIED: {},

        // blocked
        CONNECTION_REPLICA_BLOCKED: {},
        CREDENTIAL_BLOCKED: {},
        DEPENDENCY_BLOCKED: {},
        GITHUB_APP_BLOCKED: {},
        GITHUB_IDENTIFIER_BLOCKED: {},
        SECRET_TEXT_BLOCKED: {},
        VERSION_BLOCKED: {},

        // unavailable
        STORAGE_UNAVAILABLE: {},
    },
});

/** A stable cause of forge failures. */
export type ForgeReason = ReasonOf<typeof ForgeError>;
