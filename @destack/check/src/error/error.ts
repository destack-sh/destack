import { defineErrorFamily, type ReasonOf } from "@destack/error";

/** The package name: check runs outside the Destack build, where no metadata is injected. */
const DOMAIN = "@destack/check";

/** The check package's failures, each reason with the call error code it answers. */
export const CheckError = defineErrorFamily(
    { name: DOMAIN },
    {
        // bad request
        CONFIGURATION_INVALID: { code: "BAD_REQUEST" },

        // internal server error
        LANGUAGE_SERVICE_FAILED: { code: "INTERNAL_SERVER_ERROR" },
        TOOL_FAILED: { code: "INTERNAL_SERVER_ERROR" },
    },
    { audience: "host" },
);

/** A stable cause of check failures. */
export type CheckReason = ReasonOf<typeof CheckError>;
