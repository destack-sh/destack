import { Condition, OrderBy } from "@destack/db";
import { Message, t } from "@destack/locale";
import { schema } from "@destack/schema";
import { Layout } from "../layout/layout.ts";
import type { TimeWindow } from "../layout/timeline.ts";

/** A view's name: lowercase words joined by hyphens, as deep links and flags write it. */
const VIEW_NAME = schema.string().regex(/^[a-z0-9]+(?:-[a-z0-9]+)*$/u);

/** One way of looking at a type's objects: a filter, an order, a layout and the fields shown. */
const objectView = schema.object({
    /** The view's name, such as `board-by-state`, which flags and deep links pick it by. */
    name: VIEW_NAME,
    /** The view's title as people read it, such as Board by state. */
    title: Message,
    /** The condition the objects shown meet, every object when absent. */
    where: Condition.schema.exactOptional(),
    /** How the objects sort before the layout's own order. */
    orderBy: OrderBy.schema.exactOptional(),
    /** How the objects draw. */
    layout: Layout,
    /** The fields shown beside the title, the layout's default when absent. */
    fields: schema.array(schema.string().min(1)).exactOptional(),
});
/** One way of looking at a type's objects. */
export type ObjectView = schema.Infer<typeof objectView>;

/** One way of looking at a type's objects, offered by a type and read by a query. */
export const ObjectView = Object.assign(objectView, {
    /** List the views a type offers: one per offered layout, named by its kind and field. */
    offered(layouts: readonly Layout[], declared: readonly ObjectView[]): ObjectView[] {
        // take each declared view, then a view per offered layout no declared view draws
        const drawn = new Set(declared.map((view) => JSON.stringify(view.layout)));
        const derived = layouts
            .filter((layout) => !drawn.has(JSON.stringify(layout)))
            .map((layout) => ObjectView.of(layout));

        return [...declared, ...derived];
    },

    /** Derive a layout's view, named and titled by its kind and field. */
    of(layout: Layout): ObjectView {
        const name = "by" in layout ? ObjectView.nameOf(layout.kind, "by", layout.by) : layout.kind;

        return { name, title: ObjectView.titleOf(layout), layout };
    },

    /** Write camel-case words as a view name, such as `board-by-due-at`. */
    nameOf(...words: readonly string[]): string {
        return words
            .map((word) => word.replaceAll(/[A-Z]/gu, (upper) => `-${upper.toLowerCase()}`))
            .join("-");
    },

    /** Title a layout's view by its kind and its field, such as Board by state. */
    titleOf(layout: Layout): Message {
        // title a grouping or placing layout by its field, any other by its kind
        if (layout.kind === "board") {
            return t`Board by ${layout.by}`;
        } else if (layout.kind === "calendar") {
            return t`Calendar by ${layout.by}`;
        } else if (layout.kind === "tree") {
            return t`Tree`;
        } else if (layout.kind === "timeline") {
            return t`Timeline`;
        } else if (layout.kind === "gallery") {
            return t`Gallery`;
        }

        return t`Table`;
    },

    /** Extend a list query to read what a view shows. */
    query<Query extends { readonly where?: object; readonly orderBy?: object }>(
        view: ObjectView,
        query: Query,
        window?: TimeWindow,
    ): Query {
        // meet the view's condition and order before the layout's
        const where =
            view.where === undefined
                ? query.where
                : query.where === undefined
                  ? view.where
                  : { AND: [query.where, view.where] };
        const ordered = {
            ...query,
            ...(where === undefined ? {} : { where }),
            ...(view.orderBy === undefined
                ? {}
                : { orderBy: { ...view.orderBy, ...query.orderBy } }),
        };

        return Layout.query(view.layout, ordered, window);
    },
});

/** Write a field's name in lowercase words joined by hyphens, as a view key holds it. */
export function kebab(name: string): string {
    return name.replaceAll(/[A-Z]/gu, (upper) => `-${upper.toLowerCase()}`);
}
