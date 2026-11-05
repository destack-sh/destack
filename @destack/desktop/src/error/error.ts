import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The desktop package's failures, each reason with the call error code it answers. */
export const DesktopError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    EVENT_INVALID: { code: "BAD_REQUEST" },

    // unauthorized
    SIGN_IN_REQUIRED: { code: "UNAUTHORIZED" },

    // not found
    MENU_ITEM_NOT_FOUND: { code: "NOT_FOUND" },
    SHORTCUT_NOT_FOUND: { code: "NOT_FOUND" },
    INSTALLATION_NOT_FOUND: { code: "NOT_FOUND" },
    DISTRIBUTION_NOT_FOUND: { code: "NOT_FOUND" },

    // conflict
    DESKTOP_RUNNING: { code: "CONFLICT" },

    // precondition failed
    APPLICATION_UNAVAILABLE: { code: "PRECONDITION_FAILED" },

    // internal server error
    APPLICATION_LAUNCH_FAILED: { code: "INTERNAL_SERVER_ERROR" },
});

/** A stable cause of desktop failures. */
export type DesktopReason = ReasonOf<typeof DesktopError>;
