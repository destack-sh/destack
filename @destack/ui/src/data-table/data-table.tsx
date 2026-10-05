import { Icon } from "@destack/icon";
import caretDown from "@destack/icon/phosphor/caret-down";
import caretUp from "@destack/icon/phosphor/caret-up";
import caretUpDown from "@destack/icon/phosphor/caret-up-down";
import { plural, t } from "@destack/locale";
import * as style from "@destack/style";
import { color, space } from "@destack/theme/tokens.stylex";
import { useLocale } from "@destack/locale/solid";
import { text } from "@destack/theme/text";
import type { JSX } from "@solidjs/web";
import { createMemo, createSignal, For, Show, type Accessor } from "solid-js";
import { Button } from "../button/index.ts";
import { Checkbox } from "../checkbox/index.ts";
import { Input } from "../input/index.ts";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "../table/index.ts";

/** The rows a page shows without a page size, shadcn/ui's data table default. */
const PAGE_SIZE = 10;

/** The order a sorted column cycles through on each click, after TanStack Table's default. */
const NEXT_ORDER: Readonly<Record<DataTableOrder, DataTableOrder>> = {
    none: "ascending",
    ascending: "descending",
    descending: "none",
};

/** The styles of a data table and its toolbar and footer. */
const styles = style.create({
    table: {
        display: "flex",
        flexDirection: "column",
        gap: space[4],
        width: "100%",
    },
    filter: {
        maxWidth: `calc(24 * ${space[4]})`,
    },
    sort: {
        marginInlineStart: `calc(-1 * ${space[3]})`,
    },
    selection: {
        width: space[6],
    },
    empty: {
        height: `calc(6 * ${space[4]})`,
        textAlign: "center",
    },
    footer: {
        display: "flex",
        alignItems: "center",
        justifyContent: "flex-end",
        gap: space[2],
        color: color.mutedForeground,
    },
    count: {
        flex: 1,
    },
});

/** The order of a sorted column, as `aria-sort` names it. */
export type DataTableOrder = "none" | "ascending" | "descending";

/** A column of a data table: its header, its cells and how it sorts and filters. */
export interface DataTableColumn<Row> {
    /** The id of the column. */
    readonly id: string;
    /** The text of the column's header. */
    readonly header: string;
    /** Render the column's cell of a row. */
    readonly cell: (row: Row) => JSX.Element;
    /** Read the value the column sorts a row by, which makes the column sortable. */
    readonly sortValue?: (row: Row) => string | number;
    /** Read the text the filter matches in a row's cell, which makes the column searchable. */
    readonly filterValue?: (row: Row) => string;
}

/** The sorted column of a data table and its order. */
export interface DataTableSort {
    /** The id of the sorted column. */
    readonly column: string;
    /** The order of the column. */
    readonly order: DataTableOrder;
}

/** The properties of a data table. */
export interface DataTableProperties<Row> {
    /** The rows, in their own order before sorting. */
    readonly rows: readonly Row[];
    /** The columns, in display order. */
    readonly columns: readonly DataTableColumn<Row>[];
    /** Read the stable id of a row, which selection follows across sorting and pages. */
    readonly rowId: (row: Row) => string;
    /** Whether rows have checkboxes that select them. */
    readonly isSelectable?: boolean;
    /** The rows a page shows, 10 by default. */
    readonly pageSize?: number;
    /** The placeholder and name of the filter input, a generic one by default. */
    readonly filterPlaceholder?: string;
    /** Handle the selected rows changing, with their ids. */
    readonly onSelectionChange?: (ids: readonly string[]) => void;
}

/** The sorting, filter, page and selection of a data table over its rows. */
export class DataTableControl<Row> {
    /** The properties of the table, read for its rows and columns. */
    readonly #properties: DataTableProperties<Row>;
    /** The sorted column and its order. */
    readonly sorting: Accessor<DataTableSort>;
    /** The text the filter matches. */
    readonly filter: Accessor<string>;
    /** The index of the page shown, from 0. */
    readonly page: Accessor<number>;
    /** The ids of the selected rows. */
    readonly selected: Accessor<ReadonlySet<string>>;
    /** The rows that match the filter, in sorted order. */
    readonly shown: Accessor<readonly Row[]>;
    /** Replace the sort. */
    readonly #setSort: (sort: DataTableSort) => void;
    /** Replace the filter. */
    readonly #setFilter: (filter: string) => void;
    /** Replace the page. */
    readonly #setPage: (page: number) => void;
    /** Replace the selection. */
    readonly #setSelected: (selected: ReadonlySet<string>) => void;

