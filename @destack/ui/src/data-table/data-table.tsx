import { Icon, type IconBodies } from "@destack/icon";
import caretDoubleLeft from "@destack/icon/phosphor/caret-double-left";
import caretDoubleRight from "@destack/icon/phosphor/caret-double-right";
import caretDown from "@destack/icon/phosphor/caret-down";
import caretLeft from "@destack/icon/phosphor/caret-left";
import caretRight from "@destack/icon/phosphor/caret-right";
import caretUp from "@destack/icon/phosphor/caret-up";
import caretUpDown from "@destack/icon/phosphor/caret-up-down";
import slidersHorizontal from "@destack/icon/phosphor/sliders-horizontal";
import { type Direction, plural, t } from "@destack/locale";
import * as style from "@destack/style";
import { color, space } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createContext,
    createEffect,
    createMemo,
    createSignal,
    For,
    type JSX,
    omit,
    Show,
    untrack,
    useContext,
    useLocale,
} from "@destack/view";
import type {
    Cell,
    CellContext,
    ColumnDefTemplate,
    Header,
    HeaderContext,
    PaginationState,
    RowData,
    TableFeatures,
} from "@tanstack/table-core";
import { Button } from "../button/index.ts";
import { Checkbox } from "../checkbox/index.ts";
import { Focus, type GridMove, gridMoveOf, type KeyboardDelegate } from "../focus/index.ts";
import {
    DropdownMenu,
    DropdownMenuCheckboxItem,
    DropdownMenuContent,
    DropdownMenuLabel,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
} from "../dropdown-menu/index.ts";
import { Input } from "../input/index.ts";
import { Select, SelectItem } from "../select/index.ts";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "../table/index.ts";
import type { DataTableInstance } from "./table.ts";
import { createVirtualizer } from "../virtualizer/index.ts";

/** The rows a page jump moves by with Page Up and Page Down. */
const PAGE_ROWS = 10;

/** The page sizes a pagination offers without its own. */
const PAGE_SIZES: readonly number[] = [10, 20, 50, 100];

/** The elements inside a cell that take the focus in its place. */
const FOCUSABLE = "a[href], button, input, select, textarea, [tabindex]:not(td, th)";

/** The state of the nearest data table, read to track it, null outside one. */
const DataTableContext = createContext<Accessor<unknown> | null>(null);

/** The focused cell of the nearest data table, null outside one. */
const DataTableFocusContext = createContext<Focus<string> | null>(null);

/** The styles of a data table and its toolbar and footer. */
const styles = style.create({
    scroller: {
        overflowY: "auto",
        maxHeight: "100%",
    },
    spacer: (height: string) => ({ height, padding: 0 }),
    empty: {
        height: `calc(6 * ${space[4]})`,
        textAlign: "center",
        color: color.mutedForeground,
    },
    sort: {
        marginInlineStart: `calc(-1 * ${space[3]})`,
    },
    selection: {
        width: space[6],
    },
    toggle: (depth: string) => ({
        display: "inline-flex",
        alignItems: "center",
        gap: space[1],
        paddingInlineStart: `calc(${depth} * ${space[4]})`,
    }),
    members: {
        color: color.mutedForeground,
    },
    filter: {
        maxWidth: `calc(24 * ${space[4]})`,
    },
    pagination: {
        display: "flex",
        flexWrap: "wrap",
        alignItems: "center",
        justifyContent: "flex-end",
        gap: space[4],
        color: color.mutedForeground,
    },
    count: {
        flex: 1,
    },
    size: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
    },
    steps: {
        display: "flex",
        alignItems: "center",
        gap: space[1],
    },
});

/** A column a header sorts by, as the sorting feature gives it. */
export interface SortableColumn {
    /** Report whether the column sorts. */
    getCanSort(): boolean;
    /** Report the column's sort direction, false while unsorted. */
    getIsSorted(): false | "asc" | "desc";
    /** Sort by the column, descending or ascending, beside the other sorted columns when multiple. */
    toggleSorting(isDescending?: boolean, isMultiple?: boolean): void;
    /** Report the direction the next toggle sorts in, false when it clears the sort. */
    getNextSortingOrder(isMultiple?: boolean): false | "asc" | "desc";
    /** Stop sorting by the column. */
    clearSorting(): void;
}

/** A column a person shows and hides, as the visibility feature gives it. */
export interface HideableColumn {
    /** The column's id. */
    readonly id: string;
    /** Report whether the column hides. */
    getCanHide(): boolean;
    /** Report whether the column shows. */
    getIsVisible(): boolean;
    /** Show or hide the column. */
    toggleVisibility(isVisible?: boolean): void;
}

