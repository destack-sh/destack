import { t } from "@destack/locale";
import type { OrderBy } from "@destack/db";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { createSignal } from "@destack/view";
import type { SortFields } from "./control.ts";
import { Sort, SortAdd, SortClear, SortList } from "./sort.tsx";

/** The styles of the examples. */
const styles = style.create({
    page: {
        display: "grid",
        gap: space[4],
    },
});

/** The fields of a task a person sorts by. */
const FIELDS: SortFields = {
    title: { label: "Title", type: { kind: "text" } },
    estimate: { label: "Estimate", type: { kind: "number" } },
    due: { label: "Due", type: { kind: "time" } },
    state: { label: "State" },
};

/** An order a person edits, as a query takes it. */
export const sortTasks = defineExample({
    of: Sort,
    name: "tasks",
    description: t`An order a person edits, as a query takes it.`,
    layout: "padded",
    render: () => {
        const [orderBy, setOrderBy] = createSignal<OrderBy>({ due: "desc", title: "asc" });

        return (
            <div {...style.attrs(styles.page)}>
                <Sort fields={FIELDS} value={orderBy()} onValueChange={setOrderBy} />
                <output>{JSON.stringify(orderBy())}</output>
            </div>
        );
    },
});

/** An order beside each other, its fields dragged. */
export const sortRow = defineExample({
    of: SortList,
    name: "row",
    description: t`An order beside each other, its fields dragged.`,
    layout: "padded",
    render: () => (
        <Sort fields={FIELDS} defaultValue={{ due: "asc", estimate: "desc" }}>
            <SortList />
            <SortAdd />
            <SortClear />
        </Sort>
    ),
});
