import type { JsonCondition } from "@destack/db";
import { type EventStore } from "@destack/event";
import { authorizeRead, type UnmaskedRead } from "@destack/event/server";
import { FilterText } from "@destack/event/service";
import { ServiceError } from "@destack/service";
import { implement, type ServiceContext } from "@destack/service/server";
import { log, metric, span } from "../event/index.ts";
import { MetricFold } from "../metric/fold.ts";
import { observabilityService } from "../service/index.ts";

/** The longest trace, beyond which a trace read cuts it, in microseconds. */
const TRACE_MICROSECONDS = 60 * 60 * 1_000_000;

/** The events one trace read returns of each kind. */
const TRACE_EVENTS = 1000;

/** Route observability's own reads beside the event service's: one trace's spans and logs, and percentiles of metric histograms, each as the caller may read telemetry. */
export function observabilityProcedures(
    events: EventStore,
    unmasked: (context: ServiceContext, read: UnmaskedRead) => Promise<void>,
) {
    const implementation = implement(observabilityService.router).$context<ServiceContext>();

    return {
        trace: implementation.trace.handler(async ({ input, context }) => {
            // read the trace's spans and logs from the time its identifier carries
            const selection = { ...input, within: true };
            const read = await authorizeRead(context, {
                store: events,
                kind: span,
                selection,
                operation: "trace",
                unmasked,
            });
            const from = Number.parseInt(input.trace.slice(0, 12), 16) * 1000;
            const filter = {
                scope: input.scope,
                where: { AND: [read.where, { trace: input.trace }] },
                from,
                before: from + TRACE_MICROSECONDS,
            };
            const spans = await events.query(span, filter, { limit: TRACE_EVENTS });
            const logs = await events.query(log, filter, { limit: TRACE_EVENTS });

            return { spans: spans.events.map(read.redact), logs: logs.events.map(read.redact) };
        }),
        percentiles: implementation.percentiles.handler(async ({ input, context }) => {
            // fold the selected histogram points into percentiles as the caller may read metrics
            const read = await authorizeRead(context, {
                store: events,
                kind: metric,
                selection: input,
                operation: "percentiles",
                unmasked,
            });
            const series = await MetricFold.series(
                events,
                {
                    scope: input.scope,
                    where: { AND: [read.where, ...conditionOf(input.where)] },
                    from: input.from,
                    before: input.before,
                },
                {
                    fold: input.fold,
                    group: input.group,
                    ...(input.step === undefined ? {} : { step: input.step }),
                },
            );

            return {
                series: series.map((each) => ({
                    group: { ...each.group },
                    steps: [...each.steps],
                })),
            };
        }),
    };
}

/** Parse a filter's text into the conditions it adds, refusing one that does not parse. */
function conditionOf(text: string | undefined): JsonCondition[] {
    try {
        const parsed = FilterText.condition(text);

        return parsed === undefined ? [] : [parsed];
    } catch (error) {
        throw new ServiceError("BAD_REQUEST", {
            message: error instanceof Error ? error.message : String(error),
        });
    }
}
