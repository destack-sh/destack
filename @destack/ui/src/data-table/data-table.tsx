import { Icon, type IconBodies } from "@destack/icon";
import caretDoubleLeft from "@destack/icon/phosphor/caret-double-left";
import caretDoubleRight from "@destack/icon/phosphor/caret-double-right";
import caretDown from "@destack/icon/phosphor/caret-down";
import caretLeft from "@destack/icon/phosphor/caret-left";
import caretRight from "@destack/icon/phosphor/caret-right";
import caretUp from "@destack/icon/phosphor/caret-up";
import caretUpDown from "@destack/icon/phosphor/caret-up-down";
import slidersHorizontal from "@destack/icon/phosphor/sliders-horizontal";
import { plural, t } from "@destack/locale";
import { useLocale } from "@destack/locale/solid";
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
    useContext,
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
import { createVirtualList } from "./virtual.ts";

/** The rows a page jump moves by with Page Up and Page Down. */
const PAGE_ROWS = 10;

/** The page sizes a pagination offers without its own. */
const PAGE_SIZES: readonly number[] = [10, 20, 50, 100];

/** The elements inside a cell that take the focus in its place. */
const FOCUSABLE = "a[href], button, input, select, textarea, [tabindex]:not(td, th)";

/** The state of the nearest data table, read to track it, which reads nothing outside one. */
const DataTableContext = createContext<Accessor<unknown>>(() => undefined);

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
    const read = <Value,>(value: () => Value): Value => {
        properties.table.tracked();

        return value();
    };

    // render only the rows in view when the rows have a height
    const [scroller, setScroller] = createSignal<HTMLElement | undefined>(undefined, {
        ownedWrite: true,
    });
    const list = createVirtualList({
        count: () => rows().length,
        itemHeight: properties.rowHeight ?? 0,
        scrollElement: scroller,
    });
    const shown = createMemo(() =>
        properties.rowHeight === undefined
            ? rows().map((row) => ({ row, start: 0, end: 0 }))
            : list.items().flatMap((item) => {
                  const row = rows()[item.index];

                  return row === undefined ? [] : [{ row, start: item.start, end: item.end }];
              }),
    );
    const before = (): number => shown()[0]?.start ?? 0;
    const after = (): number => list.height() - (shown().at(-1)?.end ?? 0);

    // keep one tab stop in the grid as its rows change
    let grid: HTMLElement | undefined;
    createEffect(shown, () => {
        if (grid !== undefined) {
            roam(grid, grid.querySelector("[data-grid-active]") ?? grid.querySelector("th, td"));
        }
    });

    return (
        <div
            data-slot="data-table"
            {...rest}
            ref={(element) => {
                grid = element;
                setScroller(properties.rowHeight === undefined ? undefined : element);
            }}
            onKeyDown={(event) => navigate(event)}
            onFocusIn={(event) => {
                // move the tab stop to the cell the focus entered
                const cell =
                    event.target instanceof Element ? event.target.closest("th, td") : null;
                if (cell !== null) {
                    roam(event.currentTarget, cell);
                }
            }}
            {...style.attributes(
                [properties.rowHeight !== undefined && styles.scroller, properties.xstyle],
                properties.style,
            )}
        >
            <DataTableContext value={properties.table.tracked}>
                <Table role="grid" aria-rowcount={rows().length + headerGroups().length}>
                    <TableHeader>
                        <For each={headerGroups()}>
                            {(group) => (
                                <TableRow>
                                    <For each={group.headers}>
                                        {(header) => (
                                            <TableHead
                                                colspan={header.colSpan}
                                                aria-sort={read(() => sortOf(header))}
                                                data-column={header.column.id}
                                            >
                                                <Show when={!header.isPlaceholder}>
                                                    {renderTemplate(
                                                        header.column.columnDef.header,
                                                        header.getContext(),
                                                    )}
                                                </Show>
                                            </TableHead>
                                        )}
                                    </For>
                                </TableRow>
                            )}
                        </For>
                    </TableHeader>
                    <TableBody>
                        <Show when={before() > 0}>
                            <tr aria-hidden="true">
                                <td
                                    colspan={width()}
                                    {...style.attrs(styles.spacer(`${before()}px`))}
                                />
                            </tr>
                        </Show>
                        <For
                            each={shown()}
                            fallback={
                                <TableRow>
                                    <TableCell colspan={width()} xstyle={styles.empty}>
                                        {properties.empty ?? locale.render(t`No results.`)}
                                    </TableCell>
                                </TableRow>
                            }
                        >
                            {(entry) => (
                                <TableRow
                                    data-state={read(() =>
                                        isSelectedRow(entry.row) ? "selected" : undefined,
                                    )}
                                    aria-selected={read(() => selectionOf(entry.row))}
                                >
                                    <For each={entry.row.getAllCells()}>
                                        {(cell) => (
                                            <TableCell data-column={cell.column.id}>
                                                {renderCell(cell)}
                                            </TableCell>
                                        )}
                                    </For>
                                </TableRow>
                            )}
                        </For>
                        <Show when={after() > 0}>
                            <tr aria-hidden="true">
                                <td
                                    colspan={width()}
                                    {...style.attrs(styles.spacer(`${after()}px`))}
                                />
                            </tr>
                        </Show>
                    </TableBody>
                </Table>
            </DataTableContext>
        </div>
    );
}

