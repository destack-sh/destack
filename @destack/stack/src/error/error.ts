import { defineErrorFamily, type ReasonOf } from "@destack/error";
import { RetryAfter, ScopeMove } from "@destack/object";
import type {} from "@destack/package/import-meta";

/** The stack package's failures, each reason with the call error code it answers. */
export const StackError = defineErrorFamily({
    reasons: {
        // not found
        ACTIVE_TRANSFER_NOT_FOUND: {},
        BLOB_NOT_FOUND: {},
        BUILD_NOT_FOUND: {},
        HANDOFF_NOT_FOUND: {},
        HOSTNAME_NOT_FOUND: {},
        INSTALLATION_NOT_FOUND: {},
        PLACEMENT_NOT_FOUND: {},
        REPOSITORY_NOT_FOUND: {},
        RESOURCE_NOT_FOUND: {},
        REVISION_NOT_FOUND: {},
        RUNTIME_BUILD_NOT_FOUND: {},
        SNAPSHOT_NOT_FOUND: {},
        STACK_NOT_FOUND: {},
        TRANSFER_NOT_FOUND: {},
        TRANSFER_TO_CALLER_NOT_FOUND: {},
        VIEW_NOT_FOUND: {},

        // exists
        BUILD_EXISTS: {},

        // conflict
        BUILD_CONFLICT: {},
        INSTALLATION_CONFLICT: {},
        PLACEMENT_CONFLICT: {},

        // invalid
        DEFINITION_INVALID: {},
        EXPIRY_IN_PAST_INVALID: {},
        INSTALLATIONS_INVALID: {},
        MANIFEST_INVALID: {},
        METADATA_INVALID: {},
        PACKAGE_INVALID: {},
        ROUTE_INVALID: {},
        SUBMISSION_KIND_INVALID: {},
        VERSION_INVALID: {},

        // required
        HOME_REQUIRED: {},

        // denied
        DATABASE_DENIED: {},
        HOOKS_DENIED: {},
        KEY_DENIED: {},
        NETWORK_DENIED: {},
        PARTITION_TOKEN_DENIED: {},
        PERMISSION_DENIED: {},
        PROCESS_ISOLATION_DENIED: {},
        SPACE_ONLY_DENIED: {},

        // blocked
        ADAPTER_BLOCKED: {},
        BINDING_BLOCKED: {},
        BUILD_BLOCKED: {},
        BUILD_RUNNER_BLOCKED: {},
        CAPABILITY_BLOCKED: {},
        DATABASE_BLOCKED: {},
        DEPENDENCY_BLOCKED: {},
        DEPENDENT_BLOCKED: {},
        FENCE_BLOCKED: {},
        HOOK_BLOCKED: {},
        HOSTNAME_BLOCKED: {},
        INSTALLATION_BLOCKED: {},
        OUTPUT_ENTRY_BLOCKED: {},
        PLAN_BLOCKED: {},
        PROVIDER_BLOCKED: {},
        RELEASE_BLOCKED: {},
        REPLICA_BLOCKED: {},
        REPOSITORIES_BLOCKED: {},
        RESOURCE_BLOCKED: {},
        RESOURCE_PROVIDER_BLOCKED: {},
        RESTORE_BLOCKED: {},
        ROLLBACK_BLOCKED: {},
        SERVING_BLOCKED: {},
        TARGET_BLOCKED: {},
        TRANSFER_BLOCKED: {},

        // unsupported
        BINDABLE_UNSUPPORTED: {},
        PROCESS_UNSUPPORTED: {},
        RELEASE_UNSUPPORTED: {},
        RESOURCE_UNSUPPORTED: {},
        RESTORE_UNSUPPORTED: {},
        SNAPSHOT_UNSUPPORTED: {},

        // moved
        SCOPE_MOVED: { data: ScopeMove },

        // unavailable
        BUILD_UNAVAILABLE: {},
        CAPTURE_UNAVAILABLE: {},
        INSTALLATION_UNAVAILABLE: { data: RetryAfter },
        INSTANCE_UNAVAILABLE: {},
        RESOURCE_UNAVAILABLE: {},

        // unreachable
        REDIRECTS_UNREACHABLE: {},
        REDIRECT_UNREACHABLE: {},
    },
});

/** A stable cause of stack failures. */
export type StackReason = ReasonOf<typeof StackError>;
