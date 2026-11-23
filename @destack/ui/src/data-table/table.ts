import {
    constructTable,
    type RowData,
    type Table,
    type Table_CellSelection,
    type Table_RowPagination,
    type TableFeatures,
    type TableOptions,
    type TableState,
    type TableState_ColumnFiltering,
    type TableState_ColumnGrouping,
    type TableState_GlobalFiltering,
    type TableState_RowSorting,
} from "@tanstack/table-core";
import { storeReactivityBindings } from "@tanstack/table-core/store-reactivity-bindings";
import { type Accessor, createEffect, createSignal, onCleanup, untrack } from "@destack/view";

/** The defaults of a table whose rows go by their ids: new data leaves the page, and one block. */
const DEFAULTS = {
    autoResetPageIndex: false,
    autoResetExpanded: false,
    autoResetCellSelection: false,
    enableMultiCellRangeSelection: false,
};

/** A headless table whose state its components track. */
export type DataTableInstance<Features extends TableFeatures, Row extends RowData> = Table<
    Features,
    Row
> & {
    /** The table's state, read to track every change of its state and options. */
    readonly tracked: Accessor<TableState<Features>>;
};

/** Create a headless table over reactive options, its page and cells unchanged by new data. */
export function createTable<Features extends TableFeatures, Row extends RowData>(
    options: TableOptions<Features, Row>,
): DataTableInstance<Features, Row> {
    // run the table on synchronous atoms so its own reads see its writes at once
    const table = constructTable<Features, Row>({
        ...DEFAULTS,
        ...untrack(() => ({ ...options })),
        features: {
            ...options.features,
            coreReactivityFeature: storeReactivityBindings(),
        },
    });

    // mirror the state into a signal on every change of the state or the options
    const [tracked, setTracked] = createSignal<{ readonly state: TableState<Features> }>(
        { state: table.store.state },
        { ownedWrite: true },
    );
    const publish = (): void => {
        setTracked({ state: table.store.state });
    };
    const subscriptions = [table.store.subscribe(publish), table.optionsStore?.subscribe(publish)];
    onCleanup(() => {
        for (const subscription of subscriptions) {
            subscription?.unsubscribe();
        }
    });

    // drop the selected cells as a sort, filter or grouping reorders the rows, and the page as rows leave
    const view: object = table;
    const arranging = (): readonly unknown[] => {
        const state: Partial<
            TableState_RowSorting &
                TableState_ColumnFiltering &
                TableState_GlobalFiltering &
                TableState_ColumnGrouping
        > = tracked().state;

        return [state.sorting, state.columnFilters, state.globalFilter, state.grouping];
    };
    createEffect(arranging, (next, previous) => {
        // leave the table as it starts
        if (previous === undefined) {
            return;
        }

        // drop the selected cells for any new order, and go to the first page for new rows
        const [sorting, ...filtering] = next;
        const isFiltered = filtering.some((slice, index) => slice !== previous[index + 1]);
        untrack(() => {
            if (isSelectingTable(view) && (sorting !== previous[0] || isFiltered)) {
                view.resetCellSelection(true);
            }
            if (isPagingTable(view) && isFiltered) {
                view.setPageIndex(0);
            }
        });
    });
    createEffect(
        () => {
            tracked();

            return isPagingTable(view)
                ? [view.store.state.pagination.pageIndex, view.getPageCount()]
                : [];
        },
        ([index, count]) => {
            // move back to the last page where rows left the page shown
            if (
                isPagingTable(view) &&
                index !== undefined &&
                count !== undefined &&
                index >= count
            ) {
                untrack(() => view.setPageIndex(Math.max(count - 1, 0)));
            }
        },
    );

    // hand the table each change of the reactive options
    createEffect(
        () => ({ ...options, state: { ...options.state } }),
        (next) =>
            untrack(() =>
                table.setOptions((previous) => ({
                    ...previous,
                    ...next,
                    features: previous.features,
                })),
            ),
    );

    return Object.assign(table, { tracked: () => tracked().state });
}

/** Report whether a table selects blocks of cells. */
function isSelectingTable(
    table: object,
): table is Pick<Table_CellSelection<TableFeatures, RowData>, "resetCellSelection"> {
    return "resetCellSelection" in table && typeof table.resetCellSelection === "function";
}

/** Report whether a table pages its rows. */
function isPagingTable(table: object): table is Pick<
    Table_RowPagination<TableFeatures, RowData>,
    "setPageIndex" | "getPageCount"
> & {
    readonly store: { readonly state: { readonly pagination: { readonly pageIndex: number } } };
} {
    return "setPageIndex" in table && typeof table.setPageIndex === "function";
}
