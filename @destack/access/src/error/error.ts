import { defineErrorFamily, type ReasonOf } from "@destack/error";
import { StepUp } from "../context/elevation.ts";
import type {} from "@destack/package/import-meta";

/** The access package's failures, each reason with the call error code it answers. */
export const AccessError = defineErrorFamily(import.meta.destack.package, {
    // declarations and contexts
    DECLARATION_INVALID: { code: "INTERNAL_SERVER_ERROR" },
    FORWARDED_CALLER_INVALID: { code: "BAD_REQUEST" },
    ATTRIBUTE_INVALID: { code: "BAD_REQUEST" },
    REQUEST_TIME_INVALID: { code: "BAD_REQUEST" },
    PAGE_INVALID: { code: "BAD_REQUEST" },
    ACCESS_UNRESOLVED: { code: "INTERNAL_SERVER_ERROR" },
    TARGET_OUTSIDE_SCOPE: { code: "INTERNAL_SERVER_ERROR" },
    PERMISSION_TYPE_MISMATCH: { code: "BAD_REQUEST" },
    PERMISSION_REPEATED: { code: "BAD_REQUEST" },
    SCOPE_REQUIRED: { code: "BAD_REQUEST" },

    // authentication
    AUTHENTICATION_INSUFFICIENT: { code: "UNAUTHORIZED", data: StepUp.schema },
    BEARER_INVALID: { code: "UNAUTHORIZED" },
    ASSERTION_KEY_UNKNOWN: { code: "UNAUTHORIZED" },
    ASSERTION_EXPIRED: { code: "UNAUTHORIZED" },
    ASSERTION_INVALID: { code: "UNAUTHORIZED" },
    ASSERTION_TOO_LONG: { code: "UNAUTHORIZED" },
    ASSERTION_REUSED: { code: "UNAUTHORIZED" },
    ASSERTION_OUTSIDE_AUTHORITY: { code: "UNAUTHORIZED" },
    ANONYMOUS_CALLER: { code: "UNAUTHORIZED" },
    VERIFICATION_EXPIRED: { code: "UNAUTHORIZED" },
    CLIENT_REQUIRED: { code: "UNAUTHORIZED" },
    TOKEN_INVALID: { code: "UNAUTHORIZED" },
    TOKEN_OUTSIDE_AUTHORITY: { code: "UNAUTHORIZED" },
    WORKLOAD_IDENTITY_INVALID: { code: "UNAUTHORIZED" },
    DELEGATION_INVALID: { code: "UNAUTHORIZED" },
    AUTHENTICATION_EXPIRED: { code: "UNAUTHORIZED" },
    AUDIENCE_MISMATCH: { code: "UNAUTHORIZED" },
    CREDENTIAL_CONFINED: { code: "UNAUTHORIZED" },
    EXCHANGE_INVALID: { code: "BAD_REQUEST" },
    KEYS_INSECURE: { code: "PRECONDITION_FAILED" },
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
    GRANT_EXPIRY_INVALID: { code: "BAD_REQUEST" },

    // decisions
    PERMISSION_DENIED: { code: "FORBIDDEN" },
    OBJECT_NOT_FOUND: { code: "NOT_FOUND" },
    SCOPE_NOT_FOUND: { code: "NOT_FOUND" },
    SUBJECT_OUTSIDE_CHAIN: { code: "FORBIDDEN" },
    ACCESS_NOT_LOCAL: { code: "FORBIDDEN" },
    INHERITED_ROWS_MISSING: { code: "NOT_FOUND" },
    REPLICATED_TYPE_MISSING: { code: "NOT_FOUND" },
    STALE: { code: "SERVICE_UNAVAILABLE" },
    MANAGED: { code: "CONFLICT" },
    UNMANAGED: { code: "CONFLICT" },

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
    RELATIONSHIP_EXPIRY_INVALID: { code: "BAD_REQUEST" },
    RELATIONSHIP_INVALID: { code: "BAD_REQUEST" },
    MANAGER_REQUIRED: { code: "FORBIDDEN" },
    RELATIONSHIP_NOT_FOUND: { code: "NOT_FOUND" },
    RELATIONSHIP_EXISTS: { code: "CONFLICT" },
    LAST_RELATIONSHIP: { code: "CONFLICT" },
    LINK_SECRET_INVALID: { code: "FORBIDDEN" },

    // invitations
    INVITATION_NOT_FOUND: { code: "NOT_FOUND" },
    INVITATION_NOT_PENDING: { code: "CONFLICT" },
    INVITATION_EXPIRY_INVALID: { code: "BAD_REQUEST" },
    INVITATION_SUBJECT_INVALID: { code: "FORBIDDEN" },
    INVITATION_LAPSED: { code: "FORBIDDEN" },
    IDENTITY_REQUIRED: { code: "FORBIDDEN" },
    INVITATION_ADDRESSEE_MISMATCH: { code: "FORBIDDEN" },
});

/** A stable cause of access failures. */
export type AccessReason = ReasonOf<typeof AccessError>;
