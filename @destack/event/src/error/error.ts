import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The event package's failures, each reason with the call error code it answers. */
export const EventError = defineErrorFamily(import.meta.destack.package, {
    // conflict
    UPDATE_CONFLICT: {},

    // invalid
    CURSOR_INVALID: {},
    SERIES_INVALID: {},

    // blocked
    CONTENTS_BLOCKED: {},
});

/** A stable cause of event failures. */
export type EventReason = ReasonOf<typeof EventError>;