/** A table a pagination pages through, as the pagination feature gives it. */
export interface PaginatedTable {
    /** The table's state, read to track it. */
    readonly tracked: Accessor<unknown>;
    /** The current page and its size. */
    readonly store: { readonly state: { readonly pagination: PaginationState } };
    /** Report the number of pages. */
    getPageCount(): number;
    /** Report the number of rows across every page. */
    getRowCount(): number;
    /** Report whether a page comes before the current one. */
    getCanPreviousPage(): boolean;
    /** Report whether a page comes after the current one. */
    getCanNextPage(): boolean;
    /** Go to the first page. */
    firstPage(): void;
    /** Go to the previous page. */
    previousPage(): void;
    /** Go to the next page. */
    nextPage(): void;
    /** Go to the last page. */
    lastPage(): void;
    /** Show a number of rows per page. */
    setPageSize(size: number): void;
    /** List the selected rows, present when rows select. */
    getSelectedRowModel?(): { readonly rows: readonly unknown[] };
}

/** A table a person filters across every column, as the global filtering feature gives it. */
export interface FilteredTable {
    /** The table's state, read to track it. */
    readonly tracked: Accessor<unknown>;
    /** The current filter text. */
    readonly store: { readonly state: { readonly globalFilter?: unknown } };
    /** Filter the rows by a text. */
    setGlobalFilter(filter: string): void;
}

/** A table whose columns a person shows and hides. */
export interface ColumnTable {
    /** The table's state, read to track it. */
    readonly tracked: Accessor<unknown>;
    /** List every leaf column. */
    getAllLeafColumns(): readonly object[];
}

/** A table whose rows a person selects, as the selection feature gives it. */
export interface SelectableTable {
    /** Report whether every row of the page is selected. */
    getIsAllPageRowsSelected(): boolean;
    /** Report whether some rows of the page are selected. */
    getIsSomePageRowsSelected(): boolean;
    /** Select or deselect every row of the page. */
    toggleAllPageRowsSelected(isSelected?: boolean): void;
}

/** A row a person selects, as the selection feature gives it. */
export interface SelectableRow {
    /** Report whether the row selects. */
    getCanSelect(): boolean;
    /** Report whether the row is selected. */
    getIsSelected(): boolean;
    /** Select or deselect the row. */
    toggleSelected(isSelected?: boolean): void;
}

/** A row a person expands to show its sub rows, as the expanding feature gives it. */
export interface ExpandableRow {
    /** The nesting depth, 0 for a top-level row. */
    readonly depth: number;
    /** The rows inside it, such as a group's members. */
    readonly subRows: readonly unknown[];
    /** Report whether the row expands. */
    getCanExpand(): boolean;
    /** Report whether the row shows its sub rows. */
    getIsExpanded(): boolean;
    /** Show or hide the sub rows. */
    toggleExpanded(isExpanded?: boolean): void;
}

/** A cell of a grouped table, as the grouping and aggregation features give it. */
export interface GroupedCell {
    /** Report whether the cell heads its row's group. */
    getIsGrouped(): boolean;
    /** Report whether the cell repeats its group's value and shows nothing. */
    getIsPlaceholder(): boolean;
    /** Report whether the cell aggregates its group's members, present with the aggregation feature. */
    getIsAggregated?(): boolean;
}

/** The properties of a data table: the table it renders, its empty state and its scrolling, the native element's attributes included. */
export interface DataTableProperties<
    Features extends TableFeatures,
    Row extends RowData,
> extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "onKeyDown"> {
    /** The headless table, from `createTable`. */
    readonly table: DataTableInstance<Features, Row>;
    /** The content shown while no row matches, a message by default. */
    readonly empty?: JSX.Element;
    /** The height each row is estimated at, which renders only the rows in view of a scrolling table. */
    readonly rowHeight?: number | undefined;
    /** The StyleX styles applied after the table's styles. */
    readonly xstyle?: style.Styles;
}

