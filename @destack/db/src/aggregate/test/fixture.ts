import { boolean, defineTable, integer, text } from "../../index.ts";

/** The columns of a list. */
const listColumns = {
    /** The list's identifier. */
    id: text("id").primaryKey(),
    /** How many items the list holds. */
    items: integer("items").notNull().default(0),
    /** How many of its items are done. */
    done: integer("done").notNull().default(0),
    /** The sum of its items' points. */
    points: integer("points").notNull().default(0),
    /** The most points one of its items holds. */
    largest: integer("largest"),
};

/** Lists keeping aggregates of their items. */
export const list = defineTable("aggregate_list", listColumns);

/** The columns of an item. */
const itemColumns = {
    /** The item's identifier. */
    id: text("id").primaryKey(),
    /** The list holding the item. */
    listId: text("list_id").notNull(),
    /** The item's points. */
    points: integer("points").notNull(),
    /** Whether the item is done. */
    isDone: boolean("is_done").notNull(),
};

/** Items of a list, counted, summed and bounded by the list. */
export const item = defineTable("aggregate_item", itemColumns, {
    aggregates: [
        { into: () => list, column: "items", key: "listId", function: "count" },
        {
            into: () => list,
            column: "done",
            key: "listId",
            function: "count",
            where: { isDone: true },
        },
        { into: () => list, column: "points", key: "listId", function: "sum", value: "points" },
        { into: () => list, column: "largest", key: "listId", function: "max", value: "points" },
    ],
});

/** The same items before the lists keep any aggregate of them. */
export const plainItem = defineTable("aggregate_item", itemColumns);

/** The tables of the aggregate example. */
export const lists = [item, list];
