import { SeriesResult } from "@destack/event/service";
import { defineProcedure, defineService } from "@destack/service";
import { activity, announcement, subscription } from "@destack/notification";
import { comment, reaction } from "@destack/social";
import { alert, alertRule, issue } from "../object/index.ts";
import { PercentileSeries, TraceRequest, TraceResult } from "../query/index.ts";
import type {} from "@destack/package/import-meta";

/** A procedure that reads telemetry. */
const procedure = defineProcedure({ authentication: "identity", permission: "rows", audit: false });

/** The observability service. */
export const observabilityService = defineService("observability", {
    objects: {
        issue,
        alertRule,
        alert,
        comment,
        reaction,
        subscription,
        activity,
        announcement,
    },
    percentiles: procedure
        .route({ method: "POST", path: "/observability/percentiles" })
        .input(PercentileSeries)
        .output(SeriesResult),
    trace: procedure
        .route({ method: "POST", path: "/observability/trace" })
        .input(TraceRequest)
        .output(TraceResult),
});