/** Render a table's headers and rows as a grid that arrow keys, Home, End, Page Up and Page Down move through. */
export function DataTable<Features extends TableFeatures, Row extends RowData>(
    properties: DataTableProperties<Features, Row>,
): JSX.Element {
    // read the header groups and rows as the table changes
    const locale = useLocale();
    const rest = omit(properties, "table", "empty", "rowHeight", "xstyle", "style");
    const headerGroups = createMemo(() => {
        properties.table.tracked();

        return properties.table.getHeaderGroups();
    });
    const rows = createMemo(() => {
        properties.table.tracked();

        return properties.table.getRowModel().rows;
    });
    const width = (): number => headerGroups().at(-1)?.headers.length ?? 1;

    // render only the rows in view when the rows have a height
    const [scroller, setScroller] = createSignal<HTMLElement | undefined>(undefined, {
        ownedWrite: true,
    });
    const virtualizer = createVirtualizer({
        count: () => rows().length,
        itemHeight: properties.rowHeight ?? 0,
        scrollElement: scroller,
    });
    const shown = createMemo(() =>
        properties.rowHeight === undefined
            ? rows().map((row, index) => ({ row, index, start: 0, end: 0 }))
            : virtualizer.items().flatMap((item) => {
                  const row = rows()[item.index];

                  return row === undefined
                      ? []
                      : [{ row, index: item.index, start: item.start, end: item.end }];
              }),
    );
    const before = (): number => shown()[0]?.start ?? 0;
    const after = (): number => virtualizer.height() - (shown().at(-1)?.end ?? 0);

    // move one focused cell through the headers and rows, bringing a row out of view into it
    const headerWidths = createMemo(() => headerGroups().map((group) => group.headers.length));
    const rowWidths = createMemo(() => rows().map((row) => row.getAllCells().length));
    const focus = new Focus<string>({
        delegate: new TableDelegate(headerWidths, rowWidths),
        mode: "roving",
        reveal: (key) => {
            const body = cellOf(key).row - headerGroups().length;
            if (properties.rowHeight !== undefined && body >= 0) {
                virtualizer.reveal(body);
            }
        },
    });

    return (
        <div
            data-slot="data-table"
            {...rest}
            ref={(element) => setScroller(properties.rowHeight === undefined ? undefined : element)}
            onKeyDown={(event) => navigate(event, focus, locale.direction)}
            onFocusOut={(event) => focus.focusOut(event)}
            {...style.attributes(
                [properties.rowHeight !== undefined && styles.scroller, properties.xstyle],
                properties.style,
            )}
        >
            <DataTableContext value={properties.table.tracked}>
                <DataTableFocusContext value={focus}>
                    <Table
                        role={follow(properties.table.tracked, () =>
                            rows().some(isExpandableRow) ? "treegrid" : "grid",
                        )}
                        aria-rowcount={rows().length + headerGroups().length}
                    >
                        <TableHeader>
                            <For each={headerGroups()}>
                                {(group, row) => (
                                    <TableRow>
                                        <For each={group.headers}>
                                            {(header, column) => (
                                                <DataTableHeaderCell
                                                    header={header}
                                                    cell={cellKey(row(), column())}
                                                />
                                            )}
                                        </For>
                                    </TableRow>
                                )}
                            </For>
                        </TableHeader>
                        <TableBody>
                            <SpacerRow height={before()} width={width()} />
                            <For
                                each={shown()}
                                keyed={(entry) => entry.row}
                                fallback={
                                    <TableRow>
                                        <TableCell colspan={width()} xstyle={styles.empty}>
                                            {properties.empty ?? locale.render(t`No results.`)}
                                        </TableCell>
                                    </TableRow>
                                }
                            >
                                {(entry) => (
                                    <DataTableRow
                                        row={entry().row}
                                        place={headerGroups().length + entry().index}
                                    />
                                )}
                            </For>
                            <SpacerRow height={after()} width={width()} />
                        </TableBody>
                    </Table>
                </DataTableFocusContext>
            </DataTableContext>
        </div>
    );
}

/** Render a header cell of a data table, marked with its column's sort. */
function DataTableHeaderCell<Features extends TableFeatures, Data extends RowData>(properties: {
    /** The header the cell shows. */
    readonly header: Header<Features, Data>;
    /** The key of the cell's place in the grid. */
    readonly cell: string;
}): JSX.Element {
    const tracked = useDataTableState();
    const header = properties.header;

    return (
        <TableHead
            colspan={header.colSpan}
            aria-sort={follow(tracked, () => sortOf(header))}
            data-column={header.column.id}
            {...useGridCell(() => properties.cell)}
        >
            <Show when={!header.isPlaceholder}>
                {renderTemplate(header.column.columnDef.header, header.getContext())}
            </Show>
        </TableHead>
    );
}

/** Render a row of a data table, marked with its selection, level and expansion. */
function DataTableRow<Features extends TableFeatures, Data extends RowData>(properties: {
    /** The row the table row shows, with its cells. */
    readonly row: { getAllCells(): Cell<Features, Data>[] };
    /** The row's place among every row of the grid, the headers' included. */
    readonly place: number;
}): JSX.Element {
    const tracked = useDataTableState();
    const row = untrack(() => properties.row);

    return (
        <TableRow
            data-state={follow(tracked, () => (isSelectedRow(row) ? "selected" : undefined))}
            aria-selected={follow(tracked, () => selectionOf(row))}
            aria-level={levelOf(row)}
            aria-expanded={follow(tracked, () => expansionOf(row))}
        >
            <For each={row.getAllCells()}>
                {(cell, column) => (
                    <TableCell
                        data-column={cell.column.id}
                        {...useGridCell(() => cellKey(properties.place, column()))}
                    >
                        {renderCell(cell)}
                    </TableCell>
                )}
            </For>
        </TableRow>
    );
}