    /** Create a table over its rows, unsorted, unfiltered and on its first page, comparing text in a locale. */
    constructor(properties: DataTableProperties<Row>, locale: Accessor<string>) {
        // start unsorted, unfiltered, unselected and on the first page
        const [sort, setSort] = createSignal<DataTableSort>({ column: "", order: "none" });
        const [filter, setFilter] = createSignal("");
        const [page, setPage] = createSignal(0);
        const [selected, setSelected] = createSignal<ReadonlySet<string>>(new Set());
        this.#properties = properties;
        this.sorting = sort;
        this.filter = filter;
        this.page = page;
        this.selected = selected;
        this.shown = createMemo(() =>
            sorted(matching(properties, filter()), properties.columns, sort(), locale()),
        );
        this.#setSort = setSort;
        this.#setFilter = setFilter;
        this.#setPage = setPage;
        this.#setSelected = setSelected;
    }

    /** Read the rows a page shows. */
    pageSize(): number {
        return this.#properties.pageSize ?? PAGE_SIZE;
    }

    /** Count the pages of the shown rows, at least one. */
    pageCount(): number {
        return Math.max(1, Math.ceil(this.shown().length / this.pageSize()));
    }

    /** List the rows of the page shown. */
    pageRows(): readonly Row[] {
        const start = Math.min(this.page(), this.pageCount() - 1) * this.pageSize();

        return this.shown().slice(start, start + this.pageSize());
    }

    /** Move a column to its next order, other columns returning to none. */
    toggleSort(column: string): void {
        const current = this.sorting();
        const order = current.column === column ? current.order : "none";
        this.#setSort({ column, order: NEXT_ORDER[order] });
    }

    /** Filter the rows by text, returning to the first page. */
    search(filter: string): void {
        this.#setFilter(filter);
        this.#setPage(0);
    }

    /** Show a page, within the pages there are. */
    go(page: number): void {
        this.#setPage(Math.min(Math.max(page, 0), this.pageCount() - 1));
    }

    /** Select or deselect rows by id and tell the change handler. */
    select(ids: readonly string[], isSelected: boolean): void {
        // add or remove each id and report the selection
        const next = new Set(this.selected());
        for (const id of ids) {
            if (isSelected) {
                next.add(id);
            } else {
                next.delete(id);
            }
        }
        this.#setSelected(next);
        this.#properties.onSelectionChange?.([...next]);
    }

