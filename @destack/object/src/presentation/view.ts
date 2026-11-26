import { IconName } from "@destack/icon/name";
import { Condition, type Moment, OrderBy } from "@destack/db";
import { Message, t } from "@destack/locale";

import { schema } from "@destack/schema";
import type { TimeScale } from "@destack/schema";
import { Layout } from "../layout/layout.ts";

/** A view's name: lowercase words joined by hyphens, as deep links and flags write it. */
const VIEW_NAME = schema.string().regex(/^[a-z0-9]+(?:-[a-z0-9]+)*$/u);

/** One way of looking at a type's objects: a filter, an order, a layout and the fields shown. */
const objectView = schema.object({
    /** The view's name, such as `board-by-state`, which flags and deep links pick it by. */
    name: VIEW_NAME,
    /** The view's title as people read it, such as Board by state. */
    title: Message,
    /** The icon beside the view's title, such as in a view switcher. */
    icon: IconName.exactOptional(),
    /** One sentence on what the view shows. */
    description: Message.exactOptional(),
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

/** One way of looking at a type's objects. */
export const ObjectView = Object.assign(objectView, {
    /** List the views of a type. */
    list(layouts: readonly Layout[], declared: readonly ObjectView[]): ObjectView[] {
        // take each declared view, then a view per layout without one
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
            return t.context("option")`Tree`;
        } else if (layout.kind === "timeline") {
            return t.context("option")`Timeline`;
        } else if (layout.kind === "gallery") {
            return t.context("option")`Gallery`;
        }

        return t.context("option")`Table`;
    },

    /** Extend a list query to read what a view shows, its times bound to a moment. */
    query<Query extends { readonly where?: object; readonly orderBy?: object }>(
        view: ObjectView,
        query: Query,
        moment: Moment,
        window?: TimeScale,
    ): Query {
        // meet the view's condition, its times bound, and its order before the layout's
        const bound = view.where === undefined ? undefined : Condition.at(view.where, moment);
        const where =
            bound === undefined
                ? query.where
                : query.where === undefined
                  ? bound
                  : { AND: [query.where, bound] };
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