/** Read a value of a table's state, following the table as it changes. */
function follow<Value>(tracked: Accessor<unknown>, value: () => Value): Value {
    tracked();

    return value();
}

/** Render a row holding the place of the rows a virtual table leaves out, none at no height. */
function SpacerRow(properties: {
    /** The height of the rows left out, in pixels. */
    readonly height: number;
    /** The number of columns the row spans. */
    readonly width: number;
}): JSX.Element {
    return (
        <Show when={properties.height > 0}>
            <tr aria-hidden="true">
                <td
                    colspan={properties.width}
                    {...style.attrs(styles.spacer(`${properties.height}px`))}
                />
            </tr>
        </Show>
    );
}

/** Read the nearest data table's state in a header or cell template, refusing a template outside one. */
export function useDataTableState(): Accessor<unknown> {
    const tracked = useContext(DataTableContext);
    if (tracked === null) {
        throw new TypeError("a data table template needs a data table around it");
    }

    return tracked;
}

/** The properties of a sortable column's header. */
export interface DataTableColumnHeaderProperties {
    /** The column the header sorts. */
    readonly column: object;
    /** The column's name. */
    readonly title: string;
}

/** Render a column's name as a button that cycles its sort ascending, descending and off, Shift adding it to the other sorted columns. */
export function DataTableColumnHeader(properties: DataTableColumnHeaderProperties): JSX.Element {
    // show the name alone for a column that does not sort
    const tracked = useDataTableState();
    const column = properties.column;
    if (!isSortable(column) || !column.getCanSort()) {
        return <span data-slot="data-table-column-header">{properties.title}</span>;
    }

    return (
        <Button
            variant="ghost"
            size="sm"
            data-slot="data-table-column-header"
            xstyle={styles.sort}
            onClick={(event) => {
                // toggle the sort beside the other sorted columns on Shift
                const next = column.getNextSortingOrder(event.shiftKey);
                if (next === false) {
                    column.clearSorting();
                } else {
                    column.toggleSorting(next === "desc", event.shiftKey);
                }
            }}
        >
            {properties.title}
            <Icon
                icon={sortIconOf(
                    (() => {
                        tracked();

                        return column.getIsSorted();
                    })(),
                )}
            />
        </Button>
    );
}

/** The properties of a data table's pagination. */
export interface DataTablePaginationProperties {
    /** The table the pagination pages through. */
    readonly table: PaginatedTable;
    /** The page sizes offered, ten to a hundred by default. */
    readonly pageSizes?: readonly number[];
}

/** Render the selected row count, the page size, the page number and the steps between pages. */
export function DataTablePagination(properties: DataTablePaginationProperties): JSX.Element {
    // read the page and the selection as the table changes
    const locale = useLocale();
    const table = properties.table;
    const page = (): PaginationState => follow(table.tracked, () => table.store.state.pagination);
    const count = (): number => follow(table.tracked, () => table.getPageCount());
    const selected = (): number | undefined =>
        follow(table.tracked, () => table.getSelectedRowModel?.().rows.length);
    const total = (): number => follow(table.tracked, () => table.getRowCount());
    const canPrevious = (): boolean => follow(table.tracked, () => table.getCanPreviousPage());
    const canNext = (): boolean => follow(table.tracked, () => table.getCanNextPage());

    return (
        <div data-slot="data-table-pagination" {...style.attrs(text.footnote, styles.pagination)}>
            <span {...style.attrs(styles.count)}>
                <Show when={selected()}>
                    {(rows) => (
                        <>
                            {locale.render(
                                t`${rows()} of ${plural(total(), { one: "# row", other: "# rows" })} selected`,
                            )}
                        </>
                    )}
                </Show>
            </span>
            <label {...style.attrs(styles.size)}>
                {locale.render(t`Rows per page`)}
                <Select
                    value={String(page().pageSize)}
                    onValueChange={(size) => table.setPageSize(Number(size))}
                >
                    <For each={properties.pageSizes ?? PAGE_SIZES}>
                        {(size) => <SelectItem value={String(size)}>{size}</SelectItem>}
                    </For>
                </Select>
            </label>
            <span>{locale.render(t`Page ${page().pageIndex + 1} of ${Math.max(count(), 1)}`)}</span>
            <div {...style.attrs(styles.steps)}>
                <PageStep
                    label={locale.render(t`Go to the first page`)}
                    icon={caretDoubleLeft}
                    disabled={!canPrevious()}
                    go={() => table.firstPage()}
                />
                <PageStep
                    label={locale.render(t`Go to the previous page`)}
                    icon={caretLeft}
                    disabled={!canPrevious()}
                    go={() => table.previousPage()}
                />
                <PageStep
                    label={locale.render(t`Go to the next page`)}
                    icon={caretRight}
                    disabled={!canNext()}
                    go={() => table.nextPage()}
                />
                <PageStep
                    label={locale.render(t`Go to the last page`)}
                    icon={caretDoubleRight}
                    disabled={!canNext()}
                    go={() => table.lastPage()}
                />
            </div>
        </div>
    );
}

