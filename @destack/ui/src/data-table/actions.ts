import { t } from "@destack/locale";
import { type Action, defineAction } from "@destack/object";

/** Sort the column from its lowest value. */
export const sortAscending = defineAction({
    name: "sort-ascending",
    title: t`Sort ascending`,
    category: t`Table`,
});

/** Sort the column from its highest value. */
export const sortDescending = defineAction({
    name: "sort-descending",
    title: t`Sort descending`,
    category: t`Table`,
});

/** Clear the column's sort. */
export const clearSort = defineAction({
    name: "clear-sort",
    title: t`Clear sort`,
    category: t`Table`,
});

/** Pin the column to the start edge. */
export const pinToStart = defineAction({
    name: "pin-to-start",
    title: t`Pin to start`,
    category: t`Table`,
});

/** Pin the column to the end edge. */
export const pinToEnd = defineAction({
    name: "pin-to-end",
    title: t`Pin to end`,
    category: t`Table`,
});

/** Unpin the column. */
export const unpin = defineAction({ name: "unpin", title: t`Unpin`, category: t`Table` });

/** Move the column to the left. */
export const moveColumnLeft = defineAction({
    name: "move-column-left",
    title: t`Move left`,
    category: t`Table`,
});

/** Move the column to the right. */
export const moveColumnRight = defineAction({
    name: "move-column-right",
    title: t`Move right`,
    category: t`Table`,
});

/** An action a table's owner runs on rows, in the row's menu, the bulk bar and by its keys. */
export interface DataTableRowAction {
    /** The action. */
    readonly action: Action;
    /** Report whether it runs on some rows, any by default. */
    readonly isEnabled?: (rows: readonly string[]) => boolean;
    /** Run it on some rows. */
    readonly run: (rows: readonly string[]) => void;
}
