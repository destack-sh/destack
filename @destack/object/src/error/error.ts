import { defineErrorFamily, type FamilyError, type ReasonOf } from "@destack/error";
import { ValidationError, type SchemaIssue } from "@orpc/contract";
import { describeIssues } from "./call.ts";
import { GrantRefusal, Issues, PartitionRoute, RetryAfter, ScopeMove } from "./detail.ts";
import type {} from "@destack/package/import-meta";

/** The object package's failures, each reason with the call error code it answers. */
export const ObjectError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    AGGREGATE_AT_POSITION: { code: "BAD_REQUEST" },
    BODY_NOT_JSON: { code: "BAD_REQUEST" },
    BROADCAST_TOO_LARGE: { code: "BAD_REQUEST" },
    CALL_OUTSIDE_BRANCH: { code: "BAD_REQUEST" },
    DUPLICATE_RUN: { code: "BAD_REQUEST" },
    ELEMENT_NOT_FOUND: { code: "BAD_REQUEST" },
    EMPTY_COMBINATION: { code: "BAD_REQUEST" },
    EVENT_NOT_FOUND: { code: "BAD_REQUEST" },
    EXTERNAL_STORES_SPLIT: { code: "BAD_REQUEST" },
    FOREIGN_CLAIM: { code: "BAD_REQUEST" },
    HOME_UNREACHABLE: { code: "BAD_REQUEST" },
    INCLUDED_TRASH: { code: "BAD_REQUEST" },
    INVALID_CALL: { code: "BAD_REQUEST" },
    INVALID_CONVERSION: { code: "BAD_REQUEST" },
    INVALID_CURSOR: { code: "BAD_REQUEST" },
    INVALID_INPUT: { code: "BAD_REQUEST", data: Issues },
    INVALID_PAGE_LIMIT: { code: "BAD_REQUEST" },
    INVALID_TEXT_EDIT: { code: "BAD_REQUEST" },
    INVALID_VERSION: { code: "BAD_REQUEST" },
    MISSING_DESIRED_STATE: { code: "BAD_REQUEST" },
    MISSING_FIELD: { code: "BAD_REQUEST" },
    MISSING_KEY: { code: "BAD_REQUEST" },
    MISSING_PARENT: { code: "BAD_REQUEST" },
    MISSING_START: { code: "BAD_REQUEST" },
    MISSING_TEXT: { code: "BAD_REQUEST" },
    MISSING_VERSION: { code: "BAD_REQUEST" },
    MIXED_HOMES: { code: "BAD_REQUEST" },
    MIXED_SCOPES: { code: "BAD_REQUEST" },
    MIXED_STORAGE: { code: "BAD_REQUEST" },
    NOTHING_TO_READ: { code: "BAD_REQUEST" },
    NO_BRANCHES: { code: "BAD_REQUEST" },
    NO_HOME_HANDLER: { code: "BAD_REQUEST" },
    NO_REPLICA_RECEIVER: { code: "BAD_REQUEST" },
    NO_TRASH: { code: "BAD_REQUEST" },
    PARENT_TYPE_REFUSED: { code: "BAD_REQUEST" },
    PUSH_REQUIRED: { code: "BAD_REQUEST" },
    RELATION_ACROSS_STORAGE: { code: "BAD_REQUEST" },
    RELATION_BY_KEY: { code: "BAD_REQUEST" },
    RELEASE_MISMATCH: { code: "BAD_REQUEST" },
    SCOPE_MISMATCH: { code: "BAD_REQUEST" },
    SCOPE_OUTSIDE_CHAIN: { code: "BAD_REQUEST" },
    TIME_ORDER_ONLY: { code: "BAD_REQUEST" },
    UNBRANCHABLE_CALL: { code: "BAD_REQUEST" },
    UNKNOWN_INCLUDE: { code: "BAD_REQUEST" },
    UNKNOWN_METHOD: { code: "BAD_REQUEST" },
    UNKNOWN_OBJECT: { code: "BAD_REQUEST" },
    UNKNOWN_PERMISSION: { code: "BAD_REQUEST" },
    UNLISTED: { code: "BAD_REQUEST" },
    UNSERVED_EVENTS: { code: "BAD_REQUEST" },
    WRITER_REQUIRED: { code: "BAD_REQUEST" },

    // unauthorized
    INVALID_HOST_SECRET: { code: "UNAUTHORIZED" },
    INVALID_LINK_SECRET: { code: "UNAUTHORIZED" },
    MISSING_CREDENTIAL: { code: "UNAUTHORIZED" },
    UNAUTHENTICATED: { code: "UNAUTHORIZED" },

    // payment required
    STORAGE_CAPPED: { code: "PAYMENT_REQUIRED" },

    // forbidden
    ADAPTER_REFUSES_CALLER: { code: "FORBIDDEN" },
    APPROVAL_REQUIRED: { code: "FORBIDDEN", data: GrantRefusal },
    CALLER_KIND_REFUSED: { code: "FORBIDDEN" },
    CHECK_REFUSED: { code: "FORBIDDEN" },
    CREATOR_KIND_REFUSED: { code: "FORBIDDEN" },
    EVENTS_REFUSED: { code: "FORBIDDEN" },
    EXPLAIN_REFUSED: { code: "FORBIDDEN" },
    FIELD_NOT_WRITABLE: { code: "FORBIDDEN" },
    FOLLOWER_OUTSIDE_TARGET: { code: "FORBIDDEN" },
    GRANT_REFUSES_METHOD: { code: "FORBIDDEN" },
    GRANT_REFUSES_PERMISSION: { code: "FORBIDDEN" },
    HOOK_CALLER_REFUSED: { code: "FORBIDDEN" },
    INSUFFICIENT_GRANT: { code: "FORBIDDEN", data: GrantRefusal },
    MACHINE_PROCEDURE_REFUSED: { code: "FORBIDDEN" },
    NO_CHAIN_REPLICAS: { code: "FORBIDDEN" },
    NO_FOLLOWERS: { code: "FORBIDDEN" },
    PERMISSION_DENIED: { code: "FORBIDDEN" },
    REVEAL_REFUSED: { code: "FORBIDDEN" },
    STORE_ONLY: { code: "FORBIDDEN" },
    TABLE_REFUSED: { code: "FORBIDDEN" },

    // not found
    BINDING_NOT_FOUND: { code: "NOT_FOUND" },
    BRANCH_NOT_FOUND: { code: "NOT_FOUND" },
    NO_EPHEMERAL_STORAGE: { code: "NOT_FOUND" },
    NO_HOME_OR_REPLICA: { code: "NOT_FOUND" },
    NO_HOOKS: { code: "NOT_FOUND" },
    NO_HOOK_SESSION: { code: "NOT_FOUND" },
    NO_LIST_METHOD: { code: "NOT_FOUND" },
    NO_PACKAGE_ROUTES: { code: "NOT_FOUND" },
    NO_SOCKET_HANDLER: { code: "NOT_FOUND" },
    NO_WEBHOOKS: { code: "NOT_FOUND" },
    PUBLICATION_NOT_FOUND: { code: "NOT_FOUND" },
    SCOPE_NOT_CONTAINED: { code: "NOT_FOUND" },
    SCOPE_NOT_FOUND: { code: "NOT_FOUND" },
    SCOPE_OUTSIDE_REQUEST: { code: "NOT_FOUND" },
    SOCKET_NOT_FOUND: { code: "NOT_FOUND" },
    TYPE_NOT_IN_SCOPE: { code: "NOT_FOUND" },
    UNKNOWN_PACKAGE: { code: "NOT_FOUND" },
    UNKNOWN_TYPE: { code: "NOT_FOUND" },

    // conflict
    ALREADY_DELETED: { code: "CONFLICT" },
    BRANCH_NOT_OPEN: { code: "CONFLICT" },
    CALL_CONTENTS_DIFFER: { code: "CONFLICT" },
    DUPLICATE: { code: "CONFLICT" },
    ID_TAKEN: { code: "CONFLICT" },
    INVALID_TRANSITION: { code: "CONFLICT" },
    IN_TRASH: { code: "CONFLICT" },
    MOVE_INTO_SUBTREE: { code: "CONFLICT" },
    NAME_TAKEN: { code: "CONFLICT" },
    NOT_DELETED: { code: "CONFLICT" },
    NOT_DELETING: { code: "CONFLICT" },
    NOT_PURGED: { code: "CONFLICT" },
    PREDEFINED: { code: "CONFLICT" },
    PURGED: { code: "CONFLICT" },
    RECOVERY_EXPIRED: { code: "CONFLICT" },
    REMOVED: { code: "CONFLICT" },
    REPLICA_WRITE: { code: "CONFLICT" },
    REQUEST_REUSED: { code: "CONFLICT" },
    STALE_GENERATION: { code: "CONFLICT" },
    STALE_REVISION: { code: "CONFLICT" },
    VERSION_MISMATCH: { code: "CONFLICT" },
    VERSION_RETIRED: { code: "CONFLICT" },

    // precondition failed
    NO_ENCLOSING_SCOPE: { code: "PRECONDITION_FAILED" },

    // payload too large
    BODY_TOO_LARGE: { code: "PAYLOAD_TOO_LARGE" },

    // misdirected request
    PARTITION_MISDIRECTED: { code: "MISDIRECTED_REQUEST", data: PartitionRoute },
    SCOPE_MOVED: { code: "MISDIRECTED_REQUEST", data: ScopeMove },

    // too many requests
    RATE_LIMITED: { code: "TOO_MANY_REQUESTS", data: RetryAfter },

    // internal server error
    CYCLIC_DECLARATION: { code: "INTERNAL_SERVER_ERROR" },
    INVALID_DECLARATION: { code: "INTERNAL_SERVER_ERROR" },
    INVALID_OUTPUT: { code: "INTERNAL_SERVER_ERROR", data: Issues },
    MISSING_OUTCOME: { code: "INTERNAL_SERVER_ERROR" },
    UNSUPPORTED_DECLARATION: { code: "INTERNAL_SERVER_ERROR" },

    // bad gateway
    HOST_REFUSED: { code: "BAD_GATEWAY" },
    INSTANCE_REFUSED: { code: "BAD_GATEWAY" },
    MISSING_ACCEPTANCE: { code: "BAD_GATEWAY" },

    // service unavailable
    BUILD_FILE_UNAVAILABLE: { code: "SERVICE_UNAVAILABLE" },
    NO_WORKLOAD: { code: "SERVICE_UNAVAILABLE" },
    REPLICA_BEHIND: { code: "SERVICE_UNAVAILABLE" },
    REPLICA_SYNCING: { code: "SERVICE_UNAVAILABLE" },
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
): FamilyError<"INVALID_INPUT"> {
    const message = describeIssues(issues);

    return ObjectError.of("INVALID_INPUT", message, {
        data: Issues.of(issues),
        cause: new ValidationError({ message, issues, data: input }),
    });
}