/** Read the nearest data table's state in a header or cell template, to update it as the table changes. */
export function useDataTableState(): Accessor<unknown> {
    return useContext(DataTableContext);
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
    const tracked = useContext(DataTableContext);
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
    const page = (): PaginationState => {
        table.tracked();

        return table.store.state.pagination;
    };
    const count = (): number => {
        table.tracked();

        return table.getPageCount();
    };
    const selected = (): number | undefined => {
        table.tracked();

        return table.getSelectedRowModel?.().rows.length;
    };
    const total = (): number => {
        table.tracked();

        return table.getRowCount();
    };

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
                    disabled={!table.getCanPreviousPage()}
                    go={() => table.firstPage()}
                />
                <PageStep
                    label={locale.render(t`Go to the previous page`)}
                    icon={caretLeft}
                    disabled={!table.getCanPreviousPage()}
                    go={() => table.previousPage()}
                />
                <PageStep
                    label={locale.render(t`Go to the next page`)}
                    icon={caretRight}
                    disabled={!table.getCanNextPage()}
                    go={() => table.nextPage()}
                />
                <PageStep
                    label={locale.render(t`Go to the last page`)}
                    icon={caretDoubleRight}
                    disabled={!table.getCanNextPage()}
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
    /** Name a column by its id, the id by default. */
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

/** Render the box that selects every row of the page, showing a partial selection as indeterminate. */
function PageSelection(properties: {
    /** The table whose page the box selects. */
    readonly table: object;
}): JSX.Element {
    // read the table's selection where rows select
    const locale = useLocale();
    const tracked = useContext(DataTableContext);
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
    const tracked = useContext(DataTableContext);
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

/** Render a cell through its column's template, its value as text without one. */
function renderCell<Features extends TableFeatures, Row extends RowData>(
    cell: Cell<Features, Row>,
): JSX.Element {
    const template = cell.column.columnDef.cell;
    if (template === undefined) {
        const value: unknown = cell.getValue();

        return typeof value === "string" || typeof value === "number" ? String(value) : null;
    }

    return renderTemplate(template, cell.getContext());
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

/** Move the focus between a grid's cells on arrow keys, Home, End, Page Up and Page Down, Control moving to the grid's ends. */
function navigate(event: KeyboardEvent): void {
    // leave keys to text fields and to elements outside cells
    const target = event.target;
    if (
        !(target instanceof Element) ||
        target.matches("input:not([type=checkbox]), textarea, select")
    ) {
        return;
    }
    const cell = target.closest("th, td");
    const row = cell?.parentElement;
    const table = cell?.closest("table");
    if (
        !(cell instanceof HTMLTableCellElement) ||
        !(row instanceof HTMLTableRowElement) ||
        table === null ||
        table === undefined
    ) {
        return;
    }

    // find the cell the key moves to
    const rows = [...table.rows].filter((entry) => entry.getAttribute("aria-hidden") !== "true");
    const rowIndex = rows.indexOf(row);
    const last = rows.length - 1;
    const isControl = event.ctrlKey || event.metaKey;
    const [nextRow, nextColumn] =
        event.key === "ArrowRight"
            ? [rowIndex, cell.cellIndex + 1]
            : event.key === "ArrowLeft"
              ? [rowIndex, cell.cellIndex - 1]
              : event.key === "ArrowDown"
                ? [rowIndex + 1, cell.cellIndex]
                : event.key === "ArrowUp"
                  ? [rowIndex - 1, cell.cellIndex]
                  : event.key === "PageDown"
                    ? [Math.min(rowIndex + PAGE_ROWS, last), cell.cellIndex]
                    : event.key === "PageUp"
                      ? [Math.max(rowIndex - PAGE_ROWS, 0), cell.cellIndex]
                      : event.key === "Home"
                        ? [isControl ? 0 : rowIndex, 0]
                        : event.key === "End"
                          ? [isControl ? last : rowIndex, Number.MAX_SAFE_INTEGER]
                          : [undefined, undefined];
    if (nextRow === undefined || nextColumn === undefined) {
        return;
    }
    const cells = rows[nextRow]?.cells;
    const next = cells?.[Math.min(nextColumn, cells.length - 1)];
    if (next === undefined || nextColumn < 0) {
        return;
    }

    // focus it and keep the key from scrolling the page
    event.preventDefault();
    const grid = event.currentTarget;
    if (grid instanceof HTMLElement) {
        roam(grid, next);
    }
    focusOf(next).focus();
}

/** Give a grid one tab stop: the active cell, or the single control inside it. */
function roam(grid: HTMLElement, active: Element | null): void {
    // take every cell and control out of the tab order
    for (const element of grid.querySelectorAll<HTMLElement>(`th, td, ${FOCUSABLE}`)) {
        element.tabIndex = -1;
        delete element.dataset["gridActive"];
    }

    // put the active cell back
    if (active instanceof HTMLElement) {
        active.dataset["gridActive"] = "";
        focusOf(active).tabIndex = 0;
    }
}

/** Find the element that takes a cell's focus: its single control, else the cell. */
function focusOf(cell: HTMLElement): HTMLElement {
    const controls = cell.querySelectorAll<HTMLElement>(FOCUSABLE);
    const [control] = controls;

    return controls.length === 1 && control !== undefined ? control : cell;
}
