import { defineErrorFamily, type FamilyError, type ReasonOf } from "@destack/error";
import { ValidationError, type SchemaIssue } from "@orpc/contract";
import {
    ApprovalRefusal,
    GrantRefusal,
    Issues,
    PartitionRoute,
    RetryAfter,
    ScopeMove,
} from "./detail.ts";
import type {} from "@destack/package/import-meta";
import { schema } from "@destack/schema";

/** The object package's failures, each reason with the call error code it answers. */
export const ObjectError = defineErrorFamily({
    reasons: {
        // not found
        BINDING_NOT_FOUND: {},
        BRANCH_NOT_FOUND: {},
        EPHEMERAL_STORAGE_NOT_FOUND: {},
        EVENT_NOT_FOUND: {},
        HOME_OR_REPLICA_NOT_FOUND: {},
        HOOKS_NOT_FOUND: {},
        HOOK_SESSION_NOT_FOUND: {},
        INSTALLATION_NOT_FOUND: {},
        LIST_METHOD_NOT_FOUND: {},
        OBJECT_NOT_FOUND: {},
        PACKAGE_NOT_FOUND: {},
        PUBLICATION_NOT_FOUND: {},
        REFERENCE_NOT_FOUND: {},
        SCOPE_NOT_FOUND: {},
        SCOPE_OUTSIDE_REQUEST_NOT_FOUND: {},
        SOCKET_HANDLER_NOT_FOUND: {},
        TYPE_NOT_FOUND: {},

        // exists
        ID_EXISTS: {},
        NAME_EXISTS: {},
        OBJECT_EXISTS: {},

        // conflict
        APPROVALS_CONFLICT: {},
        GENERATION_CONFLICT: {},
        REVISION_CONFLICT: {},

        // invalid
        AGGREGATE_AT_POSITION_INVALID: {},
        APPROVERS_INVALID: {},
        BODY_INVALID: {},
        BROADCAST_INVALID: {},
        CALL_INVALID: {},
        CALL_OUTSIDE_BRANCH_INVALID: {},
        CALL_UNBRANCHABLE_INVALID: {},
        CLAIM_INVALID: {},
        CONVERSION_INVALID: {},
        CURSOR_INVALID: {},
        DECLARATION_INVALID: {},
        ELEMENT_INVALID: {},
        EVENTS_INVALID: {},
        EVENT_INVALID: {},
        EXTERNAL_STORES_SPLIT_INVALID: {},
        HOMES_INVALID: {},
        HOME_INVALID: {},
        INCLUDE_INVALID: {},
        INPUT_INVALID: { data: Issues },
        METHOD_INVALID: {},
        MOVE_INVALID: {},
        OBJECT_INVALID: {},
        ORDER_INVALID: {},
        PAGE_LIMIT_INVALID: {},
        PARENT_TYPE_INVALID: {},
        PERMISSION_INVALID: {},
        QUORUM_INVALID: {},
        READABLE_INVALID: {},
        READ_INVALID: {},
        RELATION_ACROSS_STORAGE_INVALID: {},
        RELATION_BY_KEY_INVALID: {},
        RELEASE_INVALID: {},
        RUN_INVALID: {},
        SCOPES_INVALID: {},
        SCOPE_INVALID: {},
        SCOPE_OUTSIDE_CHAIN_INVALID: {},
        SELF_APPROVAL_INVALID: {},
        STORAGE_INVALID: {},
        TEXT_EDIT_INVALID: {},
        TRASH_INVALID: {},
        VERSION_INVALID: {},

        // required
        APPROVERS_REQUIRED: {},
        BRANCHES_REQUIRED: {},
        COMBINATION_REQUIRED: {},
        FIELD_REQUIRED: {},
        HOME_HANDLER_REQUIRED: {},
        LISTING_REQUIRED: {},
        PARENT_REQUIRED: {},
        PUSH_REQUIRED: {},
        READING_REQUIRED: {},
        REPLICA_RECEIVER_REQUIRED: {},
        START_REQUIRED: {},
        STATE_REQUIRED: {},
        TEXT_REQUIRED: {},
        TRASH_REQUIRED: {},
        VERSION_REQUIRED: {},
        WRITER_REQUIRED: {},

        // unauthenticated
        CALLER_UNAUTHENTICATED: {},
        CREDENTIAL_UNAUTHENTICATED: {},
        HOST_SECRET_UNAUTHENTICATED: {},
        LINK_SECRET_UNAUTHENTICATED: {},
        SIGNATURE_UNAUTHENTICATED: {},
        TIMESTAMP_OUTSIDE_TOLERANCE_UNAUTHENTICATED: {},

        // denied
        ADAPTER_REFUSES_CALLER_DENIED: {},
        APPROVAL_DENIED: { data: ApprovalRefusal },
        CALLER_KIND_DENIED: {},
        CHAIN_REPLICAS_DENIED: {},
        CHECK_DENIED: {},
        CREATOR_KIND_DENIED: {},
        DECIDER_DENIED: {},
        EXPLAIN_DENIED: {},
        FIELD_DENIED: {},
        FOLLOWERS_DENIED: {},
        FOLLOWER_OUTSIDE_TARGET_DENIED: {},
        GRANT_DENIED: { data: GrantRefusal },
        GRANT_REFUSES_METHOD_DENIED: {},
        GRANT_REFUSES_PERMISSION_DENIED: {},
        HOOK_CALLER_DENIED: {},
        INSERT_DENIED: {},
        LOOKUP_DENIED: {},
        MACHINE_PROCEDURE_DENIED: {},
        PERMISSION_DENIED: {},
        REVEAL_DENIED: {},
        STORE_ONLY_DENIED: {},
        TABLE_DENIED: {},

        // blocked
        BRANCH_BLOCKED: {},
        CALL_BLOCKED: {},
        DELETION_BLOCKED: {},
        ENCLOSING_SCOPE_BLOCKED: {},
        MERGE_BLOCKED: {},
        OBJECT_BLOCKED: {},
        PURGE_BLOCKED: {},
        RECOVERY_BLOCKED: {},
        REPLICA_BLOCKED: {},
        REQUEST_BLOCKED: {},
        SECRET_BLOCKED: {},
        TRANSITION_BLOCKED: {},
        VERSION_BLOCKED: {},

        // exhausted
        RATE_EXHAUSTED: { data: RetryAfter },
        STORAGE_EXHAUSTED: {},

        // moved
        PARTITION_MOVED: { data: PartitionRoute },
        SCOPE_MOVED: { data: ScopeMove },

        // unavailable
        BUILD_FILE_UNAVAILABLE: {},
        DATABASE_UNAVAILABLE: {},
        REPLICA_UNAVAILABLE: {},
        SERVER_UNAVAILABLE: { data: RetryAfter.partial() },
        SETTLEMENT_UNAVAILABLE: {},
        WAKE_UP_UNAVAILABLE: {},
        WORKLOAD_UNAVAILABLE: {},

        // unreachable
        ACCEPTANCE_UNREACHABLE: {},
        HOST_UNREACHABLE: {},
        INSTANCE_UNREACHABLE: {},

        // broken
        DECLARATION_BROKEN: {},
        OUTCOME_BROKEN: {},
        OUTPUT_BROKEN: { data: Issues },
    },
});

/** A stable cause of object failures. */
export type ObjectReason = ReasonOf<typeof ObjectError>;

/** Refuse input its schema rejects, with its issues, as a procedure refuses invalid input. */
export function refuseInput(
    input: unknown,
    issues: readonly SchemaIssue[],
): FamilyError<"INPUT_INVALID"> {
    const message = `invalid input: ${schema.describeIssues(issues)}`;

    return ObjectError.of("INPUT_INVALID", message, {
        data: Issues.of(issues),
        cause: new ValidationError({ message, issues, data: input }),
    });
}
