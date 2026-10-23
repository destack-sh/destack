import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The account package's failures, each reason with the call error code it answers. */
export const AccountError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    ACTING_OUTSIDE_SPACE: { code: "BAD_REQUEST" },
    FOREIGN_PAYER: { code: "BAD_REQUEST" },
    SELF_SUSPENSION: { code: "BAD_REQUEST" },
    IDENTITY_NOT_STARTED: { code: "BAD_REQUEST" },
    INVALID_TOKEN_EXPIRY: { code: "BAD_REQUEST" },
    MISSING_FORM_FIELD: { code: "BAD_REQUEST" },
    MISSING_QUERY_PARAMETER: { code: "BAD_REQUEST" },
    NOT_A_SIGNER: { code: "BAD_REQUEST" },
    NO_KEYS: { code: "BAD_REQUEST" },
    PERMISSION_WITHOUT_SCOPE: { code: "BAD_REQUEST" },
    PROOF_MISMATCH: { code: "BAD_REQUEST" },
    PUSH_BROWSER_ONLY: { code: "BAD_REQUEST" },
    SPACE_REQUIRED: { code: "BAD_REQUEST" },
    SUBJECT_MISMATCH: { code: "BAD_REQUEST" },
    UNKNOWN_ACTION: { code: "BAD_REQUEST" },
    UNKNOWN_MACHINE: { code: "BAD_REQUEST" },
    UNKNOWN_PERMISSION: { code: "BAD_REQUEST" },
    UNKNOWN_RESIDENCY: { code: "BAD_REQUEST" },
    UNKNOWN_TIME_ZONE: { code: "BAD_REQUEST" },
    UNUSABLE_SCOPE: { code: "BAD_REQUEST" },

    // unauthorized
    INVALID_PERSONAL_TOKEN: { code: "UNAUTHORIZED" },
    INVALID_TOKEN: { code: "UNAUTHORIZED" },
    KEY_REVOKED: { code: "UNAUTHORIZED" },
    NOT_SIGNED_IN: { code: "UNAUTHORIZED" },
    SESSION_ENDED: { code: "UNAUTHORIZED" },
    SIGNED_OUT: { code: "UNAUTHORIZED" },
    SIGN_IN_INCOMPLETE: { code: "UNAUTHORIZED" },
    UNKNOWN_CLIENT_KEY: { code: "UNAUTHORIZED" },
    UNSIGNED_OPERATION: { code: "UNAUTHORIZED" },

    // payment required
    CUSTOM_DOMAINS_UNENTITLED: { code: "PAYMENT_REQUIRED" },
    ENTITLEMENT_EXCEEDED: { code: "PAYMENT_REQUIRED" },

    // forbidden
    ACTING_REFUSED: { code: "FORBIDDEN" },
    ACTOR_NOT_USER: { code: "FORBIDDEN" },
    CREATOR_REQUIRED: { code: "FORBIDDEN" },
    DELEGATE_OUTSIDE_GRANTS: { code: "FORBIDDEN" },
    EMAIL_CONCEALED: { code: "FORBIDDEN" },
    FOREIGN_CLIENT: { code: "FORBIDDEN" },
    FOREIGN_IDENTITY_START: { code: "FORBIDDEN" },
    FOREIGN_INSTALLATION: { code: "FORBIDDEN" },
    KEY_PROOF_REFUSED: { code: "FORBIDDEN" },
    MACHINE_ASSERTION_REQUIRED: { code: "FORBIDDEN" },
    NO_ACTIVE_ACCOUNT: { code: "FORBIDDEN" },
    NO_LIVE_GRANT: { code: "FORBIDDEN" },
    PLACED_ELSEWHERE: { code: "FORBIDDEN" },
    SESSION_REQUIRED: { code: "FORBIDDEN" },
    USER_REQUIRED: { code: "FORBIDDEN" },

    // not found
    ACCOUNT_NOT_FOUND: { code: "NOT_FOUND" },
    ADDRESS_NOT_FOUND: { code: "NOT_FOUND" },
    IDENTITY_NOT_FOUND: { code: "NOT_FOUND" },
    MACHINE_NOT_PUBLISHED: { code: "NOT_FOUND" },
    NOT_A_PERSON: { code: "NOT_FOUND" },
    NO_CLIENT_KEY: { code: "NOT_FOUND" },
    NO_LOCAL_KEYS: { code: "NOT_FOUND" },
    NO_MACHINE_KEY: { code: "NOT_FOUND" },
    NO_PERSONAL_ACCOUNT: { code: "NOT_FOUND" },
    NO_RESIDENCY_HOME: { code: "NOT_FOUND" },
    PARENT_NOT_FOUND: { code: "NOT_FOUND" },
    RESIDENCY_NOT_FOUND: { code: "NOT_FOUND" },
    SPACE_NOT_PLACED: { code: "NOT_FOUND" },

    // conflict
    DOMAIN_TAKEN: { code: "CONFLICT" },
    DOMAIN_VERIFIED: { code: "CONFLICT" },
    HANDLE_TAKEN: { code: "CONFLICT" },
    NO_CURRENT_OPERATION: { code: "CONFLICT" },
    NULLIFY_REFUSED: { code: "CONFLICT" },
    ORIGIN_MISMATCH: { code: "CONFLICT" },
    PLACEMENT_MISMATCH: { code: "CONFLICT" },
    RESERVED_HANDLE_TAKEN: { code: "CONFLICT" },
    REVOKED: { code: "CONFLICT" },
    STALE_EPOCH: { code: "CONFLICT" },

    // precondition failed
    CHALLENGE_UNPUBLISHED: { code: "PRECONDITION_FAILED" },
    DOMAIN_UNVERIFIED: { code: "PRECONDITION_FAILED" },
    HOSTNAMES_IN_USE: { code: "PRECONDITION_FAILED" },
    UNMAPPED_PROVIDER: { code: "PRECONDITION_FAILED" },

    // internal server error
    AUTHENTICATION_FAILED: { code: "INTERNAL_SERVER_ERROR" },
    LOGIN_NOT_WRITTEN: { code: "INTERNAL_SERVER_ERROR" },
    LOOKUP_BY_IDENTIFIER_ONLY: { code: "INTERNAL_SERVER_ERROR" },
    NO_COUNT: { code: "INTERNAL_SERVER_ERROR" },
    NO_SIGNING_KEY: { code: "INTERNAL_SERVER_ERROR" },
    OPERATOR_TAKES_LIST: { code: "INTERNAL_SERVER_ERROR" },
    PRIVATE_FIELDS_CONCEALED: { code: "INTERNAL_SERVER_ERROR" },
    SESSION_READ_FAILED: { code: "INTERNAL_SERVER_ERROR" },
    UNKNOWN_AUTHENTICATION_COLUMN: { code: "INTERNAL_SERVER_ERROR" },
    UNKNOWN_AUTHENTICATION_MODEL: { code: "INTERNAL_SERVER_ERROR" },
    UNKNOWN_AUTHENTICATION_OPERATOR: { code: "INTERNAL_SERVER_ERROR" },
    UNKNOWN_AUTHENTICATION_TABLE: { code: "INTERNAL_SERVER_ERROR" },

    // bad gateway
    DEVICE_CODE_FAILED: { code: "BAD_GATEWAY" },
    HOST_CONTRADICTED: { code: "BAD_GATEWAY" },
    HOST_REFUSED: { code: "BAD_GATEWAY" },
    MAIL_FAILED: { code: "BAD_GATEWAY" },
    MALFORMED_ID_TOKEN: { code: "BAD_GATEWAY" },
    NO_ID_TOKEN: { code: "BAD_GATEWAY" },
    NO_REFRESH_TOKEN: { code: "BAD_GATEWAY" },
    REVOCATION_FAILED: { code: "BAD_GATEWAY" },
    TOKEN_ENDPOINT_FAILED: { code: "BAD_GATEWAY" },
    UNREADABLE_ANSWER: { code: "BAD_GATEWAY" },

    // service unavailable
    DNS_LOOKUP_FAILED: { code: "SERVICE_UNAVAILABLE" },
    DNS_RESOLVER_FAILED: { code: "SERVICE_UNAVAILABLE" },
    MAIL_UNSENT: { code: "SERVICE_UNAVAILABLE" },
    NO_CLOUD_MACHINE: { code: "SERVICE_UNAVAILABLE" },
});

/** A stable cause of account failures. */
export type AccountReason = ReasonOf<typeof AccountError>;
