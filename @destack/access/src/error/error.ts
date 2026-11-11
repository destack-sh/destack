import { defineErrorFamily, type ReasonOf } from "@destack/error";
import { StepUp } from "../context/elevation.ts";
import type {} from "@destack/package/import-meta";

/** The access package's failures, each reason with the call error code it answers. */
export const AccessError = defineErrorFamily({
    reasons: {
        // not found
        GRANT_NOT_FOUND: {},
        INVITATION_NOT_FOUND: {},
        OBJECT_NOT_FOUND: {},
        RELATIONSHIP_NOT_FOUND: {},
        ROLE_NOT_FOUND: {},
        ROWS_NOT_FOUND: {},
        SCOPE_NOT_FOUND: {},
        TYPE_NOT_FOUND: {},

        // exists
        RELATIONSHIP_EXISTS: {},
        ROLE_NAME_EXISTS: {},

        // conflict
        REVISION_CONFLICT: {},
        UNMANAGED_CONFLICT: {},

        // invalid
        ATTRIBUTE_INVALID: {},
        CALLER_INVALID: {},
        EXCHANGE_INVALID: {},
        GRANT_EXPIRY_INVALID: {},
        INVITATION_EXPIRY_INVALID: {},
        PAGE_INVALID: {},
        PERMISSION_INVALID: {},
        PERMISSION_TYPE_INVALID: {},
        RELATIONSHIP_EXPIRY_INVALID: {},
        RELATIONSHIP_INVALID: {},
        REQUEST_TIME_INVALID: {},
        SUBJECT_INVALID: {},
        WILDCARD_ROLE_INVALID: {},

        // required
        SCOPE_REQUIRED: {},

        // unauthenticated
        ASSERTION_KEY_UNAUTHENTICATED: {},
        ASSERTION_OUTSIDE_AUTHORITY_UNAUTHENTICATED: {},
        ASSERTION_UNAUTHENTICATED: {},
        ASSURANCE_UNAUTHENTICATED: { data: StepUp.schema },
        AUDIENCE_UNAUTHENTICATED: {},
        BEARER_UNAUTHENTICATED: {},
        CALLER_UNAUTHENTICATED: {},
        CLIENT_UNAUTHENTICATED: {},
        CREDENTIAL_UNAUTHENTICATED: {},
        DELEGATION_UNAUTHENTICATED: {},
        GRANT_DELEGATE_UNAUTHENTICATED: {},
        GRANT_UNAUTHENTICATED: {},
        SESSION_UNAUTHENTICATED: {},
        TOKEN_OUTSIDE_AUTHORITY_UNAUTHENTICATED: {},
        TOKEN_UNAUTHENTICATED: {},
        VERIFICATION_UNAUTHENTICATED: {},
        WORKLOAD_IDENTITY_UNAUTHENTICATED: {},

        // denied
        ACCESS_DENIED: {},
        AUTHORITY_DENIED: {},
        DELEGATE_DENIED: {},
        DELEGATOR_DENIED: {},
        GRANT_DENIED: {},
        IDENTITY_DENIED: {},
        INVITATION_ADDRESSEE_DENIED: {},
        INVITATION_DENIED: {},
        INVITATION_SUBJECT_DENIED: {},
        LINK_SECRET_DENIED: {},
        MANAGER_DENIED: {},
        OWNER_DENIED: {},
        PERMISSION_DENIED: {},
        ROLE_SUBJECT_DENIED: {},
        SCOPE_DENIED: {},
        SUBJECT_OUTSIDE_CHAIN_DENIED: {},
        UNGRANTABLE_DENIED: {},

        // blocked
        DECLARATION_BLOCKED: {},
        INVITATION_BLOCKED: {},
        KEYS_BLOCKED: {},
        RELATIONSHIP_BLOCKED: {},
        ROLE_BLOCKED: {},

        // unavailable
        KEYS_UNAVAILABLE: {},
        REPLICA_UNAVAILABLE: {},

        // broken
        ACCESS_BROKEN: {},
        DECLARATION_BROKEN: {},
        TARGET_OUTSIDE_SCOPE_BROKEN: {},
    },
});

/** A stable cause of access failures. */
export type AccessReason = ReasonOf<typeof AccessError>;
