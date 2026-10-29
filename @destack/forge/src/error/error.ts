import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The forge package's failures, each reason with the call error code it answers. */
export const ForgeError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    ANONYMOUS_PUSH: { code: "BAD_REQUEST" },
    BUILD_ONLY_OUTPUT: { code: "BAD_REQUEST" },
    DEPENDENCY_CONFLICTING: { code: "BAD_REQUEST" },
    CREDENTIAL_MISMATCH: { code: "BAD_REQUEST" },
    PATH_DUPLICATE: { code: "BAD_REQUEST" },
    FOREIGN_MANIFEST: { code: "BAD_REQUEST" },
    IMMOVABLE_REPOSITORY: { code: "BAD_REQUEST" },
    DEPENDENCY_INVALID: { code: "BAD_REQUEST" },
    ENCODING_INVALID: { code: "BAD_REQUEST" },
    EXPORT_INVALID: { code: "BAD_REQUEST" },
    MANIFEST_INVALID: { code: "BAD_REQUEST" },
    NPM_NAME_INVALID: { code: "BAD_REQUEST" },
    ORIGIN_INVALID: { code: "BAD_REQUEST" },
    TAG_INVALID: { code: "BAD_REQUEST" },
    MACHINE_REPOSITORY: { code: "BAD_REQUEST" },
    MANIFEST_MISMATCH: { code: "BAD_REQUEST" },
    BIN_MISSING: { code: "BAD_REQUEST" },
    DECLARATION_MISSING: { code: "BAD_REQUEST" },
    EVENT_TYPE_MISSING: { code: "BAD_REQUEST" },
    EXPORT_SOURCE_MISSING: { code: "BAD_REQUEST" },
    PACKAGE_JSON_MISSING: { code: "BAD_REQUEST" },
    GITHUB_REPOSITORY_REQUIRED: { code: "BAD_REQUEST" },
    RUNTIME_OUTPUT_MISSING: { code: "BAD_REQUEST" },
    PACKAGE_ID_MISMATCH: { code: "BAD_REQUEST" },
    RESERVED_MANIFEST: { code: "BAD_REQUEST" },
    OUTPUT_UNKNOWN: { code: "BAD_REQUEST" },
    UNSUBSCRIBED_EVENT: { code: "BAD_REQUEST" },

    // unauthorized
    CREDENTIAL_MISSING: { code: "UNAUTHORIZED" },

    // forbidden
    INSTALLATION_NOT_CLAIMED: { code: "FORBIDDEN" },

    // not found
    ARCHIVE_NOT_FOUND: { code: "NOT_FOUND" },
    BUILD_NOT_FOUND: { code: "NOT_FOUND" },
    BUILD_NOT_PUSHED: { code: "NOT_FOUND" },
    GITHUB_WEBHOOK_MISSING: { code: "NOT_FOUND" },
    PUBLISHER_MISSING: { code: "NOT_FOUND" },
    RELEASES_MISSING: { code: "NOT_FOUND" },
    PACKAGE_NOT_FOUND: { code: "NOT_FOUND" },
    RELEASE_NOT_FOUND: { code: "NOT_FOUND" },
    UNPUBLISHED_TARGET: { code: "NOT_FOUND" },
    UPLOAD_OUTSIDE_PACKAGE: { code: "NOT_FOUND" },
    VERSION_NOT_FOUND: { code: "NOT_FOUND" },

    // conflict
    INSTALLATION_CLAIMED: { code: "CONFLICT" },
    FOREIGN_BUILD: { code: "CONFLICT" },
    NPM_NAME_TAKEN: { code: "CONFLICT" },
    REVISION_STALE: { code: "CONFLICT" },
    UNCOMMITTED_CHANGES: { code: "CONFLICT" },
    VERSION_ORDER: { code: "CONFLICT" },
    VERSION_PUBLISHED: { code: "CONFLICT" },

    // precondition failed
    CREDENTIAL_INVALID: { code: "PRECONDITION_FAILED" },
    GITHUB_IDENTIFIER_MISSING: { code: "PRECONDITION_FAILED" },
    SECRET_TEXT_MISSING: { code: "PRECONDITION_FAILED" },
    CONNECTION_REPLICA_MISSING: { code: "PRECONDITION_FAILED" },
    GITHUB_APP_MISSING: { code: "PRECONDITION_FAILED" },

    // internal server error
    GIT_FAILED: { code: "INTERNAL_SERVER_ERROR" },

    // bad gateway
    ARTIFACT_NAME_MISMATCH: { code: "BAD_GATEWAY" },
    TOKEN_EXPIRY_INVALID: { code: "BAD_GATEWAY" },
    UPLOAD_PACK_MISSING: { code: "BAD_GATEWAY" },
    REMOTE_FAILED: { code: "BAD_GATEWAY" },
    ADVERTISEMENT_UNEXPECTED: { code: "BAD_GATEWAY" },
    PKT_LINE_UNEXPECTED: { code: "BAD_GATEWAY" },

    // service unavailable
    FOREIGN_STORAGE: { code: "SERVICE_UNAVAILABLE" },
    STORAGE_UNAVAILABLE: { code: "SERVICE_UNAVAILABLE" },
});

/** A stable cause of forge failures. */
export type ForgeReason = ReasonOf<typeof ForgeError>;
