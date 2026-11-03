import { GrantPermission } from "@destack/access";
import { defineErrorFamily, type ReasonOf } from "@destack/error";
import { schema } from "@destack/schema";
import { ERROR_DATA } from "@destack/object";
import type {} from "@destack/package/import-meta";

/** What a person is asked to grant an installation before its view opens. */
const GrantPrompt = schema.object({
    /** The permissions the view asks the person to grant. */
    permissions: schema.array(GrantPermission.schema),
});

/** The machine package's failures, each reason with the call error code it answers. */
const machineError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    OBJECTS_MISSING: { code: "BAD_REQUEST" },
    RELEASE_NOT_BUILT: { code: "BAD_REQUEST" },
    ENDPOINT_INVALID: { code: "BAD_REQUEST" },
    REGISTRATION_INVALID: { code: "BAD_REQUEST" },
    TARGET_INVALID: { code: "BAD_REQUEST" },
    PATH_INVALID: { code: "BAD_REQUEST" },
    VERSION_INVALID: { code: "BAD_REQUEST" },
    LAUNCH_TOKEN_MISSING: { code: "BAD_REQUEST" },
    OUTSIDE_CHECKOUT: { code: "BAD_REQUEST" },
    POINT_WITHOUT_VALUE: { code: "BAD_REQUEST" },
    RELATIVE_DIRECTORY: { code: "BAD_REQUEST" },
    RELOCATION_TARGET_REQUIRED: { code: "BAD_REQUEST" },
    SETTING_SCOPE_MISMATCH: { code: "BAD_REQUEST" },
    METRIC_UNSUPPORTED: { code: "BAD_REQUEST" },

    // unauthorized
    FOREIGN_ORIGIN: { code: "UNAUTHORIZED" },
    INSTANCE_SECRET_INVALID: { code: "UNAUTHORIZED" },
    SIGN_IN_REQUIRED: { code: "UNAUTHORIZED" },
    TOKEN_WITHOUT_MACHINE: { code: "UNAUTHORIZED" },
    TOKEN_UNBOUND: { code: "UNAUTHORIZED" },
    COPY_CREDENTIAL_UNKNOWN: { code: "UNAUTHORIZED" },
    SESSION_INVALID: { code: "UNAUTHORIZED" },

    // forbidden
    CONNECTION_MISMATCH: { code: "FORBIDDEN" },
    FOREIGN_SCOPE_PUSH: { code: "FORBIDDEN" },
    FOREIGN_PARTITION_TOKEN: { code: "FORBIDDEN" },
    GRANT_REQUIRED: { code: "FORBIDDEN", data: GrantPrompt },
    OWNER_MISSING: { code: "FORBIDDEN" },
    ORIGIN_MISMATCH: { code: "FORBIDDEN" },
    OWNER_REQUIRED: { code: "FORBIDDEN" },
    PERSON_REQUIRED: { code: "FORBIDDEN" },
    ADDRESS_UNBOUND: { code: "FORBIDDEN" },
    BUCKET_UNBOUND: { code: "FORBIDDEN" },
    INSTALLATION_UNBOUND: { code: "FORBIDDEN" },
    VIEW_GRANT_REQUIRED: { code: "FORBIDDEN" },
    VIEW_PACKAGE_UNDECLARED: { code: "FORBIDDEN" },
    INSTALLATION_ORIGIN_REQUIRED: { code: "FORBIDDEN" },

    // not found
    SPACE_NOT_STORED: { code: "NOT_FOUND" },
    ACCOUNT_NOT_FOUND: { code: "NOT_FOUND" },
    CHECKOUT_NOT_FOUND: { code: "NOT_FOUND" },
    CHECKOUT_SPACE_ELSEWHERE: { code: "NOT_FOUND" },
    CLIENT_PATH_NOT_FOUND: { code: "NOT_FOUND" },
    DIRECTORY_NOT_FOUND: { code: "NOT_FOUND" },
    HOME_PATH_MISMATCH: { code: "NOT_FOUND" },
    INSTALLATION_NOT_FOUND: { code: "NOT_FOUND" },
    NOTHING_SERVED: { code: "NOT_FOUND" },
    CALLS_MISSING: { code: "NOT_FOUND" },
    EGRESS_ADDRESS_MISSING: { code: "NOT_FOUND" },
    PACKAGE_MOUNT_MISSING: { code: "NOT_FOUND" },
    PLATFORM_SERVICE_MISSING: { code: "NOT_FOUND" },
    PACKAGE_NOT_SERVED: { code: "NOT_FOUND" },
    PRESENTER_NOT_FOUND: { code: "NOT_FOUND" },
    PREVIEW_NOT_FOUND: { code: "NOT_FOUND" },
    ROUTE_NOT_FOUND: { code: "NOT_FOUND" },
    SIGNAL_UNKNOWN: { code: "NOT_FOUND" },
    VIEW_NOT_FOUND: { code: "NOT_FOUND" },

    // method not supported
    METHOD_UNSUPPORTED: { code: "METHOD_NOT_SUPPORTED" },

    // conflict
    SITE_AMBIGUOUS: { code: "CONFLICT" },
    SITE_RULE_INVALID: { code: "UNPROCESSABLE_CONTENT" },
    ALREADY_RUNNING: { code: "CONFLICT" },
    PERSON_AMBIGUOUS: { code: "CONFLICT" },
    BUILD_ENDED: { code: "CONFLICT" },
    ENROLLED: { code: "CONFLICT" },
    TREE_NOT_MOVED: { code: "CONFLICT" },

    // precondition failed
    LOGIN_NOT_SIGNED_IN: { code: "PRECONDITION_FAILED" },
    MACHINE_REVOKED: { code: "PRECONDITION_FAILED" },
    ENROLLMENT_REQUIRED: { code: "PRECONDITION_FAILED" },

    // misdirected request
    OBJECTS_MOVED: { code: "MISDIRECTED_REQUEST" },

    // too many requests
    RATE_LIMITED: { code: "TOO_MANY_REQUESTS", data: ERROR_DATA.TOO_MANY_REQUESTS },

    // internal server error
    COMMAND_FAILED: { code: "INTERNAL_SERVER_ERROR" },
    LISTENER_MISSING: { code: "INTERNAL_SERVER_ERROR" },
    COMMAND_MISSING: { code: "INTERNAL_SERVER_ERROR" },
    CONFIGURATION_INVALID: { code: "INTERNAL_SERVER_ERROR" },
    START_FAILED: { code: "INTERNAL_SERVER_ERROR" },
    STOP_FAILED: { code: "INTERNAL_SERVER_ERROR" },
    ROUTE_BASE_MISMATCH: { code: "INTERNAL_SERVER_ERROR" },
    SERVER_INVALID: { code: "INTERNAL_SERVER_ERROR" },

    // not implemented
    RUNTIME_MISSING: { code: "NOT_IMPLEMENTED" },
    SOCKETS_MISSING: { code: "NOT_IMPLEMENTED" },

    // bad gateway
    MODULE_NOT_SERVED: { code: "BAD_GATEWAY" },
    OBJECT_REFUSED: { code: "BAD_GATEWAY" },
    RUNTIME_ACTION_FAILED: { code: "BAD_GATEWAY" },
    SETTINGS_ENDED: { code: "BAD_GATEWAY" },

    // service unavailable
    INSTALLATION_NOT_SERVED: { code: "SERVICE_UNAVAILABLE" },
    INSTANCE_NOT_RUNNING: { code: "SERVICE_UNAVAILABLE" },
    MACHINE_STOPPED: { code: "SERVICE_UNAVAILABLE" },
    REVISION_MISSING: { code: "SERVICE_UNAVAILABLE" },
    RUNNING_DEPLOYMENT_MISSING: { code: "SERVICE_UNAVAILABLE" },
    WORKLOAD_MISSING: { code: "SERVICE_UNAVAILABLE" },
    RUNNER_NOT_SERVING: { code: "SERVICE_UNAVAILABLE" },
    SPACE_STARTING: { code: "SERVICE_UNAVAILABLE" },
    TRACE_REFUSED: { code: "SERVICE_UNAVAILABLE" },
    TELEMETRY_REFUSED: { code: "SERVICE_UNAVAILABLE" },

    // gateway timeout
    BUILD_TIMED_OUT: { code: "GATEWAY_TIMEOUT" },
});

/** The machine package's failures, and the refusal of what a machine does only once enrolled. */
export const MachineError = Object.assign(machineError, {
    /** Refuse what a machine does only once enrolled, telling the person how to enroll it. */
    unenrolled(action: string) {
        return machineError.of(
            "ENROLLMENT_REQUIRED",
            `${action} needs an enrolled machine: sign in on this machine first`,
        );
    },
});

/** A stable cause of machine failures. */
export type MachineReason = ReasonOf<typeof machineError>;
