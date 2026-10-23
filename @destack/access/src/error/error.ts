import type { DomainError, CallErrorCode, Failure } from "@destack/error";
import type { StepUp } from "../context/elevation.ts";
import type {} from "@destack/package/import-meta";

/** The call error code of each access failure. */
const CALL_CODES = {
    // declarations and contexts
    INVALID_DECLARATION: "INTERNAL_SERVER_ERROR",
    INVALID_FORWARDED_CALLER: "BAD_REQUEST",
    INVALID_ATTRIBUTE: "BAD_REQUEST",
    INVALID_REQUEST_TIME: "BAD_REQUEST",
    INVALID_PAGE: "BAD_REQUEST",
    ACCESS_UNRESOLVED: "INTERNAL_SERVER_ERROR",
    TARGET_OUTSIDE_SCOPE: "INTERNAL_SERVER_ERROR",
    PERMISSION_TYPE_MISMATCH: "BAD_REQUEST",
    SCOPE_REQUIRED: "BAD_REQUEST",

    // authentication
    INSUFFICIENT_AUTHENTICATION: "UNAUTHORIZED",
    INVALID_BEARER: "UNAUTHORIZED",
    ASSERTION_KEY_UNKNOWN: "UNAUTHORIZED",
    ASSERTION_EXPIRED: "UNAUTHORIZED",
    INVALID_ASSERTION: "UNAUTHORIZED",
    ASSERTION_TOO_LONG: "UNAUTHORIZED",
    ASSERTION_REUSED: "UNAUTHORIZED",
    ASSERTION_OUTSIDE_AUTHORITY: "UNAUTHORIZED",
    ANONYMOUS_CALLER: "UNAUTHORIZED",
    VERIFICATION_EXPIRED: "UNAUTHORIZED",
    CLIENT_REQUIRED: "UNAUTHORIZED",
    INVALID_TOKEN: "UNAUTHORIZED",
    TOKEN_OUTSIDE_AUTHORITY: "UNAUTHORIZED",
    INVALID_WORKLOAD_IDENTITY: "UNAUTHORIZED",
    INVALID_DELEGATION: "UNAUTHORIZED",
    AUTHENTICATION_EXPIRED: "UNAUTHORIZED",
    AUDIENCE_MISMATCH: "UNAUTHORIZED",
    CREDENTIAL_CONFINED: "UNAUTHORIZED",
    INVALID_EXCHANGE: "BAD_REQUEST",
    INSECURE_KEYS: "PRECONDITION_FAILED",
    KEYS_UNAVAILABLE: "SERVICE_UNAVAILABLE",

    // grants
    GRANT_REQUIRED: "FORBIDDEN",
    GRANT_REVOKED: "UNAUTHORIZED",
    GRANT_EXPIRED: "UNAUTHORIZED",
    GRANT_DELEGATE_MISMATCH: "UNAUTHORIZED",
    GRANT_NOT_FOUND: "NOT_FOUND",
    FOREIGN_AUTHORITY: "FORBIDDEN",
    DELEGATE_REQUIRED: "FORBIDDEN",
    DELEGATOR_REQUIRED: "FORBIDDEN",
    INVALID_GRANT_EXPIRY: "BAD_REQUEST",

    // decisions
    PERMISSION_DENIED: "FORBIDDEN",
    OBJECT_NOT_FOUND: "NOT_FOUND",
    SCOPE_NOT_FOUND: "NOT_FOUND",
    SUBJECT_OUTSIDE_CHAIN: "FORBIDDEN",
    ACCESS_NOT_LOCAL: "FORBIDDEN",
    NO_INHERITED_ROWS: "NOT_FOUND",
    NO_REPLICATED_TYPE: "NOT_FOUND",
    STALE: "SERVICE_UNAVAILABLE",
    MANAGED: "CONFLICT",

    // roles and relationships
    ROLE_NOT_FOUND: "NOT_FOUND",
    ROLE_NAME_TAKEN: "CONFLICT",
    ROLE_IN_USE: "CONFLICT",
    STALE_REVISION: "CONFLICT",
    OWNER_REQUIRED: "FORBIDDEN",
    UNGRANTABLE: "FORBIDDEN",
    SUBJECT_REFUSED: "BAD_REQUEST",
    WILDCARD_ROLE: "BAD_REQUEST",
    ROLE_SUBJECT_REFUSED: "FORBIDDEN",
    INVALID_RELATIONSHIP_EXPIRY: "BAD_REQUEST",
    INVALID_RELATIONSHIP: "BAD_REQUEST",
    MANAGER_REQUIRED: "FORBIDDEN",
    RELATIONSHIP_NOT_FOUND: "NOT_FOUND",
    RELATIONSHIP_EXISTS: "CONFLICT",
    LAST_OWNER: "CONFLICT",
    INVALID_LINK_SECRET: "FORBIDDEN",

    // invitations
    INVITATION_NOT_FOUND: "NOT_FOUND",
    INVITATION_NOT_PENDING: "CONFLICT",
    INVALID_INVITATION_EXPIRY: "BAD_REQUEST",
    INVALID_INVITATION_SUBJECT: "FORBIDDEN",
    INVITATION_LAPSED: "FORBIDDEN",
    IDENTITY_REQUIRED: "FORBIDDEN",
    INVITATION_ADDRESSEE_MISMATCH: "FORBIDDEN",
} as const satisfies Readonly<Record<string, CallErrorCode>>;

/** A stable cause of failures of access decisions and declarations. */
export type AccessReason = keyof typeof CALL_CODES;

/** Reject an invalid declaration, unavailable record, or unauthorized operation. */
export class AccessError extends Error implements DomainError {
    /** The failure's stable cause within its package. */
    readonly reason: AccessReason;
    /** The authentication that would allow the caller, for insufficient authentication. */
    readonly stepUp?: StepUp;

    /** Retain the classification, cause, and the authentication that would allow the caller. */
    constructor(
        reason: AccessReason,
        message: string,
        options?: ErrorOptions & { readonly stepUp?: StepUp },
    ) {
        // classify the error
        super(message, options);
        this.name = "AccessError";
        this.reason = reason;

        // take the refusal's challenge
        if (options?.stepUp !== undefined) {
            this.stepUp = options.stepUp;
        }
    }

    /** Convert the failure to the caller's call error, with its step-up challenge. */
    toCallError(): Failure {
        const failure = {
            code: CALL_CODES[this.reason],
            reason: this.reason,
            domain: import.meta.destack.package.name,
            message: this.message,
        };

        return this.stepUp === undefined ? failure : { ...failure, data: this.stepUp };
    }
}
