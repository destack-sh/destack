import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The git package's failures, each reason with the call error code it answers. */
export const GitError = defineErrorFamily(import.meta.destack.package, {
    // internal server error
    GIT_FAILED: { code: "INTERNAL_SERVER_ERROR" },

    // bad gateway
    ADVERTISEMENT_UNEXPECTED: { code: "BAD_GATEWAY" },
    ARTIFACT_NAME_MISMATCH: { code: "BAD_GATEWAY" },
    PKT_LINE_UNEXPECTED: { code: "BAD_GATEWAY" },
    REMOTE_FAILED: { code: "BAD_GATEWAY" },
    TOKEN_EXPIRY_INVALID: { code: "BAD_GATEWAY" },
    UPLOAD_PACK_MISSING: { code: "BAD_GATEWAY" },
});

/** A stable cause of git failures. */
export type GitReason = ReasonOf<typeof GitError>;
