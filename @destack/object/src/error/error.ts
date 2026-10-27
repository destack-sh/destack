import { defineErrorFamily, type FamilyError, type ReasonOf } from "@destack/error";
import { ValidationError, type SchemaIssue } from "@orpc/contract";
import { describeIssues } from "./call.ts";
import {
    ApprovalRefusal,
    GrantRefusal,
    Issues,
    PartitionRoute,
    RetryAfter,
    ScopeMove,
} from "./detail.ts";
import type {} from "@destack/package/import-meta";

/** The object package's failures, each reason with the call error code it answers. */
export const ObjectError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    AGGREGATE_AT_POSITION: { code: "BAD_REQUEST" },
    BODY_NOT_JSON: { code: "BAD_REQUEST" },
    BROADCAST_TOO_LARGE: { code: "BAD_REQUEST" },
    CALL_OUTSIDE_BRANCH: { code: "BAD_REQUEST" },
    RUN_DUPLICATE: { code: "BAD_REQUEST" },
    ELEMENT_NOT_FOUND: { code: "BAD_REQUEST" },
    COMBINATION_EMPTY: { code: "BAD_REQUEST" },
    EVENT_NOT_FOUND: { code: "BAD_REQUEST" },
    EXTERNAL_STORES_SPLIT: { code: "BAD_REQUEST" },
    CLAIM_FOREIGN: { code: "BAD_REQUEST" },
    HOME_UNREACHABLE: { code: "BAD_REQUEST" },
    QUORUM_UNREACHABLE: { code: "BAD_REQUEST" },
    SELF_APPROVAL: { code: "BAD_REQUEST" },
    TRASH_INCLUDED: { code: "BAD_REQUEST" },
    CALL_INVALID: { code: "BAD_REQUEST" },
    CONVERSION_INVALID: { code: "BAD_REQUEST" },
    CURSOR_INVALID: { code: "BAD_REQUEST" },
    INPUT_INVALID: { code: "BAD_REQUEST", data: Issues },
    PAGE_LIMIT_INVALID: { code: "BAD_REQUEST" },
    TEXT_EDIT_INVALID: { code: "BAD_REQUEST" },
    VERSION_INVALID: { code: "BAD_REQUEST" },
    APPROVERS_INVALID: { code: "BAD_REQUEST" },
    APPROVERS_MISSING: { code: "BAD_REQUEST" },
    DESIRED_STATE_MISSING: { code: "BAD_REQUEST" },
    FIELD_MISSING: { code: "BAD_REQUEST" },
    KEY_MISSING: { code: "BAD_REQUEST" },
    PARENT_MISSING: { code: "BAD_REQUEST" },
    START_MISSING: { code: "BAD_REQUEST" },
    TEXT_MISSING: { code: "BAD_REQUEST" },
    VERSION_MISSING: { code: "BAD_REQUEST" },
    HOMES_MIXED: { code: "BAD_REQUEST" },
    SCOPES_MIXED: { code: "BAD_REQUEST" },
    STORAGE_MIXED: { code: "BAD_REQUEST" },
    READING_MISSING: { code: "BAD_REQUEST" },
    BRANCHES_MISSING: { code: "BAD_REQUEST" },
    HOME_HANDLER_MISSING: { code: "BAD_REQUEST" },
    REPLICA_RECEIVER_MISSING: { code: "BAD_REQUEST" },
    TRASH_MISSING: { code: "BAD_REQUEST" },
    PARENT_TYPE_REFUSED: { code: "BAD_REQUEST" },
    PUSH_REQUIRED: { code: "BAD_REQUEST" },
    RELATION_ACROSS_STORAGE: { code: "BAD_REQUEST" },
    RELATION_BY_KEY: { code: "BAD_REQUEST" },
    RELEASE_MISMATCH: { code: "BAD_REQUEST" },
    SCOPE_MISMATCH: { code: "BAD_REQUEST" },
    SCOPE_OUTSIDE_CHAIN: { code: "BAD_REQUEST" },
    ORDER_UNSUPPORTED: { code: "BAD_REQUEST" },
    CALL_UNBRANCHABLE: { code: "BAD_REQUEST" },
    INCLUDE_UNKNOWN: { code: "BAD_REQUEST" },
    METHOD_UNKNOWN: { code: "BAD_REQUEST" },
    OBJECT_UNKNOWN: { code: "BAD_REQUEST" },
    OBJECT_AMBIGUOUS: { code: "BAD_REQUEST" },
    PERMISSION_UNKNOWN: { code: "BAD_REQUEST" },
    LISTING_MISSING: { code: "BAD_REQUEST" },
    EVENTS_UNSERVED: { code: "BAD_REQUEST" },
    WRITER_REQUIRED: { code: "BAD_REQUEST" },

    // unauthorized
    HOST_SECRET_INVALID: { code: "UNAUTHORIZED" },
    LINK_SECRET_INVALID: { code: "UNAUTHORIZED" },
    CREDENTIAL_MISSING: { code: "UNAUTHORIZED" },
    SIGNATURE_MISSING: { code: "UNAUTHORIZED" },
    SIGNATURE_MISMATCH: { code: "UNAUTHORIZED" },
    TIMESTAMP_OUTSIDE_TOLERANCE: { code: "UNAUTHORIZED" },
    UNAUTHENTICATED: { code: "UNAUTHORIZED" },

    // payment required
    STORAGE_CAPPED: { code: "PAYMENT_REQUIRED" },

    // forbidden
    ADAPTER_REFUSES_CALLER: { code: "FORBIDDEN" },
    APPROVAL_REQUIRED: { code: "FORBIDDEN", data: ApprovalRefusal },
    CALLER_KIND_REFUSED: { code: "FORBIDDEN" },
    CHECK_REFUSED: { code: "FORBIDDEN" },
    CREATOR_KIND_REFUSED: { code: "FORBIDDEN" },
    DECIDER_REFUSED: { code: "FORBIDDEN" },
    EVENTS_REFUSED: { code: "FORBIDDEN" },
    EXPLAIN_REFUSED: { code: "FORBIDDEN" },
    FIELD_NOT_WRITABLE: { code: "FORBIDDEN" },
    FOLLOWER_OUTSIDE_TARGET: { code: "FORBIDDEN" },
    GRANT_REFUSES_METHOD: { code: "FORBIDDEN" },
    GRANT_REFUSES_PERMISSION: { code: "FORBIDDEN" },
    HOOK_CALLER_REFUSED: { code: "FORBIDDEN" },
    INSERT_REFUSED: { code: "FORBIDDEN" },
    GRANT_INSUFFICIENT: { code: "FORBIDDEN", data: GrantRefusal },
    MACHINE_PROCEDURE_REFUSED: { code: "FORBIDDEN" },
    CHAIN_REPLICAS_MISSING: { code: "FORBIDDEN" },
    FOLLOWERS_MISSING: { code: "FORBIDDEN" },
    PERMISSION_DENIED: { code: "FORBIDDEN" },
    REVEAL_REFUSED: { code: "FORBIDDEN" },
    STORE_ONLY: { code: "FORBIDDEN" },
    TABLE_REFUSED: { code: "FORBIDDEN" },

    // not found
    BINDING_NOT_FOUND: { code: "NOT_FOUND" },
    BRANCH_NOT_FOUND: { code: "NOT_FOUND" },
    EPHEMERAL_STORAGE_MISSING: { code: "NOT_FOUND" },
    HOME_OR_REPLICA_MISSING: { code: "NOT_FOUND" },
    HOOKS_MISSING: { code: "NOT_FOUND" },
    HOOK_SESSION_MISSING: { code: "NOT_FOUND" },
    LIST_METHOD_MISSING: { code: "NOT_FOUND" },
    PACKAGE_ROUTES_MISSING: { code: "NOT_FOUND" },
    SOCKET_HANDLER_MISSING: { code: "NOT_FOUND" },
    WEBHOOKS_MISSING: { code: "NOT_FOUND" },
    PUBLICATION_NOT_FOUND: { code: "NOT_FOUND" },
    SCOPE_NOT_CONTAINED: { code: "NOT_FOUND" },
    SCOPE_NOT_FOUND: { code: "NOT_FOUND" },
    SCOPE_OUTSIDE_REQUEST: { code: "NOT_FOUND" },
    SOCKET_NOT_FOUND: { code: "NOT_FOUND" },
    TYPE_NOT_IN_SCOPE: { code: "NOT_FOUND" },
    PACKAGE_UNKNOWN: { code: "NOT_FOUND" },
    TYPE_UNKNOWN: { code: "NOT_FOUND" },

    // conflict
    ALREADY_DELETED: { code: "CONFLICT" },
    BRANCH_NOT_OPEN: { code: "CONFLICT" },
    UNMERGED_CALLS: { code: "CONFLICT" },
    APPROVALS_CHANGED: { code: "CONFLICT" },
    CALL_CONTENTS_DIFFER: { code: "CONFLICT" },
    DUPLICATE: { code: "CONFLICT" },
    ID_TAKEN: { code: "CONFLICT" },
    TRANSITION_INVALID: { code: "CONFLICT" },
    OBJECT_TRASHED: { code: "CONFLICT" },
    MOVE_INTO_SUBTREE: { code: "CONFLICT" },
    NAME_TAKEN: { code: "CONFLICT" },
    DELETION_REQUIRED: { code: "CONFLICT" },
    DELETION_UNSTARTED: { code: "CONFLICT" },
    PURGE_REQUIRED: { code: "CONFLICT" },
    PREDEFINED: { code: "CONFLICT" },
    PURGED: { code: "CONFLICT" },
    RECOVERY_EXPIRED: { code: "CONFLICT" },
    REMOVED: { code: "CONFLICT" },
    REPLICA_WRITE: { code: "CONFLICT" },
    REQUEST_REUSED: { code: "CONFLICT" },
    GENERATION_STALE: { code: "CONFLICT" },
    REVISION_STALE: { code: "CONFLICT" },
    VERSION_MISMATCH: { code: "CONFLICT" },
    VERSION_RETIRED: { code: "CONFLICT" },

    // precondition failed
    SECRET_INVALID: { code: "PRECONDITION_FAILED" },
    ENCLOSING_SCOPE_MISSING: { code: "PRECONDITION_FAILED" },

    // payload too large
    BODY_TOO_LARGE: { code: "PAYLOAD_TOO_LARGE" },

    // misdirected request
    PARTITION_MISDIRECTED: { code: "MISDIRECTED_REQUEST", data: PartitionRoute },
    SCOPE_MOVED: { code: "MISDIRECTED_REQUEST", data: ScopeMove },

    // too many requests
    RATE_LIMITED: { code: "TOO_MANY_REQUESTS", data: RetryAfter },

    // internal server error
    DECLARATION_CYCLIC: { code: "INTERNAL_SERVER_ERROR" },
    DECLARATION_INVALID: { code: "INTERNAL_SERVER_ERROR" },
    OUTPUT_INVALID: { code: "INTERNAL_SERVER_ERROR", data: Issues },
    OUTCOME_MISSING: { code: "INTERNAL_SERVER_ERROR" },
    DECLARATION_UNSUPPORTED: { code: "INTERNAL_SERVER_ERROR" },

    // bad gateway
    HOST_REFUSED: { code: "BAD_GATEWAY" },
    INSTANCE_REFUSED: { code: "BAD_GATEWAY" },
    ACCEPTANCE_MISSING: { code: "BAD_GATEWAY" },

    // service unavailable
    BUILD_FILE_UNAVAILABLE: { code: "SERVICE_UNAVAILABLE" },
    WORKLOAD_MISSING: { code: "SERVICE_UNAVAILABLE" },
    REPLICA_BEHIND: { code: "SERVICE_UNAVAILABLE" },
    REPLICA_SYNCING: { code: "SERVICE_UNAVAILABLE" },
    SERVER_DRAINING: { code: "SERVICE_UNAVAILABLE" },
    SERVER_NOT_SERVING: { code: "SERVICE_UNAVAILABLE" },
    SERVER_STARTING: { code: "SERVICE_UNAVAILABLE" },
    SERVER_STOPPED: { code: "SERVICE_UNAVAILABLE" },
    SETTLEMENT_EXPIRED: { code: "SERVICE_UNAVAILABLE" },
    WAKE_UP_UNSTORED: { code: "SERVICE_UNAVAILABLE" },
});

/** A stable cause of object failures. */
export type ObjectReason = ReasonOf<typeof ObjectError>;

/** Refuse input its schema rejects, with its issues, as a procedure refuses invalid input. */
export function refuseInput(
    input: unknown,
    issues: readonly SchemaIssue[],
): FamilyError<"INPUT_INVALID"> {
    const message = describeIssues(issues);

    return ObjectError.of("INPUT_INVALID", message, {
        data: Issues.of(issues),
        cause: new ValidationError({ message, issues, data: input }),
    });
}
