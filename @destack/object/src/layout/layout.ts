import { schema } from "@destack/schema";
import type { TimeScale } from "@destack/schema";
import type { TreeColumns } from "@destack/db";
import type { Field } from "../field/field.ts";
import { BoardLayout } from "./board.ts";
import { CalendarLayout } from "./calendar.ts";
import { GalleryLayout } from "./gallery.ts";
import { TableLayout } from "./table.ts";
import { TimelineLayout } from "./timeline.ts";

import { TreeLayout } from "./tree.ts";

/** The fields, tree and declared layouts of an object type. */
export interface LayoutSource {
    /** The fields, by property. */
    readonly fields: Readonly<Record<string, Field>>;
    /** The tree over the type's own parent field, absent without one. */
    readonly tree: TreeColumns | undefined;
    /** Whether the type presents a cover, which a gallery shows. */
    readonly hasCover: boolean;
    /** The layouts the type declares beside the derived ones. */
    readonly declared: readonly Layout[];
}

/** How a list of objects draws: a table, a tree, a board, a calendar, a timeline or a gallery. */
const kinds = schema.discriminatedUnion("kind", [
    TableLayout,
    TreeLayout,
    BoardLayout,
    CalendarLayout,
    TimelineLayout,
    GalleryLayout,
]);
/** How a list of objects draws. */
export type Layout = schema.Infer<typeof kinds>;

/** How a list of objects draws: a table, a tree, a board, a calendar, a timeline or a gallery. */
export const Layout = Object.assign(kinds, {
    /** List the layouts of a type. */
    list(source: LayoutSource): Layout[] {
        // derive a tree from the type's own parent, a board per grouping field, a calendar per time field
        const fields = Object.entries(source.fields);
        const derived: Layout[] = [
            { kind: "table" },
            ...(source.tree === undefined
                ? []
                : [{ kind: "tree" as const, id: source.tree.id, parent: source.tree.parent }]),
            ...fields
                .filter(([, field]) => BoardLayout.isGrouping(field))
                .map(([property]) => ({
                    kind: "board" as const,
                    by: property,
                    empty: "show" as const,
                })),
            ...fields
                .filter(([, field]) => field.type === "time")
                .map(([property]) => ({ kind: "calendar" as const, by: property })),
            ...(source.hasCover ? [{ kind: "gallery" as const }] : []),
        ];

        // add each declared layout a derived one leaves out
        const keys = new Set(derived.map((layout) => JSON.stringify(layout)));

        return [
            ...derived,
            ...source.declared.filter((layout) => !keys.has(JSON.stringify(layout))),
        ];
    },

    /** Extend a list query to read what a layout shows, by its kind. */
    query<Query extends { readonly where?: object; readonly orderBy?: object }>(
        layout: Layout,
        query: Query,
        window?: TimeScale,
    ): Query {
        // extend it by the kinds that order or window their objects
        if (layout.kind === "board") {
            return BoardLayout.query(layout, query);
        } else if (layout.kind === "calendar") {
            return CalendarLayout.query(layout, query, window);
        } else if (layout.kind === "timeline") {
            return TimelineLayout.query(layout, query, window);
        }

        return query;
    },
});