/** Render a button that steps to another page. */
function PageStep(properties: {
    /** The button's accessible name. */
    readonly label: string;
    /** The arrow it shows. */
    readonly icon: IconBodies;
    /** Whether no page lies that way. */
    readonly disabled: boolean;
    /** Go to the page. */
    readonly go: () => void;
}): JSX.Element {
    return (
        <Button
            variant="outline"
            size="icon-sm"
            aria-label={properties.label}
            disabled={properties.disabled}
            onClick={() => properties.go()}
        >
            <Icon icon={properties.icon} />
        </Button>
    );
}

/** The properties of a data table's filter input. */
export interface DataTableFilterProperties {
    /** The table the input filters. */
    readonly table: FilteredTable;
    /** The input's prompt and accessible name. */
    readonly placeholder: string;
}

/** Render an input that filters a table's rows across every column. */
export function DataTableFilter(properties: DataTableFilterProperties): JSX.Element {
    const filter = (): string => {
        properties.table.tracked();
        const value = properties.table.store.state.globalFilter;

        return typeof value === "string" ? value : "";
    };

    return (
        <Input
            type="search"
            data-slot="data-table-filter"
            aria-label={properties.placeholder}
            placeholder={properties.placeholder}
            value={filter()}
            onInput={(event) => properties.table.setGlobalFilter(event.currentTarget.value)}
            xstyle={styles.filter}
        />
    );
}

/** The properties of a data table's column menu. */
export interface DataTableViewOptionsProperties {
    /** The table whose columns the menu shows and hides. */
    readonly table: ColumnTable;
    /** Label a column by its id, the id by default. */
    readonly label?: (id: string) => string;
}

/** Render a menu that shows and hides a table's hideable columns. */
export function DataTableViewOptions(properties: DataTableViewOptionsProperties): JSX.Element {
    const locale = useLocale();
    const columns = createMemo(() => {
        properties.table.tracked();

        return properties.table.getAllLeafColumns().filter(isHideable);
    });

    return (
        <DropdownMenu>
            <DropdownMenuTrigger variant="outline" size="sm" data-slot="data-table-view-options">
                <Icon icon={slidersHorizontal} />
                {locale.render(t`View`)}
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
                <DropdownMenuLabel>{locale.render(t`Toggle columns`)}</DropdownMenuLabel>
                <DropdownMenuSeparator />
                <For each={columns().filter((column) => column.getCanHide())}>
                    {(column) => (
                        <DropdownMenuCheckboxItem
                            checked={column.getIsVisible()}
                            onCheckedChange={(isVisible) => column.toggleVisibility(isVisible)}
                            onSelect={(event) => event.preventDefault()}
                        >
                            {properties.label?.(column.id) ?? column.id}
                        </DropdownMenuCheckboxItem>
                    )}
                </For>
            </DropdownMenuContent>
        </DropdownMenu>
    );
}

/** The column that selects rows: a box per row and a box in the header for the whole page. */
export function selectionColumn<Features extends TableFeatures, Row extends RowData>(): {
    /** The column's id. */
    readonly id: "select";
    /** The box that selects the page. */
    readonly header: (context: HeaderContext<Features, Row>) => JSX.Element;
    /** The box that selects a row. */
    readonly cell: (context: CellContext<Features, Row>) => JSX.Element;
    /** The column neither sorts nor hides. */
    readonly enableSorting: false;
    /** The column neither sorts nor hides. */
    readonly enableHiding: false;
} {
    return {
        id: "select",
        header: (context) => <PageSelection table={context.table} />,
        cell: (context) => <RowSelection row={context.row} />,
        enableSorting: false,
        enableHiding: false,
    };
}

/** The column that expands rows: a button per row with sub rows, indented by the row's depth. */
export function expansionColumn<Features extends TableFeatures, Row extends RowData>(): {
    /** The column's id. */
    readonly id: "expand";
    /** The column's header, empty. */
    readonly header: () => JSX.Element;
    /** The button that expands a row. */
    readonly cell: (context: CellContext<Features, Row>) => JSX.Element;
    /** The column neither sorts nor hides. */
    readonly enableSorting: false;
    /** The column neither sorts nor hides. */
    readonly enableHiding: false;
} {
    return {
        id: "expand",
        header: () => null,
        cell: (context) => <RowToggle row={context.row} />,
        enableSorting: false,
        enableHiding: false,
    };
}

