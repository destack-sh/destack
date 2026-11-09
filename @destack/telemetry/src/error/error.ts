import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The telemetry package's failures, each reason with the call error code it answers. */
export const TelemetryError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    EXPORT_INVALID: { code: "BAD_REQUEST" },

    // forbidden
    EMITTER_ATTRIBUTE_RESERVED: { code: "FORBIDDEN" },
});
/** A stable cause of telemetry failures. */
export type TelemetryReason = ReasonOf<typeof TelemetryError>;
