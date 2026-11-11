import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The desktop package's failures, each reason with the call error code it answers. */
export const DesktopError = defineErrorFamily({
    reasons: {
        // not found
        DISTRIBUTION_NOT_FOUND: {},
        INSTALLATION_NOT_FOUND: {},
        MENU_ITEM_NOT_FOUND: {},
        SHORTCUT_NOT_FOUND: {},

        // invalid
        EVENT_INVALID: {},

        // unauthenticated
        SIGN_IN_UNAUTHENTICATED: {},

        // blocked
        APPLICATION_BLOCKED: {},
        DESKTOP_BLOCKED: {},

        // broken
        APPLICATION_LAUNCH_BROKEN: {},
    },
});

/** A stable cause of desktop failures. */
export type DesktopReason = ReasonOf<typeof DesktopError>;
