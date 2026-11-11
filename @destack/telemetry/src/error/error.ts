import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The telemetry package's failures, each reason with the call error code it answers. */
export const TelemetryError = defineErrorFamily(import.meta.destack.package, {
    // invalid
    EXPORT_INVALID: {},

    // denied
    EMITTER_ATTRIBUTE_DENIED: {},
});
/** A stable cause of telemetry failures. */
export type TelemetryReason = ReasonOf<typeof TelemetryError>;
