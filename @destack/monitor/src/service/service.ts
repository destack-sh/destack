import { schema } from "@destack/schema";
import { defineProcedure, defineService, eventIterator } from "@destack/service";
import { Entry, EntryFilter, EntryPage, EntrySearch, PointSeries, Series } from "../entry/index.ts";
import { activity, announcement, subscription } from "@destack/notification";
import { comment, reaction } from "@destack/social";
import { alert, alertRule, issue } from "../object/index.ts";
import type {} from "@destack/package/import-meta";

/** A procedure that checks its entry permission on the installation in its handler. */
const procedure = defineProcedure({ authentication: "identity", permission: null, audit: false });

/** The monitor service, which searches, follows and traces entries and charts metrics beside OTLP ingestion, and serves issues, alert rules and alerts with their comments and notifications. */
export const monitorService = defineService("monitor", {
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
    series: procedure
        .route({ method: "POST", path: "/monitor/series" })
        .input(PointSeries)
        .output(Series),
    search: procedure
        .route({ method: "POST", path: "/monitor/search" })
        .input(EntrySearch)
        .output(EntryPage),
    tail: procedure
        .route({ method: "POST", path: "/monitor/tail" })
        .input(EntryFilter)
        .output(eventIterator(Entry)),
    trace: procedure
        .route({ method: "POST", path: "/monitor/trace" })
        .input(
            schema.object({
                /** The space of the installation, or the host whose own trace to read. */
                scope: schema.string().min(1),
                /** The installation whose trace to read, or the scope's host when absent. */
                installation: schema.identifier("installation").exactOptional(),
                /** The trace, as 32 hexadecimal digits. */
                trace: schema.string().regex(/^[0-9a-f]{32}$/u),
            }),
        )
        .output(schema.object({ entries: schema.array(Entry) })),
});