/** Render the button that shows and hides a row's sub rows, with what follows it in the cell. */
function RowToggle(properties: {
    /** The row the button expands. */
    readonly row: object;
    /** What follows the button, such as a group's value and member count. */
    readonly children?: JSX.Element;
}): JSX.Element {
    // read the row's expansion where rows expand
    const locale = useLocale();
    const tracked = useDataTableState();
    const row = properties.row;
    if (!isExpandableRow(row)) {
        return properties.children;
    }
    const isExpanded = (): boolean => {
        tracked();

        return row.getIsExpanded();
    };
    const collapsedIcon = locale.direction === "rtl" ? caretLeft : caretRight;

    return (
        <span data-slot="data-table-toggle" {...style.attrs(styles.toggle(String(row.depth)))}>
            <Show when={row.getCanExpand()} fallback={<span {...style.attrs(styles.selection)} />}>
                <Button
                    variant="ghost"
                    size="icon-sm"
                    aria-expanded={isExpanded() ? "true" : "false"}
                    aria-label={locale.render(isExpanded() ? t`Collapse row` : t`Expand row`)}
                    onClick={() => row.toggleExpanded()}
                >
                    <Icon icon={isExpanded() ? caretDown : collapsedIcon} />
                </Button>
            </Show>
            {properties.children}
        </span>
    );
}

/** Render the box that selects every row of the page, showing a partial selection as indeterminate. */
function PageSelection(properties: {
    /** The table whose page the box selects. */
    readonly table: object;
}): JSX.Element {
    // read the table's selection where rows select
    const locale = useLocale();
    const tracked = useDataTableState();
    const table = properties.table;
    if (!isSelectableTable(table)) {
        return null;
    }
    const isAll = (): boolean => {
        tracked();

        return table.getIsAllPageRowsSelected();
    };
    const isSome = (): boolean => {
        tracked();

        return table.getIsSomePageRowsSelected();
    };

    return (
        <Checkbox
            aria-label={locale.render(t`Select all`)}
            checked={isAll()}
            indeterminate={isSome() && !isAll()}
            onCheckedChange={(isSelected) => table.toggleAllPageRowsSelected(isSelected)}
            xstyle={styles.selection}
        />
    );
}

/** Render the box that selects one row. */
function RowSelection(properties: {
    /** The row the box selects. */
    readonly row: object;
}): JSX.Element {
    // read the row's selection where rows select
    const locale = useLocale();
    const tracked = useDataTableState();
    const row = properties.row;
    if (!isSelectableRow(row)) {
        return null;
    }
    const isSelected = (): boolean => {
        tracked();

        return row.getIsSelected();
    };

    return (
        <Checkbox
            aria-label={locale.render(t`Select row`)}
            checked={isSelected()}
            disabled={!row.getCanSelect()}
            onCheckedChange={(isChecked) => row.toggleSelected(isChecked)}
        />
    );
}

/** Render a header's or cell's template: its text, or the element its function returns. */
function renderTemplate<Context extends object>(
    template: ColumnDefTemplate<Context> | undefined,
    context: Context,
): JSX.Element {
    if (typeof template !== "function") {
        return template;
    }
    const rendered: unknown = template(context);

    return isElement(rendered) ? rendered : null;
}

/** Render a cell: a group's toggle, value and count, a group's aggregate, nothing for a repeated value, or its column's template. */
function renderCell<Features extends TableFeatures, Row extends RowData>(
    cell: Cell<Features, Row>,
): JSX.Element {
    // head a group with its toggle, its value and the number of its members
    const grouped: object = cell;
    const row: object = cell.row;
    if (isGroupedCell(grouped) && grouped.getIsGrouped()) {
        const members = isExpandableRow(row) ? row.subRows.length : 0;

        return (
            <RowToggle row={cell.row}>
                {renderValue(cell, cell.column.columnDef.cell)}
                <span {...style.attrs(styles.members)}>({members})</span>
            </RowToggle>
        );
    }
    // show a group's aggregate of its members' values
    else if (isGroupedCell(grouped) && grouped.getIsAggregated?.() === true) {
        const aggregated: unknown =
            "aggregatedCell" in cell.column.columnDef
                ? cell.column.columnDef.aggregatedCell
                : undefined;

        return renderValue(
            cell,
            isTemplate<CellContext<Features, Row>>(aggregated) ? aggregated : undefined,
        );
    }
    // leave a repeated group value empty
    else if (isGroupedCell(grouped) && grouped.getIsPlaceholder()) {
        return null;
    }
    // render every other cell through its column's template
    else {
        return renderValue(cell, cell.column.columnDef.cell);
    }
}

