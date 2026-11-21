import { type JsonObject, type JsonValue, schema } from "@destack/schema";
import type { TimeScale } from "@destack/schema";
import { TreeLayout } from "./tree.ts";

/** The microseconds in a millisecond. */
const MICROSECONDS_PER_MILLISECOND = 1000;

/** The units a timeline's times and durations count in. */
export const TimeUnit = schema.enum(["millisecond", "microsecond"]);
/** The unit a timeline's times and durations count in. */
export type TimeUnit = schema.Infer<typeof TimeUnit>;

/** One object on a timeline's axis, in milliseconds. */
export interface TimelineEntry {
    /** The object. */
    readonly row: JsonObject;
    /** When it starts. */
    readonly start: number;
    /** When it ends. */
    readonly end: number;
}

/** A timeline's axis: the window its objects span and each object on it. */
export interface TimelineAxis {
    /** The window from the first start to the last end. */
    readonly window: TimeScale;
    /** The objects with a start and an end, in the order given. */
    readonly entries: readonly TimelineEntry[];
}

/** Where a timeline's bar ends: at a time field, or after a duration field. */
const TimelineEnd = schema.union([
    schema.object({
        /** The field with the end time. */
        field: schema.string().min(1),
    }),
    schema.object({
        /** The field with the duration from the start. */
        duration: schema.string().min(1),
    }),
]);

/** The value of a field that marks an object failed, such as a span's error status. */
const TimelineFailure = schema.object({
    /** The field. */
    field: schema.string().min(1),
    /** The value marking a failure. */
    value: schema.union([schema.string(), schema.number(), schema.boolean()]),
});

/** One bar per object on a shared time axis. */
const timelineLayout = schema.object({
    /** The kind. */
    kind: schema.literal("timeline"),
    /** The field with the start time. */
    start: schema.string().min(1),
    /** Where the bar ends. */
    end: TimelineEnd,
    /** The unit of the times and the duration. */
    unit: TimeUnit,
    /** The field value that marks an object failed, none when objects cannot fail. */
    failed: TimelineFailure.exactOptional(),
});
/** One bar per object on a shared time axis. */
export type TimelineLayout = schema.Infer<typeof timelineLayout>;

/** One bar per object on a shared time axis. */
export const TimelineLayout = Object.assign(timelineLayout, {
    /** Read an object's start and end in milliseconds, absent for an object without them. */
    period(
        layout: TimelineLayout,
        row: JsonObject,
    ): { readonly start: number; readonly end: number } | undefined {
        // read the start, then the end by its field or after the duration
        const start = milliseconds(row[layout.start], layout.unit);
        const after =
            "field" in layout.end
                ? milliseconds(row[layout.end.field], layout.unit)
                : milliseconds(row[layout.end.duration], layout.unit);
        if (start === undefined || after === undefined) {
            return undefined;
        }

        return { start, end: "field" in layout.end ? after : start + after };
    },

    /** Place objects with a start and an end on one axis, absent when none has both. */
    axis(layout: TimelineLayout, rows: readonly JsonObject[]): TimelineAxis | undefined {
        // read each object's period
        const timed = rows.flatMap((row) => {
            const period = TimelineLayout.period(layout, row);

            return period === undefined ? [] : [{ row, ...period }];
        });

        // span the periods, at least one millisecond long
        if (timed.length === 0) {
            return undefined;
        }
        const start = Math.min(...timed.map((each) => each.start));
        const end = Math.max(...timed.map((each) => each.end));
        const length = Math.max(1, end - start);

        return { window: { start, end: start + length }, entries: timed };
    },

    /** Tell whether an object failed, by the layout's failure field. */
    isFailed(layout: TimelineLayout, row: JsonObject): boolean {
        return layout.failed !== undefined && row[layout.failed.field] === layout.failed.value;
    },

    /** Read the objects on the critical path: the chain of children that sets each parent's end. */
    critical(
        layout: TimelineLayout,
        tree: TreeLayout,
        rows: readonly JsonObject[],
    ): ReadonlySet<JsonObject> {
        // read each timed object's period under its parent
        const timed = rows.flatMap((row) => {
            const period = TimelineLayout.period(layout, row);

            return period === undefined ? [] : [{ row, ...period }];
        });
        const roots = new Set(
            TreeLayout.nest(
                tree,
                timed.map((each) => each.row),
            )
                .filter((each) => each.isLast.length === 0)
                .map((each) => each.row),
        );
        const children = Map.groupBy(
            timed.filter((each) => !roots.has(each.row)),
            (each) => JSON.stringify(each.row[tree.parent]),
        );

        // start at the root that ends last
        const critical = new Set<JsonObject>();
        const last = timed
            .filter((each) => roots.has(each.row))
            .toSorted((left, right) => right.end - left.end)[0];
        if (last !== undefined) {
            markCritical(last, tree, children, critical);
        }

        return critical;
    },

    /** Extend a list query to read the objects starting before a window ends, in start order. */
    query<Query extends { readonly where?: object; readonly orderBy?: object }>(
        layout: TimelineLayout,
        query: Query,
        window?: TimeScale,
    ): Query {
        // read the objects starting before the window ends
        const before = window === undefined ? undefined : { [layout.start]: { lt: window.end } };
        const where =
            before === undefined
                ? query.where
                : query.where === undefined
                  ? before
                  : { AND: [query.where, before] };

        return {
            ...query,
            ...(where === undefined ? {} : { where }),
            orderBy: { [layout.start]: "asc", ...query.orderBy },
        };
    },
});

/** An object with its period in milliseconds. */
interface TimedRow {
    /** The object. */
    readonly row: JsonObject;
    /** When it starts. */
    readonly start: number;
    /** When it ends. */
    readonly end: number;
}

/** Mark an object critical, then the children on its critical path. */
function markCritical(
    entry: TimedRow,
    tree: TreeLayout,
    children: ReadonlyMap<string, readonly TimedRow[]>,
    critical: Set<JsonObject>,
): void {
    // mark the object, then each child that ends before the last marked one starts
    critical.add(entry.row);
    const below = (children.get(JSON.stringify(entry.row[tree.id])) ?? []).toSorted(
        (left, right) => right.end - left.end,
    );
    let cursor = entry.end;
    for (const child of below) {
        if (child.end <= cursor) {
            markCritical(child, tree, children, critical);
            cursor = child.start;
        }
    }
}

/** Convert a time or duration in a unit to milliseconds, absent for a value that is no number. */
function milliseconds(value: JsonValue | undefined, unit: TimeUnit): number | undefined {
    // read a number in its unit
    if (typeof value !== "number") {
        return undefined;
    }

    return unit === "microsecond" ? value / MICROSECONDS_PER_MILLISECOND : value;
}
