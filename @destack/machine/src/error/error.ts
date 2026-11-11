import { GrantRequest } from "@destack/account/authentication";
import { defineErrorFamily, type ReasonOf } from "@destack/error";
import { ERROR_DATA, RetryAfter } from "@destack/object";
import type {} from "@destack/package/import-meta";

/** The machine package's failures, each reason with the call error code it answers. */
const machineError = defineErrorFamily({
    reasons: {
        // not found
        ACCOUNT_NOT_FOUND: {},
        CALLS_NOT_FOUND: {},
        CHECKOUT_NOT_FOUND: {},
        CHECKOUT_SPACE_ELSEWHERE_NOT_FOUND: {},
        CLIENT_PATH_NOT_FOUND: {},
        DIRECTORY_NOT_FOUND: {},
        EGRESS_ADDRESS_NOT_FOUND: {},
        HOME_PATH_NOT_FOUND: {},
        INSTALLATION_NOT_FOUND: {},
        PACKAGE_MOUNT_NOT_FOUND: {},
        PACKAGE_NOT_FOUND: {},
        PLATFORM_SERVICE_NOT_FOUND: {},
        PRESENTER_NOT_FOUND: {},
        PREVIEW_NOT_FOUND: {},
        ROUTE_NOT_FOUND: {},
        SIGNAL_NOT_FOUND: {},
        SPACE_NOT_FOUND: {},
        VIEW_NOT_FOUND: {},

        // invalid
        ENDPOINT_INVALID: {},
        METRIC_INVALID: {},
        OUTSIDE_CHECKOUT_INVALID: {},
        PATH_INVALID: {},
        POINT_WITHOUT_VALUE_INVALID: {},
        REGISTRATION_INVALID: {},
        RELATIVE_DIRECTORY_INVALID: {},
        RELEASE_INVALID: {},
        REQUEST_INVALID: {},
        SETTING_SCOPE_INVALID: {},
        SITE_INVALID: {},
        SITE_RULE_INVALID: {},
        SPACE_INVALID: {},
        TARGET_INVALID: {},
        VERSION_INVALID: {},

        // required
        LAUNCH_TOKEN_REQUIRED: {},
        OBJECTS_REQUIRED: {},
        RELOCATION_TARGET_REQUIRED: {},

        // unauthenticated
        CONNECTION_UNAUTHENTICATED: {},
        COPY_CREDENTIAL_UNAUTHENTICATED: {},
        INSTANCE_SECRET_UNAUTHENTICATED: {},
        ORIGIN_UNAUTHENTICATED: {},
        PROOF_UNAUTHENTICATED: {},
        SESSION_UNAUTHENTICATED: {},
        SIGN_IN_UNAUTHENTICATED: {},
        TOKEN_UNAUTHENTICATED: {},
        TOKEN_WITHOUT_MACHINE_UNAUTHENTICATED: {},

        // denied
        ADDRESS_DENIED: {},
        BUCKET_DENIED: {},
        GRANT_DENIED: { data: GrantRequest },
        INSTALLATION_DENIED: {},
        INSTALLATION_ORIGIN_DENIED: {},
        ORIGIN_DENIED: {},
        OWNER_DENIED: {},
        PARTITION_TOKEN_DENIED: {},
        PERSON_DENIED: {},
        PUBLISHER_DENIED: {},
        PUSH_DENIED: {},
        SIGN_IN_DENIED: {},
        VIEW_GRANT_DENIED: {},
        VIEW_PACKAGE_DENIED: {},

        // blocked
        BUILD_BLOCKED: {},
        ENROLLMENT_BLOCKED: {},
        LOGIN_BLOCKED: {},
        MACHINE_BLOCKED: {},
        PERSON_BLOCKED: {},
        SIGN_IN_BLOCKED: {},
        TREE_BLOCKED: {},

        // exhausted
        RATE_EXHAUSTED: { data: ERROR_DATA.TOO_MANY_REQUESTS },

        // unsupported
        METHOD_UNSUPPORTED: {},
        RUNTIME_UNSUPPORTED: {},
        SOCKETS_UNSUPPORTED: {},

        // moved
        OBJECTS_MOVED: {},

        // unavailable
        DEPLOYMENT_UNAVAILABLE: { data: RetryAfter.partial() },
        INSTALLATION_UNAVAILABLE: {},
        INSTANCE_UNAVAILABLE: {},
        MACHINE_UNAVAILABLE: {},
        REVISION_UNAVAILABLE: {},
        RUNNER_UNAVAILABLE: {},
        SPACE_UNAVAILABLE: {},
        TELEMETRY_UNAVAILABLE: {},
        TRACE_UNAVAILABLE: {},
        WORKLOAD_UNAVAILABLE: {},

        // unreachable
        BUILD_UNREACHABLE: {},
        MODULE_UNREACHABLE: {},
        OBJECT_UNREACHABLE: {},
        RUNTIME_ACTION_UNREACHABLE: {},
        SETTINGS_UNREACHABLE: {},

        // broken
        COMMAND_BROKEN: {},
        CONFIGURATION_BROKEN: {},
        LISTENER_BROKEN: {},
        MACHINE_ANSWER_BROKEN: {},
        ROUTE_BASE_BROKEN: {},
        RUNNER_BROKEN: {},
        SERVER_BROKEN: {},
        START_BROKEN: {},
        STOP_BROKEN: {},
    },
});

/** The machine package's failures, and the refusal of what a machine does only once enrolled. */
export const MachineError = Object.assign(machineError, {
    /** Refuse what a machine does only once enrolled, telling the person how to enroll it. */
    unenrolled(action: string) {
        return machineError.of(
            "ENROLLMENT_BLOCKED",
            `cannot run '${action}': it needs an enrolled machine, so sign in on this machine first`,
        );
    },
});

/** A stable cause of machine failures. */
export type MachineReason = ReasonOf<typeof machineError>;
