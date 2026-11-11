import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The cli package's failures, each reason with the call error code it answers. */
export const CliError = defineErrorFamily({
    reasons: {
        // not found

        INSTALLATION_NOT_FOUND: {},

        METHOD_NOT_FOUND: {},

        NOUN_NOT_FOUND: {},

        SETTING_NOT_FOUND: {},

        // exists

        INSTALLATION_NAME_EXISTS: {},

        // invalid

        METHOD_INVALID: {},

        RESOURCE_LIMIT_INVALID: {},

        SETTING_INVALID: {},

        USAGE_INVALID: {},

        // required

        SPACE_REQUIRED: {},

        // unauthenticated

        SIGN_IN_UNAUTHENTICATED: {},

        // blocked

        APPROVAL_BLOCKED: {},

        COMMAND_BLOCKED: {},

        DISTRIBUTION_BLOCKED: {},

        INSTALLATION_BLOCKED: {},

        PLAN_BLOCKED: {},

        // unavailable

        FOLLOW_UNAVAILABLE: {},

        // unreachable

        RESTART_UNREACHABLE: {},

        // broken

        MACHINE_START_BROKEN: {},

        SETTING_BROKEN: {},
    },
});

/** A stable cause of cli failures. */
export type CliReason = ReasonOf<typeof CliError>;
