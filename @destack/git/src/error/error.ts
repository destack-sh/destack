import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The git package's failures, each reason with the call error code it answers. */
export const GitError = defineErrorFamily({
    reasons: {
        // unreachable
        ADVERTISEMENT_UNREACHABLE: {},
        ARTIFACT_NAME_UNREACHABLE: {},
        PKT_LINE_UNREACHABLE: {},
        REMOTE_UNREACHABLE: {},
        TOKEN_EXPIRY_UNREACHABLE: {},
        UPLOAD_PACK_UNREACHABLE: {},

        // broken
        COMMAND_BROKEN: {},
    },
});

/** A stable cause of git failures. */
export type GitReason = ReasonOf<typeof GitError>;