    /** Read the ids of the page's rows. */
    pageIds(): string[] {
        return this.pageRows().map((row) => this.#properties.rowId(row));
    }
}

/** Render rows as a table that sorts by its headers, filters by text, selects rows and pages through them. */
export function DataTable<Row>(properties: DataTableProperties<Row>): JSX.Element {
    // share one table state with the header, rows and footer
    const locale = useLocale();
    const control = new DataTableControl(properties, () => locale.tag);
    const isSelectable = (): boolean => properties.isSelectable === true;
    const width = (): number => properties.columns.length + (isSelectable() ? 1 : 0);

    return (
        <div data-slot="data-table" {...style.attrs(styles.table)}>
            <Input
                type="search"
                placeholder={properties.filterPlaceholder ?? locale.render(t`Filter`)}
                aria-label={properties.filterPlaceholder ?? locale.render(t`Filter`)}
                value={control.filter()}
                onInput={(event) => control.search(event.currentTarget.value)}
                style={styles.filter}
            />
            <Table>
                <TableHeader>
                    <DataTableHeader
                        control={control}
                        columns={properties.columns}
                        isSelectable={isSelectable()}
                    />
                </TableHeader>
                <TableBody>
                    <For each={control.pageRows()} fallback={<DataTableEmpty width={width()} />}>
                        {(row) => (
                            <DataTableRow control={control} row={row} properties={properties} />
                        )}
                    </For>
                </TableBody>
            </Table>
            <DataTableFooter control={control} isSelectable={isSelectable()} />
        </div>
    );
}

/** Render the header row: the page's select-all box and a sort button per sortable column. */
function DataTableHeader<Row>(properties: {
    readonly control: DataTableControl<Row>;
    readonly columns: readonly DataTableColumn<Row>[];
    readonly isSelectable: boolean;
}): JSX.Element {
    // count the page's selected rows
    const locale = useLocale();
    const control = properties.control;
    const pageSelected = (): number =>
        control.pageIds().filter((id) => control.selected().has(id)).length;

    return (
        <TableRow>
            <Show when={properties.isSelectable}>
                <TableHead style={styles.selection}>
                    <Checkbox
                        aria-label={locale.render(t`Select all`)}
                        checked={pageSelected() > 0 && pageSelected() === control.pageIds().length}
                        indeterminate={
                            pageSelected() > 0 && pageSelected() < control.pageIds().length
                        }
                        onChange={(event) =>
                            control.select(control.pageIds(), event.currentTarget.checked)
                        }
                    />
                </TableHead>
            </Show>
            <For each={properties.columns}>
                {(column) => <DataTableColumnHead control={control} column={column} />}
            </For>
        </TableRow>
    );
}

/** Render a column's header cell, a button that cycles the sort when the column sorts. */
function DataTableColumnHead<Row>(properties: {
    readonly control: DataTableControl<Row>;
    readonly column: DataTableColumn<Row>;
}): JSX.Element {
    // read the column's order in the table's sort
    const control = properties.control;
    const column = properties.column;
    const order = (): DataTableOrder =>
        control.sorting().column === column.id ? control.sorting().order : "none";

    return (
        <TableHead
            scope="col"
            aria-sort={column.sortValue === undefined || order() === "none" ? undefined : order()}
        >
            <Show when={column.sortValue !== undefined} fallback={column.header}>
                <Button
                    variant="ghost"
                    size="sm"
                    style={styles.sort}
                    onClick={() => control.toggleSort(column.id)}
                >
                    {column.header}
                    <Icon
                        icon={
                            order() === "ascending"
                                ? caretUp
                                : order() === "descending"
                                  ? caretDown
                                  : caretUpDown
                        }
                    />
                </Button>
            </Show>
        </TableHead>
    );
}

/** Render one row with its selection box and cells. */
function DataTableRow<Row>(properties: {
    readonly control: DataTableControl<Row>;
    readonly row: Row;
    readonly properties: DataTableProperties<Row>;
}): JSX.Element {
    // read the row's id and selection
    const locale = useLocale();
    const id = properties.properties.rowId(properties.row);
    const isSelected = (): boolean => properties.control.selected().has(id);

    return (
        <TableRow data-state={isSelected() ? "selected" : undefined}>
            <Show when={properties.properties.isSelectable === true}>
                <TableCell>
                    <Checkbox
                        aria-label={locale.render(t`Select row`)}
                        checked={isSelected()}
                        onChange={(event) =>
                            properties.control.select([id], event.currentTarget.checked)
                        }
                    />
                </TableCell>
            </Show>
            <For each={properties.properties.columns}>
                {(column) => <TableCell>{column.cell(properties.row)}</TableCell>}
            </For>
        </TableRow>
    );
}

/** Render the row shown while no row matches the filter. */
function DataTableEmpty(properties: { readonly width: number }): JSX.Element {
    const locale = useLocale();

    return (
        <TableRow>
            <TableCell colspan={properties.width} style={styles.empty}>
                {locale.render(t`No results`)}
            </TableCell>
        </TableRow>
    );
}

/** Render the count of selected rows, the page number and the buttons to the previous and next page. */
function DataTableFooter<Row>(properties: {
    readonly control: DataTableControl<Row>;
    readonly isSelectable: boolean;
}): JSX.Element {
    const locale = useLocale();
    const control = properties.control;

    return (
        <div data-slot="data-table-footer" {...style.attrs(text.footnote, styles.footer)}>
            <span aria-live="polite" {...style.attrs(styles.count)}>
                <Show when={properties.isSelectable}>
                    {locale.render(
                        t`${control.selected().size} of ${plural(control.shown().length, { one: "# row", other: "# rows" })} selected`,
                    )}
                </Show>
            </span>
            <span>{locale.render(t`Page ${control.page() + 1} of ${control.pageCount()}`)}</span>
            <Button
                variant="outline"
                size="sm"
                disabled={control.page() === 0}
                onClick={() => control.go(control.page() - 1)}
            >
                {locale.render(t`Previous`)}
            </Button>
            <Button
                variant="outline"
                size="sm"
                disabled={control.page() >= control.pageCount() - 1}
                onClick={() => control.go(control.page() + 1)}
            >
                {locale.render(t`Next`)}
            </Button>
        </div>
    );
}

/** List the rows whose searchable cells contain the filter text, ignoring case. */
function matching<Row>(properties: DataTableProperties<Row>, filter: string): readonly Row[] {
    const needle = filter.trim().toLowerCase();
    if (needle === "") {
        return properties.rows;
    }

    return properties.rows.filter((row) =>
        properties.columns.some(
            (column) => column.filterValue?.(row).toLowerCase().includes(needle) === true,
        ),
    );
}

/** Sort rows by a column's values, comparing text in a locale with numbers in their numeric order. */
function sorted<Row>(
    rows: readonly Row[],
    columns: readonly DataTableColumn<Row>[],
    sort: DataTableSort,
    locale: string,
): readonly Row[] {
    // keep the rows' own order without a sorted column
    const read = columns.find((column) => column.id === sort.column)?.sortValue;
    if (read === undefined || sort.order === "none") {
        return rows;
    }

    // compare the values, reversing for descending order
    const collator = new Intl.Collator(locale, { numeric: true });
    const sign = sort.order === "ascending" ? 1 : -1;

    return rows.toSorted((left, right) => {
        const first = read(left);
        const second = read(right);

        return (
            sign *
            (typeof first === "number" && typeof second === "number"
                ? first - second
                : collator.compare(String(first), String(second)))
        );
    });
}
