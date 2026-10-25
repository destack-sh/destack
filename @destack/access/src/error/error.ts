import { defineErrorFamily, type ReasonOf } from "@destack/error";
import { StepUp } from "../context/elevation.ts";
import type {} from "@destack/package/import-meta";

/** The access package's failures, each reason with the call error code it answers. */
export const AccessError = defineErrorFamily(import.meta.destack.package, {
    // declarations and contexts
    DECLARATION_INVALID: { code: "INTERNAL_SERVER_ERROR" },
    INVALID_FORWARDED_CALLER: { code: "BAD_REQUEST" },
    INVALID_ATTRIBUTE: { code: "BAD_REQUEST" },
    INVALID_REQUEST_TIME: { code: "BAD_REQUEST" },
    INVALID_PAGE: { code: "BAD_REQUEST" },
    ACCESS_UNRESOLVED: { code: "INTERNAL_SERVER_ERROR" },
    TARGET_OUTSIDE_SCOPE: { code: "INTERNAL_SERVER_ERROR" },
    PERMISSION_TYPE_MISMATCH: { code: "BAD_REQUEST" },
    SCOPE_REQUIRED: { code: "BAD_REQUEST" },

    // authentication
    INSUFFICIENT_AUTHENTICATION: { code: "UNAUTHORIZED", data: StepUp.schema },
    INVALID_BEARER: { code: "UNAUTHORIZED" },
    ASSERTION_KEY_UNKNOWN: { code: "UNAUTHORIZED" },
    ASSERTION_EXPIRED: { code: "UNAUTHORIZED" },
    INVALID_ASSERTION: { code: "UNAUTHORIZED" },
    ASSERTION_TOO_LONG: { code: "UNAUTHORIZED" },
    ASSERTION_REUSED: { code: "UNAUTHORIZED" },
    ASSERTION_OUTSIDE_AUTHORITY: { code: "UNAUTHORIZED" },
    ANONYMOUS_CALLER: { code: "UNAUTHORIZED" },
    VERIFICATION_EXPIRED: { code: "UNAUTHORIZED" },
    CLIENT_REQUIRED: { code: "UNAUTHORIZED" },
    INVALID_TOKEN: { code: "UNAUTHORIZED" },
    TOKEN_OUTSIDE_AUTHORITY: { code: "UNAUTHORIZED" },
    INVALID_WORKLOAD_IDENTITY: { code: "UNAUTHORIZED" },
    INVALID_DELEGATION: { code: "UNAUTHORIZED" },
    AUTHENTICATION_EXPIRED: { code: "UNAUTHORIZED" },
    AUDIENCE_MISMATCH: { code: "UNAUTHORIZED" },
    CREDENTIAL_CONFINED: { code: "UNAUTHORIZED" },
    INVALID_EXCHANGE: { code: "BAD_REQUEST" },
    INSECURE_KEYS: { code: "PRECONDITION_FAILED" },
    KEYS_UNAVAILABLE: { code: "SERVICE_UNAVAILABLE" },

    // grants
    GRANT_REQUIRED: { code: "FORBIDDEN" },
    GRANT_REVOKED: { code: "UNAUTHORIZED" },
    GRANT_EXPIRED: { code: "UNAUTHORIZED" },
    GRANT_DELEGATE_MISMATCH: { code: "UNAUTHORIZED" },
    GRANT_NOT_FOUND: { code: "NOT_FOUND" },
    FOREIGN_AUTHORITY: { code: "FORBIDDEN" },
    DELEGATE_REQUIRED: { code: "FORBIDDEN" },
    DELEGATOR_REQUIRED: { code: "FORBIDDEN" },
    INVALID_GRANT_EXPIRY: { code: "BAD_REQUEST" },

    // decisions
    PERMISSION_DENIED: { code: "FORBIDDEN" },
    OBJECT_NOT_FOUND: { code: "NOT_FOUND" },
    SCOPE_NOT_FOUND: { code: "NOT_FOUND" },
    SUBJECT_OUTSIDE_CHAIN: { code: "FORBIDDEN" },
    ACCESS_NOT_LOCAL: { code: "FORBIDDEN" },
    NO_INHERITED_ROWS: { code: "NOT_FOUND" },
    NO_REPLICATED_TYPE: { code: "NOT_FOUND" },
    STALE: { code: "SERVICE_UNAVAILABLE" },
    MANAGED: { code: "CONFLICT" },

    // roles and relationships
    ROLE_NOT_FOUND: { code: "NOT_FOUND" },
    ROLE_NAME_TAKEN: { code: "CONFLICT" },
    ROLE_IN_USE: { code: "CONFLICT" },
    REVISION_STALE: { code: "CONFLICT" },
    OWNER_REQUIRED: { code: "FORBIDDEN" },
    UNGRANTABLE: { code: "FORBIDDEN" },
    SUBJECT_REFUSED: { code: "BAD_REQUEST" },
    WILDCARD_ROLE: { code: "BAD_REQUEST" },
    ROLE_SUBJECT_REFUSED: { code: "FORBIDDEN" },
    INVALID_RELATIONSHIP_EXPIRY: { code: "BAD_REQUEST" },
    INVALID_RELATIONSHIP: { code: "BAD_REQUEST" },
    MANAGER_REQUIRED: { code: "FORBIDDEN" },
    RELATIONSHIP_NOT_FOUND: { code: "NOT_FOUND" },
    RELATIONSHIP_EXISTS: { code: "CONFLICT" },
    LAST_RELATIONSHIP: { code: "CONFLICT" },
    LINK_SECRET_INVALID: { code: "FORBIDDEN" },

    // invitations
    INVITATION_NOT_FOUND: { code: "NOT_FOUND" },
    INVITATION_NOT_PENDING: { code: "CONFLICT" },
    INVALID_INVITATION_EXPIRY: { code: "BAD_REQUEST" },
    INVALID_INVITATION_SUBJECT: { code: "FORBIDDEN" },
    INVITATION_LAPSED: { code: "FORBIDDEN" },
    IDENTITY_REQUIRED: { code: "FORBIDDEN" },
    INVITATION_ADDRESSEE_MISMATCH: { code: "FORBIDDEN" },
});

/** A stable cause of access failures. */
export type AccessReason = ReasonOf<typeof AccessError>;
