import { defineErrorFamily, type ReasonOf } from "@destack/error";
import { ScopeMove } from "@destack/object";
import type {} from "@destack/package/import-meta";

/** The stack package's failures, each reason with the call error code it answers. */
export const StackError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    EXPIRY_IN_PAST: { code: "BAD_REQUEST" },
    DEFINITION_INVALID: { code: "BAD_REQUEST" },
    MANIFEST_MISMATCH: { code: "BAD_REQUEST" },
    METADATA_TOO_LARGE: { code: "BAD_REQUEST" },
    HOME_MISSING: { code: "BAD_REQUEST" },
    PACKAGE_MISMATCH: { code: "BAD_REQUEST" },
    SUBMISSION_KIND_MISMATCH: { code: "BAD_REQUEST" },
    VERSION_MISMATCH: { code: "BAD_REQUEST" },

    // unauthorized
    FOREIGN_ASSERTION: { code: "UNAUTHORIZED" },
    SPACE_NOT_PLACED: { code: "UNAUTHORIZED" },

    // forbidden
    FOREIGN_PARTITION_TOKEN: { code: "FORBIDDEN" },
    HOOKS_MISSING: { code: "FORBIDDEN" },
    RELEASE_MISMATCH: { code: "FORBIDDEN" },
    RELEASE_NOT_SHIPPED: { code: "FORBIDDEN" },
    SPACE_ONLY: { code: "FORBIDDEN" },
    NETWORK_UNDECLARED: { code: "FORBIDDEN" },

    // not found
    BLOB_NOT_FOUND: { code: "NOT_FOUND" },
    BUILD_NOT_FOUND: { code: "NOT_FOUND" },
    BUILD_NOT_RUNNING: { code: "NOT_FOUND" },
    HANDOFF_NOT_FOUND: { code: "NOT_FOUND" },
    HOSTNAME_NOT_FOUND: { code: "NOT_FOUND" },
    INSTALLATION_NOT_FOUND: { code: "NOT_FOUND" },
    INSTALLATION_NOT_HERE: { code: "NOT_FOUND" },
    ACTIVE_TRANSFER_MISSING: { code: "NOT_FOUND" },
    TRANSFER_TO_CALLER_MISSING: { code: "NOT_FOUND" },
    RUNTIME_BUILD_MISSING: { code: "NOT_FOUND" },
    REPOSITORY_NOT_FOUND: { code: "NOT_FOUND" },
    RESOURCE_NOT_FOUND: { code: "NOT_FOUND" },
    SNAPSHOT_NOT_REQUESTED: { code: "NOT_FOUND" },
    REVISION_NOT_FOUND: { code: "NOT_FOUND" },
    STACK_NOT_FOUND: { code: "NOT_FOUND" },
    TRANSFER_NOT_FOUND: { code: "NOT_FOUND" },
    VIEW_NOT_FOUND: { code: "NOT_FOUND" },

    // conflict
    INSTALLATION_AMBIGUOUS: { code: "CONFLICT" },
    BOUND: { code: "CONFLICT" },
    BUILD_EXPIRED: { code: "CONFLICT" },
    BUILD_NOT_READY: { code: "CONFLICT" },
    BUILD_STATE_CONFLICT: { code: "CONFLICT" },
    CAPTURED: { code: "CONFLICT" },
    CHANGE_REQUEST_REQUIRED: { code: "CONFLICT" },
    INSTALLATIONS_CYCLIC: { code: "CONFLICT" },
    BUILD_DUPLICATE: { code: "CONFLICT" },
    RELEASE_NOT_PULLED: { code: "CONFLICT" },
    KEY_REVOKED: { code: "CONFLICT" },
    PLAN_WAITING_MISSING: { code: "CONFLICT" },
    PREVIOUS_REVISION_MISSING: { code: "CONFLICT" },
    PLACEMENT_MOVED: { code: "CONFLICT" },
    PLAN_MISMATCH: { code: "CONFLICT" },
    REPLICA_DIFFERS: { code: "CONFLICT" },
    RESTORE_PENDING: { code: "CONFLICT" },
    ROLLBACK_TO_BUILD: { code: "CONFLICT" },
    SERVED_BY_TARGET: { code: "CONFLICT" },
    SERVED_ELSEWHERE: { code: "CONFLICT" },
    TRANSFER_ENDED: { code: "CONFLICT" },
    UNMERGED_CALLS: { code: "CONFLICT" },
    VERSION_CAPTURED: { code: "CONFLICT" },

    // precondition failed
    FOREIGN_RESOURCE: { code: "PRECONDITION_FAILED" },
    HOSTNAME_NOT_SERVED: { code: "PRECONDITION_FAILED" },
    OUTPUT_ENTRY_MISSING: { code: "PRECONDITION_FAILED" },
    NEEDED_BY_DEPENDENT: { code: "PRECONDITION_FAILED" },
    FENCE_REQUIRED: { code: "PRECONDITION_FAILED" },
    BUILD_RUNNER_MISSING: { code: "PRECONDITION_FAILED" },
    REPOSITORIES_MISSING: { code: "PRECONDITION_FAILED" },
    RESOURCE_PROVIDER_MISSING: { code: "PRECONDITION_FAILED" },
    PLAN_UNAPPLIED: { code: "PRECONDITION_FAILED" },
    RELEASE_REQUIRED: { code: "PRECONDITION_FAILED" },
    PROVIDER_MISMATCH: { code: "PRECONDITION_FAILED" },
    SERVED_IN_PROCESS: { code: "PRECONDITION_FAILED" },
    UNSERVED: { code: "PRECONDITION_FAILED" },

    // misdirected request
    SCOPE_MOVED: { code: "MISDIRECTED_REQUEST", data: ScopeMove },

    // bad gateway
    REDIRECTS_EXCEEDED: { code: "BAD_GATEWAY" },
    REDIRECT_REFUSED: { code: "BAD_GATEWAY" },

    // not implemented
    RESOURCE_UNSUPPORTED: { code: "NOT_IMPLEMENTED" },
    RESTORE_UNSUPPORTED: { code: "NOT_IMPLEMENTED" },
    SNAPSHOT_UNSUPPORTED: { code: "NOT_IMPLEMENTED" },

    // service unavailable
    CAPTURE_STOPPED: { code: "SERVICE_UNAVAILABLE" },
    INSTALLATION_NOT_SERVED: { code: "SERVICE_UNAVAILABLE" },
    INSTANCE_NOT_RUNNING: { code: "SERVICE_UNAVAILABLE" },
    RESOURCE_UNPROVISIONED: { code: "SERVICE_UNAVAILABLE" },
});

/** A stable cause of stack failures. */
export type StackReason = ReasonOf<typeof StackError>;
