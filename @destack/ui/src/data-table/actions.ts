import { t } from "@destack/locale";
import { defineAction } from "@destack/object";

/** Sort the column from its lowest value. */
export const sortAscending = defineAction({
    name: "column.sortAscending",
    title: t`Sort ascending`,
    category: t`Table`,
});

/** Sort the column from its highest value. */
export const sortDescending = defineAction({
    name: "column.sortDescending",
    title: t`Sort descending`,
    category: t`Table`,
});

/** Clear the column's sort. */
export const clearSort = defineAction({
    name: "column.clearSort",
    title: t`Clear sort`,
    category: t`Table`,
});

/** Pin the column to the start edge. */
export const pinToStart = defineAction({
    name: "column.pinToStart",
    title: t`Pin to start`,
    category: t`Table`,
});

/** Pin the column to the end edge. */
export const pinToEnd = defineAction({
    name: "column.pinToEnd",
    title: t`Pin to end`,
    category: t`Table`,
});

/** Unpin the column. */
export const unpin = defineAction({ name: "column.unpin", title: t`Unpin`, category: t`Table` });

/** Move the column to the left. */
export const moveColumnLeft = defineAction({
    name: "column.moveLeft",
    title: t`Move left`,
    category: t`Table`,
});

/** Move the column to the right. */
export const moveColumnRight = defineAction({
    name: "column.moveRight",
    title: t`Move right`,
    category: t`Table`,
});
