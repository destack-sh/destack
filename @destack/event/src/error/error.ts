import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The event package's failures, each reason with the call error code it answers. */
export const EventError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    INVALID_CURSOR: { code: "BAD_REQUEST" },
    INVALID_SERIES: { code: "BAD_REQUEST" },

    // conflict
    CONTENTS_DIFFER: { code: "CONFLICT" },

    // service unavailable
    CONCURRENT_UPDATE: { code: "SERVICE_UNAVAILABLE" },
});

/** A stable cause of event failures. */
export type EventReason = ReasonOf<typeof EventError>;
