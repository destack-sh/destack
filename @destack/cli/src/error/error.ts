import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The cli package's failures, each reason with the call error code it answers. */
export const CliError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    USAGE_INVALID: { code: "BAD_REQUEST" },
    SPACE_REQUIRED: { code: "BAD_REQUEST" },
    RESOURCE_LIMIT_EXCEEDED: { code: "BAD_REQUEST" },

    // unauthorized
    SIGN_IN_REQUIRED: { code: "UNAUTHORIZED" },

    // forbidden

    // not found
    NOUN_NOT_FOUND: { code: "NOT_FOUND" },
    METHOD_MISSING: { code: "NOT_FOUND" },
    INSTALLATION_MISSING: { code: "NOT_FOUND" },
    SETTING_NOT_DECLARED: { code: "NOT_FOUND" },

    // conflict
    INSTALLATION_FAILED: { code: "CONFLICT" },
    INSTALLATION_NAME_TAKEN: { code: "CONFLICT" },
    PLAN_NOT_WAITING: { code: "CONFLICT" },
    METHOD_AMBIGUOUS: { code: "CONFLICT" },

    // precondition failed
    APPROVAL_REQUIRED: { code: "PRECONDITION_FAILED" },
    SETTING_AMBIGUOUS: { code: "PRECONDITION_FAILED" },
    INSTALLATION_MISMATCH: { code: "PRECONDITION_FAILED" },
    INSTALLED_DISTRIBUTION_MISSING: { code: "PRECONDITION_FAILED" },
    COMMAND_UNAVAILABLE: { code: "PRECONDITION_FAILED" },

    // internal server error
    MACHINE_START_FAILED: { code: "INTERNAL_SERVER_ERROR" },

    // service unavailable
    FOLLOW_STOPPED: { code: "SERVICE_UNAVAILABLE" },

    // gateway timeout
    RESTART_TIMED_OUT: { code: "GATEWAY_TIMEOUT" },
});

/** A stable cause of cli failures. */
export type CliReason = ReasonOf<typeof CliError>;
