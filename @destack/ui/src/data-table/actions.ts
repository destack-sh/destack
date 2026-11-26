import { t } from "@destack/locale";
import { defineAction } from "@destack/object";

/** Sort the column from its lowest value. */
export const sortAscending = defineAction({
    name: "column.sortAscending",
    title: t.context("action")`Sort ascending`,
    category: t.context("title")`Table`,
});

/** Sort the column from its highest value. */
export const sortDescending = defineAction({
    name: "column.sortDescending",
    title: t.context("action")`Sort descending`,
    category: t.context("title")`Table`,
});

/** Clear the column's sort. */
export const clearSort = defineAction({
    name: "column.clearSort",
    title: t.context("action")`Clear sort`,
    category: t.context("title")`Table`,
});

/** Pin the column to the start edge. */
export const pinToStart = defineAction({
    name: "column.pinToStart",
    title: t`Pin to start`,
    category: t.context("title")`Table`,
});

/** Pin the column to the end edge. */
export const pinToEnd = defineAction({
    name: "column.pinToEnd",
    title: t`Pin to end`,
    category: t.context("title")`Table`,
});

/** Unpin the column. */
export const unpin = defineAction({
    name: "column.unpin",
    title: t.context("action")`Unpin`,
    category: t.context("title")`Table`,
});

/** Move the column to the left. */
export const moveColumnLeft = defineAction({
    name: "column.moveLeft",
    title: t.context("action")`Move left`,
    category: t.context("title")`Table`,
});

/** Move the column to the right. */
export const moveColumnRight = defineAction({
    name: "column.moveRight",
    title: t.context("action")`Move right`,
    category: t.context("title")`Table`,
});
