import { Condition, type JsonCondition, type Row } from "@destack/db";
import { SERIES_FOLDS } from "@destack/event";
import { defineSchema, schema } from "@destack/schema";
import type { Call } from "../method/call.ts";
import { ObjectType } from "../object/object.ts";
import { method } from "../method/record.ts";
import { EventLog } from "../query/event.ts";
import { derivePermissions, type Trait } from "./trait.ts";
import { Execution, type StoreExecution } from "../method/execution.ts";

/** The most keys a series groups by: four keys of a few values each stay a readable chart. */
const MAX_GROUP_KEYS = 4;

/** The narrowest step, a minute: the finest resolution a chart of a day reads. */
const MIN_STEP_MILLISECONDS = 60_000;

/** A fold of the scope's events in a time range, per step and group of key values. */
export const SeriesQuery = defineSchema(
    schema.object({
        /** The events folded, every event in the range when absent. */
        where: Condition.schema.exactOptional(),
        /** The fold. */
        fold: schema.enum(SERIES_FOLDS),
        /** The key folded, needed by every fold but count and quantiles. */
        measure: schema.string().min(1).exactOptional(),
        /** The keys whose values each get a series of their own. */
        group: schema.array(schema.string().min(1)).max(MAX_GROUP_KEYS),
        /** The step width in milliseconds, one step over the range when absent. */
        step: schema.number().int().min(MIN_STEP_MILLISECONDS).exactOptional(),
        /** The earliest time, in Unix microseconds. */
        from: schema.number().int(),
        /** The time the events come before, in Unix microseconds. */
        before: schema.number().int(),
    }),
);

/** The folded series, each group's steps oldest first. */
export const SeriesResult = defineSchema(
    schema.object({
        /** The series, one per group of key values. */
        series: schema.array(
            schema.object({
                /** The value of each grouping key. */
                group: schema.record(
                    schema.string(),
                    schema.union([schema.string(), schema.number()]).nullable(),
                ),
                /** The steps with events. */
                steps: schema.array(
                    schema.object({
                        /** The step's start, in Unix microseconds. */
                        start: schema.number(),
                        /** The folded value. */
                        value: schema.number(),
                        /** The events folded. */
                        events: schema.number().int(),
                    }),
                ),
            }),
        ),
    }),
);

/** Fold the events a condition selects by keys the caller may read, per step and group. */
const seriesMethod = Execution.handle(
    method.query({
        permission: "read",
        target: "scope",
        selected: (input) => series.selectedKeys(SeriesQuery.parse(input).where),
        input: SeriesQuery,
        output: SeriesResult,
    }),
    (call, execution) => series.fold(call, Execution.store(execution, call)),
);

/** The series method an events type derives, none for another storage. */
export type SeriesMethodMap<Storage> = Storage extends "events"
    ? { readonly series: typeof seriesMethod }
    : {};

/** Events types, folded into series over time by the readers of the keys a fold selects. */
export const series: Trait<{}> & {
    /** Fold the scope's events in the range per step and group, which reveals no single event. */
    readonly fold: (
        call: Call,
        execution: StoreExecution,
    ) => Promise<schema.Infer<typeof SeriesResult>>;
    /** Read the key values a series condition requires at its top level. */
    readonly selectedKeys: (where: JsonCondition | undefined) => Row;
} = {
    options: (definition) =>
        ObjectType.storageKind(definition.storage) === "events" ? {} : undefined,
    columns: () => ({}),
    constraints: () => [],
    policy: (_options, _object, permissions) => derivePermissions(permissions, ["read"]),
    methods: () => ({ series: seriesMethod }),
    async fold(call: Call, execution: StoreExecution): Promise<schema.Infer<typeof SeriesResult>> {
        // select the range's events the condition selects
        const kind = call.object.eventKind();
        const {
            where,
            fold: folding,
            measure,
            group,
            step,
            from,
            before,
        } = SeriesQuery.parse(call.input);
        const range = { time: { gte: from, lt: before } };
        const filter = {
            scopes: await EventLog.scopes(call, Execution.snapshot(execution).database),
            where: { AND: [range, ...(where === undefined ? [] : [where])] },
        };

        // fold them per step and group
        const folded = await execution.store.events.series(kind, filter, {
            fold: folding,
            group,
            ...(measure === undefined ? {} : { measure }),
            ...(step === undefined ? {} : { step }),
        });

        return {
            series: folded.map((each) => ({ group: { ...each.group }, steps: [...each.steps] })),
        };
    },
    selectedKeys(where: JsonCondition | undefined): Row {
        return Object.fromEntries(
            Condition.conjuncts(where).flatMap((condition) =>
                Object.entries(condition).filter(
                    ([name, value]) =>
                        name !== "AND" && (typeof value === "string" || typeof value === "number"),
                ),
            ),
        );
    },
};