/** Render a cell through a template, its value as text without one. */
function renderValue<Features extends TableFeatures, Row extends RowData>(
    cell: Cell<Features, Row>,
    template: ColumnDefTemplate<CellContext<Features, Row>> | undefined,
): JSX.Element {
    if (template === undefined) {
        const value: unknown = cell.getValue();

        return typeof value === "string" || typeof value === "number" ? String(value) : null;
    }

    return renderTemplate(template, cell.getContext());
}

/** Report whether a value is a header or cell template: text or a function. */
function isTemplate<Context extends object>(value: unknown): value is ColumnDefTemplate<Context> {
    return typeof value === "string" || typeof value === "function";
}

/** Report whether a value renders as an element. */
function isElement(value: unknown): value is JSX.Element {
    return (
        value === null ||
        value === undefined ||
        typeof value === "string" ||
        typeof value === "number" ||
        typeof value === "boolean" ||
        typeof value === "function" ||
        typeof value === "object"
    );
}

/** Report whether a column sorts through the sorting feature. */
function isSortable(column: object): column is SortableColumn {
    return "getCanSort" in column && typeof column.getCanSort === "function";
}

/** Report whether a column hides through the visibility feature. */
function isHideable(column: object): column is HideableColumn {
    return "getCanHide" in column && typeof column.getCanHide === "function";
}

/** Report whether a table selects rows through the selection feature. */
function isSelectableTable(table: object): table is SelectableTable {
    return (
        "toggleAllPageRowsSelected" in table &&
        typeof table.toggleAllPageRowsSelected === "function"
    );
}

/** Report whether a row selects through the selection feature. */
function isSelectableRow(row: object): row is SelectableRow {
    return "getIsSelected" in row && typeof row.getIsSelected === "function";
}

/** Report whether a row expands through the expanding feature. */
function isExpandableRow(row: object): row is ExpandableRow {
    return "getCanExpand" in row && typeof row.getCanExpand === "function";
}

/** Report whether a cell belongs to a table that groups rows. */
function isGroupedCell(cell: object): cell is GroupedCell {
    return "getIsPlaceholder" in cell && typeof cell.getIsPlaceholder === "function";
}

/** Report a row's nesting level for `aria-level`, absent where rows do not expand. */
function levelOf(row: object): number | undefined {
    return isExpandableRow(row) ? row.depth + 1 : undefined;
}

/** Report a row's expansion for `aria-expanded`, absent on a row without sub rows. */
function expansionOf(row: object): "true" | "false" | undefined {
    if (!isExpandableRow(row) || !row.getCanExpand()) {
        return undefined;
    }

    return row.getIsExpanded() ? "true" : "false";
}

/** Report whether a row is selected. */
function isSelectedRow(row: object): boolean {
    return isSelectableRow(row) && row.getIsSelected();
}

/** Report a row's selection for `aria-selected`, absent where rows do not select. */
function selectionOf(row: object): "true" | "false" | undefined {
    if (!isSelectableRow(row) || !row.getCanSelect()) {
        return undefined;
    }

    return row.getIsSelected() ? "true" : "false";
}

/** Report a header's sort for `aria-sort`, absent on a column that does not sort. */
function sortOf<Features extends TableFeatures, Row extends RowData>(
    header: Header<Features, Row>,
): "ascending" | "descending" | "none" | undefined {
    // read the direction of a column that sorts
    const column: object = header.column;
    if (!isSortable(column) || !column.getCanSort()) {
        return undefined;
    }
    const direction = column.getIsSorted();

    return direction === "asc" ? "ascending" : direction === "desc" ? "descending" : "none";
}

/** Pick the arrow that shows a sort direction. */
function sortIconOf(direction: false | "asc" | "desc"): IconBodies {
    return direction === "asc" ? caretUp : direction === "desc" ? caretDown : caretUpDown;
}

/** A cell's place in a data table's grid: its row among every row, the headers' first, and its column. */
interface GridCell {
    /** The row among every row. */
    readonly row: number;
    /** The column within the row. */
    readonly column: number;
}

/** The cells of a data table's header and body rows, moved through by row and column. */
class TableDelegate implements KeyboardDelegate<string> {
    /** The number of cells in each header row. */
    readonly #headers: Accessor<readonly number[]>;
    /** The number of cells in each body row, every row of the table and not only those in view. */
    readonly #rows: Accessor<readonly number[]>;

    /** Lay out the cells of the header and body rows. */
    constructor(headers: Accessor<readonly number[]>, rows: Accessor<readonly number[]>) {
        this.#headers = headers;
        this.#rows = rows;
    }

