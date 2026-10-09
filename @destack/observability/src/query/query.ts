import { Event, SERIES_FOLDS } from "@destack/event";
import { EventSeries } from "@destack/event/service";
import { defineSchema, schema } from "@destack/schema";
import { QUANTILE_FOLDS } from "../metric/fold.ts";

/** The folds alerts compare: the event store's, and the percentiles of histogram points. */
export const EVENT_FOLDS = [...SERIES_FOLDS, ...QUANTILE_FOLDS] as const;

/** A percentile fold of the metric histograms a selection names, per step and group. */
export const PercentileSeries = defineSchema(
    EventSeries.omit({ kind: true, measure: true, fold: true }).extend({
        /** The percentile. */
        fold: schema.enum(QUANTILE_FOLDS),
    }),
);
/** A percentile fold of metric histograms. */
export type PercentileSeries = schema.Infer<typeof PercentileSeries>;

/** A read of one trace's spans and log records. */
export const TraceRequest = defineSchema(
    schema.object({
        /** The space of the installation, or the machine whose own trace to read. */
        scope: schema.string().min(1),
        /** The installation whose trace to read, every installation's and the scope's own when absent. */
        object: schema.identifier("installation").exactOptional(),
        /** The trace, as 32 hexadecimal digits. */
        trace: schema.string().regex(/^[0-9a-f]{32}$/u),
    }),
);
/** A read of one trace. */
export type TraceRequest = schema.Infer<typeof TraceRequest>;

/** One trace's spans and log records, each in start order. */
export const TraceResult = defineSchema(
    schema.object({
        /** The spans. */
        spans: schema.array(Event),
        /** The log records emitted in the trace. */
        logs: schema.array(Event),
    }),
);
/** One trace's spans and log records. */
export type TraceResult = schema.Infer<typeof TraceResult>;