    /** Read the cell a key press moves the focus to: along a row, across rows, a page of rows, or to the ends of the row or grid. */
    target(event: KeyboardEvent, from: string, direction: Direction): string | undefined {
        // read the move and the cell it starts from
        const move = gridMoveOf(event, direction);
        if (move === undefined) {
            return undefined;
        }
        const place = placeOf(move, cellOf(from), this.#count() - 1);

        // land on the cell the move reaches, the row's last for a column past its end
        const width = this.#width(place.row);
        if (width === undefined || (place.column !== "last" && place.column < 0)) {
            return undefined;
        }

        return cellKey(
            place.row,
            Math.min(place.column === "last" ? width - 1 : place.column, width - 1),
        );
    }

    /** Read the first cell, the first header's. */
    first(): string | undefined {
        return this.#count() > 0 ? cellKey(0, 0) : undefined;
    }

    /** Report whether a cell stands in the grid. */
    has(key: string): boolean {
        const cell = cellOf(key);
        const width = this.#width(cell.row);

        return width !== undefined && cell.column < width;
    }

    /** The number of rows, the headers' included. */
    #count(): number {
        return this.#headers().length + this.#rows().length;
    }

    /** Read the number of cells in a row, undefined past the ends. */
    #width(row: number): number | undefined {
        const headers = this.#headers();

        return row < headers.length ? headers[row] : this.#rows()[row - headers.length];
    }
}

/** Build the key of a cell from its row and column. */
function cellKey(row: number, column: number): string {
    return `${String(row)}:${String(column)}`;
}

/** Read the row and column of a cell's key. */
function cellOf(key: string): GridCell {
    const [row = "0", column = "0"] = key.split(":");

    return { row: Number(row), column: Number(column) };
}

/** Read the row and column a grid move lands the focus on, the last column of a row as `last`. */
function placeOf(
    move: GridMove,
    cell: GridCell,
    last: number,
): { readonly row: number; readonly column: number | "last" } {
    // step a cell along a row or a column
    const { row, column } = cell;
    if (move === "next") {
        return { row, column: column + 1 };
    } else if (move === "previous") {
        return { row, column: column - 1 };
    } else if (move === "down") {
        return { row: row + 1, column };
    } else if (move === "up") {
        return { row: row - 1, column };
    }
    // jump a page of rows
    else if (move === "pageDown") {
        return { row: Math.min(row + PAGE_ROWS, last), column };
    } else if (move === "pageUp") {
        return { row: Math.max(row - PAGE_ROWS, 0), column };
    }
    // the ends of the row or of the grid
    else if (move === "rowStart") {
        return { row, column: 0 };
    } else if (move === "rowEnd") {
        return { row, column: "last" };
    } else if (move === "gridStart") {
        return { row: 0, column: 0 };
    } else {
        return { row: last, column: "last" };
    }
}

/** Move the focused cell on arrow keys, Home, End, Page Up and Page Down, leaving keys to text fields inside cells. */
function navigate(event: KeyboardEvent, focus: Focus<string>, direction: Direction): void {
    const target = event.target;
    if (
        target instanceof Element &&
        target.closest("th, td") !== null &&
        !target.matches("input:not([type=checkbox]), textarea, select")
    ) {
        focus.move(event, direction);
    }
}

/** The reference and handler that make a header or body cell one place of a data table's grid. */
interface GridCellAttributes extends Record<"ref", (element: HTMLTableCellElement) => void> {
    /** Follow the focus into the cell. */
    readonly onFocusIn: () => void;
}

/** Make a cell one place of its data table's grid: the tab stop while focused, its single control taking the focus in its place. */
function useGridCell(key: () => string): GridCellAttributes {
    // read the table's focus and take the cell's element
    const focus = useContext(DataTableFocusContext);
    if (focus === null) {
        throw new TypeError("a data table cell needs a data table around it");
    }
    let element: HTMLTableCellElement | undefined;

    // focus the cell's single control, else the cell, unless the focus already rests inside it
    focus.bind(key, () =>
        element === undefined || element.contains(document.activeElement)
            ? undefined
            : focusOf(element),
    );

    // put the focused cell's single control, else the cell, alone in the tab order once its content renders
    createEffect(
        () => focus.isActive(key()),
        (isActive) => {
            if (element === undefined) {
                return;
            }
            const target = focusOf(element);
            element.tabIndex = isActive && target === element ? 0 : -1;
            for (const control of element.querySelectorAll<HTMLElement>(FOCUSABLE)) {
                control.tabIndex = isActive && target === control ? 0 : -1;
            }
        },
    );

    return {
        ref: (cell) => {
            element = cell;
        },
        onFocusIn: () => focus.focusIn(key()),
    };
}

/** Find the element that takes a cell's focus: its single control, else the cell. */
function focusOf(cell: HTMLElement): HTMLElement {
    const controls = cell.querySelectorAll<HTMLElement>(FOCUSABLE);
    const [control] = controls;

    return controls.length === 1 && control !== undefined ? control : cell;
}
