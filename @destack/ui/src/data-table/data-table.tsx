import { Icon } from "@destack/ui/icon";
import type { IconBodies } from "@destack/icon";
import caretDoubleLeft from "@destack/icon/phosphor/caret-double-left";
import caretDoubleRight from "@destack/icon/phosphor/caret-double-right";
import caretDown from "@destack/icon/phosphor/caret-down";
import caretLeft from "@destack/icon/phosphor/caret-left";
import caretRight from "@destack/icon/phosphor/caret-right";
import caretUp from "@destack/icon/phosphor/caret-up";
import caretUpDown from "@destack/icon/phosphor/caret-up-down";
import slidersHorizontal from "@destack/icon/phosphor/sliders-horizontal";
import { type Direction, plural, t } from "@destack/locale";
import { present } from "@destack/schema";
import * as style from "@destack/style";
import { state } from "@destack/theme/state";
import { media } from "@destack/style/media.stylex";
import { color, size, space, stroke, swatch, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    handle,
    useActions,
    type Accessor,
    createContext,
    createEffect,
    createMemo,
    createSignal,
    For,
    type JSX,
    omit,
    onCleanup,
    onSettled,
    Show,
    untrack,
    useContext,
    useLocale,
} from "@destack/view";
import type {
    Cell,
    Cell_ColumnGrouping,
    Cell_RowAggregation,
    CellContext,
    CellSelectionDirection,
    Column,
    Column_ColumnVisibility,
    Column_ColumnPinning,
    Column_ColumnResizing,
    Column_ColumnSizing,
    Column_CoreProperties,
    Column_RowSorting,
    ColumnDefTemplate,
    Header,
    Header_ColumnResizing,
    Header_ColumnSizing,
    HeaderContext,
    PaginationState,
    Row_Core,
    Row_CoreProperties,
    Row_RowExpanding,
    Row_RowSelection,
    RowData,
    Table_RowPagination,
    Table_RowSelection,
    TableFeatures,
    TableState_ColumnGrouping,
    TableState_RowExpanding,
    TableState_RowPagination,
    TableState_RowSelection,
    TableState_RowSorting,
} from "@tanstack/table-core";
import { Button } from "../button/index.ts";
import { Checkbox } from "../checkbox/index.ts";
import { DisclosureCaret, indent, indicator } from "../disclosure/index.ts";
import { mergeProperties } from "../part/index.ts";
import {
    DropdownMenu,
    DropdownMenuCheckboxItem,
    DropdownMenuContent,
    DropdownMenuLabel,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
} from "../dropdown-menu/index.ts";
import type { Action } from "@destack/object";
import { ActionButton, hideColumn } from "../action/index.ts";
import {
    clearSort,
    type DataTableRowAction,
    moveColumnLeft,
    moveColumnRight,
    pinToEnd,
    pinToStart,
    sortAscending,
    sortDescending,
    unpin,
} from "./actions.ts";
import { OpenChange } from "../layer/index.ts";
import { Menu, MenuAction, MenuControl } from "../menu/index.ts";
import { ContextMenuContent } from "../context-menu/index.ts";
import { Input } from "../input/index.ts";
import { Select, SelectItem } from "../select/index.ts";
import {
    tableStyle,
    TableBody,
    TableCell,
    TableFooter,
    TableHead,
    TableHeader,
    TableRow,
} from "../table/index.ts";
import {
    Sortable,
    SortableContainer,
    SortableHandle,
    SortableItem,
    SortableMove,
} from "../sortable/index.ts";
import {
    type CellKey,
    type DataTableCellChange,
    DataTableControl,
    type DataTableEditor,
    type DataTableEditorKind,
    type DataTableRowMove,
    type DataTableSpan,
    type DataTableSummary,
    FOCUSABLE,
    focusOf,
    isGroupCell,
    isGroupRow,
    isSelectableCell,
    keyText,
    metaOf,
} from "./control.ts";
import { createGutter, type DataTableGutter } from "./gutter.ts";
import type { DataTableInstance } from "./table.ts";
import { Length, Virtualizer } from "../virtualizer/index.ts";
import { VisuallyHidden, visuallyHiddenStyle } from "../visually-hidden/index.ts";

/** The page sizes a pagination offers without its own. */
const PAGE_SIZES: readonly number[] = [10, 20, 50, 100];

/** The directions an arrow key extends a block of cells in, a row's in the reading direction. */
const ARROWS: Readonly<Partial<Record<string, (direction: Direction) => CellSelectionDirection>>> =
    {
        ArrowUp: () => "up",
        ArrowDown: () => "down",
        ArrowLeft: (direction) => (direction === "rtl" ? "right" : "left"),
        ArrowRight: (direction) => (direction === "rtl" ? "left" : "right"),
    };

/** The elements in a cell whose presses start no block of cells: controls, handles, the resizer. */
const PRESSED =
    "a[href], button, input, select, textarea, [role=checkbox], [data-slot=sortable-handle], [data-slot=data-table-resizer]";

/** The state of the nearest data table and its slices, read to track them, null outside one. */
const DataTableContext = createContext<DataTableSlices | null>(null);

/** The line a row's cells show along the edge a dragged row lands on, as sortable items show it. */
const LINE = `calc(2 * ${stroke.border})`;

/** The prefix of a row's id among the sortable items, apart from the columns'. */
const ROW_PREFIX = "row:";

/** The prefix of a column's id among the sortable items, apart from the rows'. */
const COLUMN_PREFIX = "column:";

/** The sortable list of a table's rows, which a row's move reports. */
const ROWS = "data-table-rows";

/** The sortable row of a table's columns, which a column's move reports. */
const COLUMNS = "data-table-columns";

/** The drag type of a table's rows. */
const ROW_TYPE = "data-table-row";

/** The drag type of a table's columns. */
const COLUMN_TYPE = "data-table-column";

/** The styles of a data table and its toolbar and footer. */
const styles = style.create({
    frame: {
        overflow: "auto",
    },
    hung: (gutter: string) => ({
        width: `calc(100% + ${gutter})`,
        marginInlineStart: `calc(-1 * ${gutter})`,
    }),
    row: {
        borderBottomWidth: 0,
        backgroundColor: "transparent",
    },
    fixed: {
        height: size[2],
        paddingBlock: 0,
        paddingInline: space[2],
    },
    lined: {
        borderBlockEndWidth: stroke.border,
        borderBlockEndStyle: "solid",
        borderBlockEndColor: color.border,
    },
    lidded: {
        borderBlockStartWidth: stroke.border,
        borderBlockStartStyle: "solid",
        borderBlockStartColor: color.border,
    },
    scroller: {
        overflowY: "auto",
        maxHeight: "100%",
    },
    spacer: (height: string) => ({ height, padding: 0 }),
    head: {
        position: "sticky",
        top: 0,
        zIndex: 2,
        backgroundColor: color.background,
        color: color.mutedForeground,
        fontWeight: weight.regular,
    },
    divided: {
        borderInlineEndWidth: { default: 0, ":not(:last-child)": stroke.border },
        borderInlineEndStyle: "solid",
        borderInlineEndColor: color.border,
    },
    rowControl: {
        opacity: {
            default: 1,
            [media.hover]: {
                default: 0,
                ":is(tr:hover *)": 1,
                ":focus-visible": 1,
                ":is([data-checked])": 1,
            },
        },
    },
    footer: {
        backgroundColor: "transparent",
        fontWeight: weight.regular,
    },
    pinnedStart: (pixels: string, controls: string) => ({
        insetInlineStart: `calc(${pixels} + ${controls} * ${size[1]})`,
        insetInlineEnd: "auto",
    }),
    pinnedEnd: (pixels: string, controls: string) => ({
        insetInlineStart: "auto",
        insetInlineEnd: `calc(${pixels} + ${controls} * ${size[1]})`,
    }),
    pinned: {
        position: "sticky",
        zIndex: 1,
        backgroundColor: color.background,
        backgroundImage: {
            default: null,
            ":is(tbody tr:hover > *)": {
                default: null,
                [media.hover]: `linear-gradient(color-mix(in oklab, ${color.muted} 50%, transparent), color-mix(in oklab, ${color.muted} 50%, transparent))`,
            },
        },
    },
    sized: (width: string) => ({ width, minWidth: width, maxWidth: width }),
    resizer: {
        position: "absolute",
        top: 0,
        insetInlineEnd: 0,
        width: space[1],
        height: "100%",
        cursor: "col-resize",
        touchAction: "none",
        userSelect: "none",
        backgroundColor: {
            default: "transparent",
            ":hover": { default: null, [media.hover]: color.border },
        },
    },
    resizing: {
        backgroundColor: swatch.accentMark,
    },
    bulkBar: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
        paddingBlock: space[2],
    },
    sortable: {
        display: "contents",
    },
    headerContent: {
        display: "flex",
        alignItems: "center",
        gap: space[1],
    },
    end: {
        justifyContent: "flex-end",
        textAlign: "end",
    },
    columnHeader: {
        display: "inline-flex",
        alignItems: "center",
        gap: space[1],
    },
    priority: {
        color: color.mutedForeground,
        fontVariantNumeric: "tabular-nums",
    },
    summary: {
        color: color.mutedForeground,
        fontVariantNumeric: "tabular-nums",
        whiteSpace: "nowrap",
    },
    cell: {
        backgroundColor: {
            default: null,
            ":is(tbody tr:hover > *)": {
                default: null,
                [media.hover]: `color-mix(in oklab, ${color.muted} 50%, transparent)`,
            },
            ":is([data-in-range])": swatch.accentTint,
        },
        boxShadow: {
            default: null,
            ":is([data-drop=before] > *)": `inset 0 ${LINE} 0 ${swatch.accentMark}`,
            ":is([data-drop=after] > *)": `inset 0 calc(-1 * ${LINE}) 0 ${swatch.accentMark}`,
        },
    },
    unmarked: {
        "::after": { content: "none" },
        "::before": { content: "none" },
    },
    selecting: {
        userSelect: "none",
    },
    refused: {
        outlineStyle: "solid",
        outlineWidth: stroke.ring,
        outlineOffset: `calc(-1 * ${stroke.ring})`,
        outlineColor: swatch.destructiveMark,
    },

    editor: {
        width: `calc(100% + 2 * ${space[2]})`,
        height: size[2],
        marginInline: `calc(-1 * ${space[2]})`,
        paddingInline: `calc(${space[2]} - ${stroke.border})`,
    },
    empty: {
        height: `calc(6 * ${space[4]})`,
        textAlign: "center",
        color: color.mutedForeground,
    },
    sort: {
        marginInlineStart: `calc(-1 * ${space[3]})`,
        color: "inherit",
        fontWeight: "inherit",
    },
    reversed: {
        flexDirection: "row-reverse",
    },
    sortEnd: {
        flexDirection: "row-reverse",
        marginInlineStart: 0,
        marginInlineEnd: `calc(-1 * ${space[3]})`,
    },
    quiet: {
        opacity: {
            default: 1,
            [media.hover]: {
                default: 0,
                ":is(th:hover *)": 1,
                ":focus-visible": 1,
                ":is([aria-expanded=true])": 1,
            },
        },
    },
    selection: {
        width: space[6],
    },
    box: {
        display: "block",
    },
    tableQuiet: {
        opacity: {
            default: 1,
            [media.hover]: {
                default: 0,
                ":is(table:hover *)": 1,
                ":focus-visible": 1,
                ":is([data-checked], [aria-checked=mixed])": 1,
            },
        },
    },
    control: {
        width: size[1],
        minWidth: size[1],
        maxWidth: size[1],
        paddingInline: 0,
        textAlign: "center",
    },
    gutter: {
        backgroundColor: "transparent",
        backgroundImage: "none",
    },
    toggle: {
        display: "inline-flex",
        alignItems: "center",
        gap: space[1],
    },
    expand: {
        marginInlineStart: `calc(-1 * ${space[2]})`,
    },
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
        whiteSpace: "nowrap",
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

/** The properties of a data table: the table it renders, its empty state and its scrolling. */
export interface DataTableProperties<
    Features extends TableFeatures,
    Row extends RowData,
> extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "onKeyDown"> {
    /** The headless table, from `createTable`. */
    readonly table: DataTableInstance<Features, Row>;
    /** The content shown while no row matches, a message by default. */
    readonly empty?: JSX.Element;
    /** Whether only the rows in view render, the table scrolling inside its box, read once. */
    readonly isVirtual?: boolean | undefined;
    /** Take changed cells from an edit or a paste, in one call each. */
    readonly onCellsChange?: (changes: readonly DataTableCellChange[]) => void;
    /** Take a row a person moved between two rows, which lets rows move by their handles. */
    readonly onRowMove?: (move: DataTableRowMove) => void;
    /** What the owner does with rows: in a row's menu and by its keys, read once. */
    readonly rowActions?: readonly DataTableRowAction[];
    /** Where the row controls stand, in the margin by default where it has room. */
    readonly gutter?: DataTableGutter;
    /** The StyleX styles applied after the table's styles. */
    readonly xstyle?: style.Styles;
}

/** Render a table's headers and rows as a grid. */
export function DataTable<Features extends TableFeatures, Row extends RowData>(
    properties: DataTableProperties<Features, Row>,
): JSX.Element {
    // read the header groups, the rows and the shown columns as the table changes
    const locale = useLocale();
    const rest = omit(
        properties,
        "table",
        "empty",
        "isVirtual",
        "onCellsChange",
        "onRowMove",
        "rowActions",
        "gutter",
        "xstyle",
        "style",
    );
    const headerGroups = createMemo(() => {
        properties.table.tracked();

        return properties.table.getHeaderGroups();
    });
    const rows = createMemo(() => {
        properties.table.tracked();

        return properties.table.getRowModel().rows;
    });
    const leafHeaders = () => headerGroups().at(-1)?.headers ?? [];
    const columns = createMemo(() => leafHeaders().map((header) => header.column.id), {
        equals: isSameList,
    });
    const width = (): number => Math.max(columns().length, 1);

    // lay the headers and rows out as a grid that edits, copies, pastes and selects blocks
    const [root, setRoot] = createSignal<HTMLElement | undefined>(undefined, { ownedWrite: true });
    const isVirtual = untrack(() => properties.isVirtual === true);
    const gutter = createGutter(root, columns, () => properties.gutter ?? "outside");
    const control = new DataTableControl({
        locale: useLocale(),
        table: untrack(() => properties.table),
        rows,
        columns,
        headers: createMemo(
            () => headerGroups().map((group) => group.headers.map((header) => header.colSpan)),
            { equals: (previous, next) => JSON.stringify(previous) === JSON.stringify(next) },
        ),
        root,
        reveal: (row) => {
            if (isVirtual) {
                virtualizer.scrollToIndex(row);
            }
        },
        onCellsChange: untrack(() => properties.onCellsChange),
    });

    // mount the focused and the edited row out of view, as their elements have the focus
    const persisted = createMemo(
        () => {
            const focused = control.focus.current();
            const ids = new Set([
                focused?.kind === "body" ? focused.row : undefined,
                control.edit()?.row,
            ]);

            return rows().flatMap((row, index) => (ids.has(row.id) ? [index] : []));
        },
        { equals: isSameList },
    );

    // render only the rows in view when the rows have a height
    const [scroller, setScroller] = createSignal<HTMLElement | undefined>(undefined, {
        ownedWrite: true,
    });
    const rowLength = new Length(() => size[2]);
    const estimate = createMemo(() => {
        // estimate every row at the measured row height, a new height laying the rows out again
        const pixels = rowLength.pixels();

        return () => present(pixels, "the data table's row height");
    });
    const virtualizer = new Virtualizer({
        count: () => rows().length,
        estimateSize: estimate,
        scrollElement: scroller,
        persisted: () => persisted(),
        isEnabled: () => isVirtual && rowLength.pixels() !== undefined,
    });
    const shown = createMemo(() =>
        !isVirtual
            ? rows().map((row, index) => ({ row, index }))
            : virtualizer.items().flatMap((item) => {
                  const row = rows()[item.index];

                  return row === undefined ? [] : [{ row, index: item.index }];
              }),
    );
    const spacers = () => (isVirtual ? virtualizer.spacers() : { before: 0, after: 0 });

    // announce a refused paste by its text and the column that refused it
    const refusalText = () => {
        // say nothing without a refusal
        const refusal = control.refusal();
        if (refusal === undefined) {
            return undefined;
        }

        // read the column of the refused cell
        const cell = present(
            control.cellAt({ kind: "body", row: refusal.row, column: refusal.column }),
            `the refused cell '${refusal.row}' '${refusal.column}'`,
        );

        return locale.render(t`Can't paste “${refusal.text}” into ${labelOf(cell.column)}`);
    };

    // follow the state, and the slices of it the rows read apart from the cells' selection
    const tracked: Accessor<DataTableState> = untrack(() => properties.table.tracked);
    const slices: DataTableSlices = {
        state: tracked,
        rowSelection: createMemo(() => tracked().rowSelection),
        expanded: createMemo(() => tracked().expanded),
        isArranged: createMemo(() => isArranged(tracked())),
        hasRowMenu: untrack(() => properties.rowActions !== undefined),
        changeColumn: (column, run) => control.changeColumn(column, run),
    };

    // move rows by their handles where the owner takes moves, and columns where the table orders them
    const isRowOrdered = untrack(() => properties.onRowMove !== undefined);
    const table: object = untrack(() => properties.table);
    const isColumnOrdered = isOrderedTable(table);
    const onMove = (move: SortableMove) => {
        // hand a row's move to the owner, and place a column in the table's order
        if (move.container === ROWS) {
            properties.onRowMove?.({
                id: idOf(move.id),
                previous: move.previous === undefined ? undefined : idOf(move.previous),
                next: move.next === undefined ? undefined : idOf(move.next),
            });
        } else if (isOrderedTable(table)) {
            placeColumn(table, move);
        }
    };

    // run the owner's actions on the selection a row is in, else on the row alone
    const actions = useActions();
    const rowActions = untrack(() => properties.rowActions ?? []);
    const targetsAt = (element: Element | null) => {
        const id = element?.closest<HTMLElement>("tr[data-row]")?.dataset["row"];
        const selected = isSelectableTable(table)
            ? table.getSelectedRowModel().rows.map((row) => row.id)
            : [];

        return id === undefined || selected.includes(id) ? selected : [id];
    };
    const rowHandlers = handleRowActions(rowActions, targetsAt);

    // open a menu of the owner's actions on a row, with its selection, from a cell at a point
    const menu = new MenuControl(null);
    const openRowMenu = (id: string, cell: HTMLElement, point: { x: number; y: number }) => {
        // leave the browser's menu where no action runs
        if (!rowActions.some(({ action }) => actions.has(action.reference, cell))) {
            return false;
        }
        menu.open(
            "first",
            new OpenChange("trigger-press", undefined, { element: cell, payload: undefined }),
            point,
        );

        return true;
    };

    // place each pinned column at its edge, once per change of the pinning or the sizes
    const pins = createMemo(
        () => {
            properties.table.tracked();

            return pinsOf(leafHeaders().map((header) => header.column));
        },
        { equals: isSamePins },
    );

    return (
        <div
            data-slot="data-table"
            {...rest}
            ref={(element) => {
                setRoot(element);
                rowHandlers.forEach((attach) => attach(element));
                setScroller(isVirtual ? element : undefined);
            }}
            onKeyDown={(event) => keyDown(event, control, openRowMenu, locale.direction)}
            onClick={(event) => {
                // open a row's menu under its grip on a click
                const grip =
                    event.target instanceof Element
                        ? event.target.closest<HTMLElement>("[data-slot=data-table-row-handle]")
                        : null;
                if (grip !== null) {
                    const cell = present(grip.closest("td"), "the row grip's cell");
                    const row = present(grip.closest("tr"), "the row grip's row").dataset["row"];
                    const bounds = grip.getBoundingClientRect();
                    openRowMenu(present(row, "the row grip's row id"), cell, {
                        x: bounds.left,
                        y: bounds.bottom,
                    });
                }
            }}
            onCopy={(event) => control.copy(event)}
            onPaste={(event) => control.paste(event)}
            onFocusOut={(event) => control.focus.focusOut(event)}
            {...style.attributes(
                [
                    styles.frame,
                    gutter() > 0 && styles.hung(`${String(gutter())}px`),
                    isVirtual && styles.scroller,
                    properties.xstyle,
                ],
                properties.style,
            )}
        >
            {/* Refused paste */}
            <div
                role="status"
                data-slot="data-table-refusal"
                {...style.attrs(visuallyHiddenStyle())}
            >
                {refusalText()}
            </div>

            {/* Row height */}
            <Show when={isVirtual}>{rowLength.render()}</Show>

            <Menu control={menu}>
                <DataTableContext value={slices}>
                    <Sortable onMove={onMove} xstyle={styles.sortable}>
                        <table
                            data-slot="table"
                            {...style.attrs(tableStyle())}
                            role={follow(properties.table.tracked, () =>
                                rows().some(isExpandableRow) ? "treegrid" : "grid",
                            )}
                            aria-multiselectable={control.isSelecting() ? "true" : undefined}
                            aria-rowcount={rows().length + headerGroups().length}
                        >
                            <TableHeader>
                                <For each={headerGroups()}>
                                    {(group, row) => (
                                        <HeaderRow
                                            isOrdered={
                                                isColumnOrdered &&
                                                row() === headerGroups().length - 1
                                            }
                                            aria-rowindex={row() + 1}
                                        >
                                            <For each={group.headers}>
                                                {(header, column) => (
                                                    <DataTableHeaderCell
                                                        control={control}
                                                        pins={pins}
                                                        header={header}
                                                        cell={{
                                                            kind: "head",
                                                            row: row(),
                                                            column: column(),
                                                        }}
                                                        isOrdered={
                                                            isColumnOrdered &&
                                                            row() === headerGroups().length - 1
                                                        }
                                                    />
                                                )}
                                            </For>
                                        </HeaderRow>
                                    )}
                                </For>
                            </TableHeader>
                            <BodyRows isOrdered={isRowOrdered}>
                                <SpacerRow height={spacers().before} width={width()} />
                                <For
                                    each={shown()}
                                    keyed={(entry) => entry.row.id}
                                    fallback={
                                        <TableRow>
                                            <TableCell colspan={width()} xstyle={styles.empty}>
                                                {properties.empty ?? locale.render(t`No results.`)}
                                            </TableCell>
                                        </TableRow>
                                    }
                                >
                                    {(entry) => (
                                        <BodyRow
                                            control={control}
                                            pins={pins}
                                            row={entry().row}
                                            index={headerGroups().length + entry().index + 1}
                                            isOrdered={isRowOrdered}
                                            onContextMenu={(id, event) => {
                                                // open the row's menu at the pointer, from the cell it is on
                                                const cell =
                                                    event.target instanceof Element
                                                        ? event.target.closest<HTMLElement>("td")
                                                        : null;
                                                const point = {
                                                    x: event.clientX,
                                                    y: event.clientY,
                                                };
                                                if (cell !== null && openRowMenu(id, cell, point)) {
                                                    event.preventDefault();
                                                }
                                            }}
                                        />
                                    )}
                                </For>
                                <SpacerRow height={spacers().after} width={width()} />
                            </BodyRows>
                            <Show
                                when={leafHeaders().some(
                                    (header) => metaOf(header.column).summary !== undefined,
                                )}
                            >
                                <TableFooter xstyle={styles.footer}>
                                    <TableRow data-slot="data-table-summaries" xstyle={styles.row}>
                                        <For each={leafHeaders()}>
                                            {(header) => (
                                                <DataTableSummaryCell
                                                    pins={pins}
                                                    table={properties.table}
                                                    column={header.column}
                                                />
                                            )}
                                        </For>
                                    </TableRow>
                                </TableFooter>
                            </Show>
                        </table>
                    </Sortable>
                </DataTableContext>
                <Show when={rowActions.length > 0}>
                    <ContextMenuContent data-slot="data-table-row-menu">
                        <For each={rowActions}>
                            {({ action }) => <MenuAction action={action} />}
                        </For>
                    </ContextMenuContent>
                </Show>
            </Menu>
        </div>
    );
}

/** Render a header cell of a data table, marked with its column's sort. */
function DataTableHeaderCell<Features extends TableFeatures, Data extends RowData>(properties: {
    /** The grid the cell is a place of. */
    readonly control: DataTableControl<Features, Data>;
    /** Each pinned column's offsets at its edges, by column id. */
    readonly pins: Accessor<ReadonlyMap<string, PinPlace | undefined>>;
    /** The header the cell shows. */
    readonly header: Header<Features, Data>;
    /** The key of the cell's place among the header rows. */
    readonly cell: CellKey;
    /** Whether the header moves along its row, dragged by pointer. */
    readonly isOrdered: boolean;
}): JSX.Element {
    // render the header, as an item of the sortable row where columns move by a drag of the header
    const tracked = useDataTableState();
    const pins = properties.pins;
    const header = properties.header;
    const cell = (attributes: object) => {
        // follow the grid for each header element, as pinning the column swaps its element
        const gridCell = useGridCell(
            properties.control,
            () => properties.cell,
            () => undefined,
        );

        return (
            <TableHead
                {...mergeProperties(attributes, gridCell)}
                colspan={header.colSpan}
                xstyle={[
                    state.focusInset,
                    styles.head,
                    styles.fixed,
                    metaOf(header.column).isControl !== true && [
                        styles.divided,
                        styles.lined,
                        styles.lidded,
                    ],
                    metaOf(header.column).isControl === true
                        ? styles.control
                        : follow(tracked, () => sizeOf(header)),
                    pinStyleOf(pins().get(header.column.id)),
                ]}
                aria-sort={follow(tracked, () => sortOf(header))}
                data-column={header.column.id}
                data-control={metaOf(header.column).isControl === true ? "" : undefined}
            >
                <span {...style.attrs(styles.headerContent, isEnd(header.column) && styles.end)}>
                    <Show when={!header.isPlaceholder}>
                        {renderTemplate(header.column.columnDef.header, header.getContext())}
                    </Show>
                </span>
                <ColumnResizer header={header} />
            </TableHead>
        );
    };

    // move the header while its column moves, a pinned one standing out of the moving headers
    const isMoving = () => properties.isOrdered && follow(tracked, () => isMovable(header));

    return (
        <Show when={isMoving()} fallback={cell({})}>
            <SortableItem
                id={`${COLUMN_PREFIX}${header.column.id}`}
                type={COLUMN_TYPE}
                label={labelOf(header.column)}
                activator="item"
                sensors={["pointer"]}
                render={(attributes) => cell(attributes)}
            />
        </Show>
    );
}

/** Render a body row of a data table, marked with its selection, level and expansion. */
function BodyRow<Features extends TableFeatures, Data extends RowData>(properties: {
    /** The grid the row is part of. */
    readonly control: DataTableControl<Features, Data>;
    /** Each pinned column's offsets at its edges, by column id. */
    readonly pins: Accessor<ReadonlyMap<string, PinPlace | undefined>>;
    /** The row, with its cells, the same record's newest row as the data changes. */
    readonly row: Row_Core<Features, Data>;
    /** The row's place among every row of the grid from one, the headers' included. */
    readonly index: number;
    /** Whether the row moves among the rows by its handle. */
    readonly isOrdered: boolean;
    /** Open the row's menu at a right click. */
    readonly onContextMenu: (id: string, event: MouseEvent) => void;
}): JSX.Element {
    // read the row's cells in the columns' order, its edited column, and its selected runs
    const control = properties.control;
    const slices = useDataTableSlices();
    const id = untrack(() => properties.row.id);
    const cells = createMemo(() => control.cellsOf(properties.row));
    const edited = createMemo(() => {
        const edit = control.edit();

        return edit?.row === id ? edit.column : undefined;
    });
    const spans = createMemo(() => control.spansOf(properties.row), { equals: isSameSpans });
    const first = createMemo(
        () => cells().find((each) => metaOf(each.column).isControl !== true)?.column.id,
    );
    const element = (attributes: object) => (
        <TableRow
            {...attributes}
            xstyle={styles.row}
            aria-rowindex={properties.index}
            data-row={id}
            onContextMenu={(event: MouseEvent) => properties.onContextMenu(id, event)}
            data-selected={follow(slices.rowSelection, () =>
                isSelectedRow(properties.row) ? "" : undefined,
            )}
            aria-selected={follow(slices.rowSelection, () => selectionOf(properties.row))}
            aria-level={levelOf(properties.row)}
            aria-expanded={follow(slices.expanded, () => expansionOf(properties.row))}
        >
            <For each={cells()} keyed={(cell) => cell.column.id}>
                {(cell) => (
                    <DataTableBodyCell
                        control={control}
                        pins={properties.pins}
                        cell={cell()}
                        depth={cell().column.id === first() ? depthOf(properties.row) : 0}
                        isEditing={edited() === cell().column.id}
                        isInBlock={isInSpans(spans(), control.columnPlaceOf(cell().column.id))}
                    />
                )}
            </For>
        </TableRow>
    );

    // render the row, as an item of the sortable body where rows move while no sort or group applies
    return untrack(() => properties.isOrdered) ? (
        <SortableItem
            id={`${ROW_PREFIX}${id}`}
            type={ROW_TYPE}
            disabled={slices.isArranged()}
            xstyle={styles.unmarked}
            render={(attributes) => element(attributes)}
        />
    ) : (
        element({})
    );
}

/** Render a body cell: its content, its editor while edited, a checkbox for a yes or no. */
function DataTableBodyCell<Features extends TableFeatures, Data extends RowData>(properties: {
    /** The grid the cell is a place of. */
    readonly control: DataTableControl<Features, Data>;
    /** Each pinned column's offsets at its edges, by column id. */
    readonly pins: Accessor<ReadonlyMap<string, PinPlace | undefined>>;
    /** The cell, the same column's newest cell as the data changes. */
    readonly cell: Cell<Features, Data>;
    /** The levels its value is indented by, for the first value of a sub row. */
    readonly depth: number;
    /** Whether the cell is the one being edited. */
    readonly isEditing: boolean;
    /** Whether the cell lies in the selected block. */
    readonly isInBlock: boolean;
}): JSX.Element {
    // read the cell anew only for a changed record or value, which spares its content's elements
    const { control, pins } = properties;
    const cell = createMemo(() => properties.cell, {
        equals: (previous, next) =>
            previous.row.original === next.row.original && previous.getValue() === next.getValue(),
    });

    // read the cell's key, how it shows its value, and its editor where it edits
    const key = (): CellKey => ({ kind: "body", row: cell().row.id, column: cell().column.id });
    const isRefused = () => {
        const refusal = control.refusal();

        return refusal?.row === cell().row.id && refusal.column === cell().column.id;
    };
    const isBox = () =>
        metaOf(cell().column).editor?.kind === "checkbox" &&
        !isGroupRow(cell().row) &&
        !isGroupCell(cell());
    const editor = () => control.editorOf(cell());
    const label = () => labelOf(cell().column);
    const isSelectable = (): boolean => {
        const each: object = cell();

        return isSelectableCell(each) && each.getCanSelect();
    };

    // follow the grid for the cell
    const gridCell = useGridCell(control, key, cell);

    return (
        <TableCell
            data-column={cell().column.id}
            data-in-range={properties.isInBlock ? "" : undefined}
            data-editing={properties.isEditing ? "" : undefined}
            aria-selected={
                control.isSelecting() && isSelectable()
                    ? properties.isInBlock
                        ? "true"
                        : "false"
                    : undefined
            }
            onDblClick={() => {
                // edit a cell that edits in a field
                const each = editor();
                if (each !== undefined && each.kind !== "checkbox") {
                    control.begin(cell());
                }
            }}
            xstyle={[
                state.focusInset,
                styles.cell,
                styles.fixed,
                isEnd(cell().column) && styles.end,
                properties.depth > 0 && indent.by(space[2], properties.depth),
                metaOf(cell().column).isControl !== true && [styles.divided, styles.lined],
                control.isSelecting() && styles.selecting,
                isRefused() && styles.refused,
                metaOf(cell().column).isControl === true && styles.control,
                pinStyleOf(pins().get(cell().column.id)),
                metaOf(cell().column).isControl === true && styles.gutter,
            ]}
            aria-invalid={isRefused() ? "true" : undefined}
            {...gridCell}
        >
            {/* A checkbox for a yes or no outside groups, disabled in a read-only cell */}
            <Show
                when={isBox()}
                fallback={
                    <Show
                        when={properties.isEditing ? editor() : undefined}
                        fallback={renderCell(cell())}
                        keyed
                    >
                        {(each) => (
                            <DataTableCellEditor
                                control={control}
                                editor={each}
                                value={cell().getValue()}
                                label={label()}
                            />
                        )}
                    </Show>
                }
            >
                <Checkbox
                    aria-label={label()}
                    checked={cell().getValue() === true}
                    disabled={editor() === undefined}
                    onCheckedChange={(checked) => control.set(cell(), checked)}
                    xstyle={styles.box}
                />
            </Show>
        </TableCell>
    );
}

/** Render a column's summary in the footer: its count, sum, average, smallest or largest. */
function DataTableSummaryCell<Features extends TableFeatures, Data extends RowData>(properties: {
    /** Each pinned column's offsets at its edges, by column id. */
    readonly pins: Accessor<ReadonlyMap<string, PinPlace | undefined>>;
    /** The table whose rows the summary reads. */
    readonly table: DataTableInstance<Features, Data>;
    /** The column. */
    readonly column: Column<Features, Data>;
}): JSX.Element {
    // read the column's values only where it has a summary, once per change of the rows
    const locale = useLocale();
    const pins = properties.pins;
    const column = untrack(() => properties.column);
    const meta = metaOf(column);
    const summary = meta.summary;
    const model = createMemo(() =>
        follow(properties.table.tracked, () => properties.table.getPrePaginatedRowModel()),
    );
    const line = createMemo(() =>
        summary === undefined
            ? undefined
            : summaryText(
                  summary,
                  meta.editor?.kind,
                  model()
                      .flatRows.filter((row) => !isGroupRow(row))
                      .map((row) => row.getValue(column.id)),
                  locale,
              ),
    );

    return (
        <TableCell
            data-column={column.id}
            xstyle={[
                text.xs,
                styles.summary,
                styles.fixed,
                isEnd(column) && styles.end,
                pinStyleOf(pins().get(column.id)),
            ]}
        >
            {line()}
        </TableCell>
    );
}

/** Render the handle that drags a resizable column's width, none for a fixed column. */
function ColumnResizer<Features extends TableFeatures, Data extends RowData>(properties: {
    /** The header whose column the handle sizes. */
    readonly header: Header<Features, Data>;
}): JSX.Element {
    // read the header and its column, which the table's features may size
    const tracked = useDataTableState();
    const header: object = properties.header;
    const column: object = properties.header.column;
    const drag = (event: MouseEvent | TouchEvent) => {
        if (isResizingHeader(header)) {
            header.getResizeHandler()(event);
        }
    };

    return (
        <Show when={isResizable(column) && column.getCanResize()}>
            <div
                aria-hidden="true"
                data-slot="data-table-resizer"
                onPointerDown={(event) => event.stopPropagation()}
                onMouseDown={drag}
                onTouchStart={drag}
                onDblClick={() => isSizable(column) && column.resetSize()}
                {...style.attrs(
                    styles.resizer,
                    follow(tracked, () => isResizable(column) && column.getIsResizing()) &&
                        styles.resizing,
                )}
            />
        </Show>
    );
}

/** Read a header's width where columns size, none otherwise. */
function sizeOf(header: object): style.Styles {
    return isSizedHeader(header) ? styles.sized(`${String(header.getSize())}px`) : null;
}

/** A pinned column's offset from its edge: the columns pinned between it and the edge. */
interface PinPlace {
    /** The edge it sticks to. */
    readonly edge: "start" | "end";
    /** The widths of the sized columns between it and its edge, in pixels. */
    readonly pixels: number;
    /** The number of control columns between it and its edge, each a control's size wide. */
    readonly controls: number;
}

/** Place each pinned column past the pinned columns between it and its edge, none for the rest. */
function pinsOf<Features extends TableFeatures, Data extends RowData>(
    columns: readonly Column<Features, Data>[],
): ReadonlyMap<string, PinPlace | undefined> {
    // start with every column scrolling, then place the pinned ones edge by edge
    const pins = new Map<string, PinPlace | undefined>(
        columns.map((column) => [column.id, undefined]),
    );
    for (const edge of ["start", "end"] as const) {
        // walk in from the edge, each column widening the offset of the next
        const pinned = columns.filter(
            (
                column,
            ): column is Column<Features, Data> & Column_ColumnPinning & Column_ColumnSizing =>
                isPinnable(column) && column.getIsPinned() === edge,
        );
        let pixels = 0;
        let controls = 0;
        for (const column of edge === "start" ? pinned : pinned.toReversed()) {
            pins.set(column.id, { edge, pixels, controls });
            if (metaOf(column).isControl === true) {
                controls += 1;
            }
            // add a sized column's width
            else {
                pixels += column.getSize();
            }
        }
    }

    return pins;
}

/** Read the styles that place a pinned column at its edge, none for a column that scrolls. */
function pinStyleOf(place: PinPlace | undefined): style.Styles {
    // leave a column that scrolls in place
    if (place === undefined) {
        return null;
    }
    const pixels = `${String(place.pixels)}px`;
    const controls = String(place.controls);

    return [
        styles.pinned,
        place.edge === "start"
            ? styles.pinnedStart(pixels, controls)
            : styles.pinnedEnd(pixels, controls),
    ];
}

/** Report whether two maps of pinned columns place every column alike. */
function isSamePins(
    previous: ReadonlyMap<string, PinPlace | undefined>,
    next: ReadonlyMap<string, PinPlace | undefined>,
): boolean {
    return (
        previous.size === next.size &&
        [...previous].every(([id, place]) => {
            const other = next.get(id);

            return (
                place?.edge === other?.edge &&
                place?.pixels === other?.pixels &&
                place?.controls === other?.controls &&
                next.has(id)
            );
        })
    );
}

/** Report whether two lists have the same values in the same order. */
function isSameList<Value>(previous: readonly Value[], next: readonly Value[]): boolean {
    return (
        previous.length === next.length && previous.every((value, index) => value === next[index])
    );
}

/** Report whether two lists of selected runs have the same edges. */
function isSameSpans(previous: readonly DataTableSpan[], next: readonly DataTableSpan[]): boolean {
    return (
        previous.length === next.length &&
        previous.every(
            (span, index) => span.left === next[index]?.left && span.right === next[index]?.right,
        )
    );
}

/** Report whether a column's place lies in one of a row's selected runs. */
function isInSpans(spans: readonly DataTableSpan[], column: number | undefined): boolean {
    return (
        column !== undefined && spans.some((span) => column >= span.left && column <= span.right)
    );
}

/** Read a value of a table's state, following the table as it changes. */
function follow<Value>(tracked: Accessor<unknown>, value: () => Value): Value {
    tracked();

    return value();
}

/** Render a row in place of the rows a virtual table leaves out, none at no height. */
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

/** The state of a data table a template reads: its sorting, grouping, selection and expansion. */
export type DataTableState = Partial<
    TableState_RowSorting &
        TableState_ColumnGrouping &
        TableState_RowSelection &
        TableState_RowExpanding
>;

/** A table's state, and the slices of it its rows read, each changing apart. */
interface DataTableSlices {
    /** The whole state. */
    readonly state: Accessor<DataTableState>;
    /** The selected rows. */
    readonly rowSelection: Accessor<DataTableState["rowSelection"]>;
    /** The expanded rows. */
    readonly expanded: Accessor<DataTableState["expanded"]>;
    /** Whether a sort or grouping sets the rows' order. */
    readonly isArranged: Accessor<boolean>;
    /** Whether each row opens a menu of its actions. */
    readonly hasRowMenu: boolean;
    /** Run a change to a column, then focus its header, else the header that takes its place. */
    readonly changeColumn: (column: string, run: () => void) => void;
}

/** Read the nearest data table's state in a header or cell template. */
export function useDataTableState(): Accessor<DataTableState> {
    return useDataTableSlices().state;
}

/** Read the nearest data table's state and its slices, refusing a template outside one. */
function useDataTableSlices(): DataTableSlices {
    const slices = useContext(DataTableContext);
    if (slices === null) {
        throw new TypeError("a data table template needs a data table around it");
    }

    return slices;
}

/** The properties of a sortable column's header. */
export interface DataTableColumnHeaderProperties<
    Features extends TableFeatures,
    Row extends RowData,
    Value,
> {
    /** The column the header sorts and shows by its label. */
    readonly column: Column<Features, Row, Value> & Column_RowSorting<Features, Row>;
}

/** Render a column's name as a sort button, its sort's place among several, and its menu. */
export function DataTableColumnHeader<Features extends TableFeatures, Row extends RowData, Value>(
    properties: DataTableColumnHeaderProperties<Features, Row, Value>,
): JSX.Element {
    // read the column's sort and its place among the sorted columns
    const tracked = useDataTableState();
    const locale = useLocale();
    const column = properties.column;
    const sorted = () => follow(tracked, () => column.getIsSorted());
    const priority = () =>
        (tracked().sorting ?? []).length > 1 ? column.getSortIndex() + 1 : undefined;
    const slices = useDataTableSlices();
    const handlers = handleColumnActions(column, tracked, slices);

    return (
        <span
            ref={handlers}
            data-slot="data-table-column-header"
            {...style.attrs(styles.columnHeader, isEnd(column) && styles.reversed)}
        >
            {/* The name, which sorts a sortable column */}
            <Show when={column.getCanSort()} fallback={labelOf(column)}>
                <Button
                    variant="ghost"
                    xstyle={[text.xs, state.unframed, styles.sort, isEnd(column) && styles.sortEnd]}
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
                    {labelOf(column)}
                    <Icon icon={sortIconOf(sorted())} xstyle={sorted() === false && styles.quiet} />
                    <Show when={priority()}>
                        {(place) => (
                            <span {...style.attrs(text.xs, styles.priority)}>{place()}</span>
                        )}
                    </Show>
                </Button>
            </Show>

            {/* The column's menu */}
            <Show
                when={COLUMN_ACTIONS.some((action) =>
                    follow(tracked, () => hasColumnAction(column, action)),
                )}
            >
                <DropdownMenu>
                    <DropdownMenuTrigger
                        render={(attributes) => (
                            <Button
                                {...attributes}
                                variant="ghost"
                                size="icon-sm"
                                tabindex={-1}
                                xstyle={styles.quiet}
                            />
                        )}
                        aria-label={locale.render(t`${labelOf(column)} options`)}
                        data-slot="data-table-column-menu"
                        isOpenedByArrows={false}
                    >
                        <Icon icon={caretDown} />
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="start">
                        <For each={COLUMN_ACTIONS}>
                            {(action) => <MenuAction action={action} />}
                        </For>
                    </DropdownMenuContent>
                </DropdownMenu>
            </Show>
        </span>
    );
}

/** The actions on a column, as its menu lists them. */
const COLUMN_ACTIONS: readonly Action[] = [
    sortAscending,
    sortDescending,
    clearSort,
    hideColumn,
    pinToStart,
    pinToEnd,
    unpin,
    moveColumnLeft,
    moveColumnRight,
];

/** Report whether a column takes an action now: it sorts, hides, pins or moves. */
function hasColumnAction<Features extends TableFeatures, Row extends RowData, Value>(
    column: Column<Features, Row, Value> & Column_RowSorting<Features, Row>,
    action: Action,
): boolean {
    // sort a sortable column, and clear a sorted one
    const columnObject: object = column;
    const table: object = column.table;
    const isMoving = isOrderedTable(table) && isMovable({ isPlaceholder: false, column });
    if (action === sortAscending || action === sortDescending) {
        return column.getCanSort();
    } else if (action === clearSort) {
        return column.getCanSort() && column.getIsSorted() !== false;
    }
    // hide a hideable column
    else if (action === hideColumn) {
        return isHideable(columnObject) && columnObject.getCanHide();
    }
    // pin an unpinned column, and unpin a pinned one
    else if (action === pinToStart || action === pinToEnd || action === unpin) {
        return (
            isPinning(columnObject) &&
            columnObject.getCanPin() &&
            (columnObject.getIsPinned() === false) === (action !== unpin)
        );
    }

    // move a movable column past a neighbour
    return (
        isMoving && neighbourOf(table, column.id, action === moveColumnLeft ? -1 : 1) !== undefined
    );
}

/** Run an action on a column it takes. */
function runColumnAction<Features extends TableFeatures, Row extends RowData, Value>(
    column: Column<Features, Row, Value> & Column_RowSorting<Features, Row>,
    action: Action,
): void {
    // sort the column, or clear its sort
    const columnObject: object = column;
    const table: object = column.table;
    if (action === sortAscending || action === sortDescending) {
        column.toggleSorting(action === sortDescending, false);
    } else if (action === clearSort) {
        column.clearSorting();
    }
    // hide the column
    else if (action === hideColumn && isHideable(columnObject)) {
        columnObject.toggleVisibility(false);
    }
    // pin the column to an edge, or unpin it
    else if (
        isPinning(columnObject) &&
        (action === pinToStart || action === pinToEnd || action === unpin)
    ) {
        columnObject.pin(action === unpin ? false : action === pinToStart ? "start" : "end");
    }
    // move the column past its neighbour
    else if (isOrderedTable(table)) {
        moveColumn(table, column.id, action === moveColumnLeft ? -1 : 1);
    }
}

/** List what handles the actions on a column, for its header's `ref`. */
function handleColumnActions<Features extends TableFeatures, Row extends RowData, Value>(
    column: Column<Features, Row, Value> & Column_RowSorting<Features, Row>,
    tracked: Accessor<unknown>,
    slices: DataTableSlices,
): ((element: Element) => void)[] {
    return COLUMN_ACTIONS.map((action) =>
        handle(action, {
            isEnabled: () => follow(tracked, () => hasColumnAction(column, action)),
            run: () => slices.changeColumn(column.id, () => runColumnAction(column, action)),
        }),
    );
}

/** List what handles the owner's actions on rows, for the `ref` of the element they are in. */
function handleRowActions(
    rowActions: readonly DataTableRowAction[],
    targetsAt: (element: Element | null) => readonly string[],
): ((element: Element) => void)[] {
    return rowActions.map(({ action, isEnabled, run }) =>
        handle(action, {
            isEnabled: (element) => {
                const targets = targetsAt(element);

                return targets.length > 0 && (isEnabled?.(targets) ?? true);
            },
            run: ({ element }) => run(targetsAt(element)),
        }),
    );
}

/** Find a column's nearest movable, shown neighbour in a step's direction, none at the end. */
function neighbourOf(table: OrderedTable, id: string, step: -1 | 1): string | undefined {
    // look along the row from the column, its way
    const columns = table.getAllLeafColumns();
    const index = columns.findIndex((column) => column.id === id);
    const side = step === 1 ? columns.slice(index + 1) : columns.slice(0, index).toReversed();

    return side.find(
        (column) =>
            (!isHideable(column) || column.getIsVisible()) &&
            isMovable({ isPlaceholder: false, column }),
    )?.id;
}

/** Move a column past its movable neighbour in a step's direction. */
function moveColumn(table: OrderedTable, id: string, step: -1 | 1): void {
    // place the column on the far side of its neighbour
    const neighbour = present(neighbourOf(table, id, step), `the neighbour of column '${id}'`);
    const order = table.getAllLeafColumns().map((column) => column.id);
    const rest = order.filter((each) => each !== id);
    const index = rest.indexOf(neighbour) + (step === 1 ? 1 : 0);
    table.setColumnOrder(rest.toSpliced(index, 0, id));
}

/** The properties of a data table's pagination. */
export interface DataTablePaginationProperties<
    Features extends TableFeatures,
    Row extends RowData,
> {
    /** The table the pagination pages through. */
    readonly table: Table_RowPagination<Features, Row> & {
        /** The table's state, read to track it. */
        readonly tracked: Accessor<TableState_RowPagination>;
    };
    /** The page sizes offered, ten to a hundred by default. */
    readonly pageSizes?: readonly number[] | undefined;
}

/** Render the selected row count, the page size, the page number and the steps between pages. */
export function DataTablePagination<Features extends TableFeatures, Row extends RowData>(
    properties: DataTablePaginationProperties<Features, Row>,
): JSX.Element {
    // read the page and the selection as the table changes
    const locale = useLocale();
    const table = properties.table;
    const page = (): PaginationState => table.tracked().pagination;
    const count = (): number => follow(table.tracked, () => table.getPageCount());

    // read the page size among the sizes offered, refusing a size the menu leaves out
    const sizes = (): readonly number[] => properties.pageSizes ?? PAGE_SIZES;
    const pageSize = (): string => {
        const current = page().pageSize;
        if (!sizes().includes(current)) {
            throw new TypeError(`data table page size '${String(current)}' is not offered`);
        }

        return String(current);
    };
    const selected = (): number | undefined =>
        follow(table.tracked, () =>
            isSelectableTable(table) ? table.getSelectedRowModel().rows.length : undefined,
        );
    const selectedLabel = (): string | undefined => {
        // count the selected rows of a selectable table, zero included
        const rows = selected();

        return rows === undefined
            ? undefined
            : locale.render(
                  t`${rows} of ${plural(total(), { one: "# row", other: "# rows" })} selected`,
              );
    };
    const total = (): number => follow(table.tracked, () => table.getRowCount());
    const canPrevious = (): boolean => follow(table.tracked, () => table.getCanPreviousPage());
    const canNext = (): boolean => follow(table.tracked, () => table.getCanNextPage());

    return (
        <div data-slot="data-table-pagination" {...style.attrs(text.xs, styles.pagination)}>
            <span {...style.attrs(styles.count)}>{selectedLabel()}</span>
            <label {...style.attrs(styles.size)}>
                {locale.render(t`Rows per page`)}
                <Select
                    value={pageSize()}
                    onValueChange={(chosen) => table.setPageSize(Number(chosen))}
                >
                    <For each={sizes()}>
                        {(each) => <SelectItem value={String(each)}>{each}</SelectItem>}
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
            size="icon"
            aria-label={properties.label}
            disabled={properties.disabled}
            onClick={() => properties.go()}
        >
            <Icon icon={properties.icon} />
        </Button>
    );
}

/** The properties of a data table's bulk bar. */
export interface DataTableBulkBarProperties<Features extends TableFeatures, Row extends RowData> {
    /** The table whose selected rows the bar acts on. */
    readonly table: Table_RowSelection<Features, Row> & {
        /** The table's state, read to track it. */
        readonly tracked: Accessor<unknown>;
    };
    /** What the owner does with the selected rows, read once. */
    readonly actions: readonly DataTableRowAction[];
}

/** Render the selected rows' count and actions while rows are selected. */
export function DataTableBulkBar<Features extends TableFeatures, Row extends RowData>(
    properties: DataTableBulkBarProperties<Features, Row>,
): JSX.Element {
    // read the selected rows as the table changes
    const locale = useLocale();
    const rows = () =>
        follow(properties.table.tracked, () =>
            properties.table.getSelectedRowModel().rows.map((row) => row.id),
        );
    const rowActions = untrack(() => properties.actions);
    const handlers = handleRowActions(rowActions, () => rows());

    return (
        <Show when={rows().length > 0}>
            <div
                ref={handlers}
                role="group"
                aria-label={locale.render(t`Selected rows`)}
                data-slot="data-table-bulk-bar"
                {...style.attrs(text.sm, styles.bulkBar)}
            >
                <span {...style.attrs(styles.count)}>
                    {locale.render(
                        t`${plural(rows().length, { one: "# row", other: "# rows" })} selected`,
                    )}
                </span>
                <For each={rowActions}>
                    {({ action }) => (
                        <ActionButton
                            action={action}
                            variant={action.isDestructive ? "destructive" : "outline"}
                            size="sm"
                        />
                    )}
                </For>
                <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => properties.table.resetRowSelection()}
                >
                    {locale.render(t`Clear selection`)}
                </Button>
            </div>
        </Show>
    );
}

/** The properties of a data table's column menu. */
export interface DataTableViewOptionsProperties<
    Features extends TableFeatures,
    Row extends RowData,
> {
    /** The table whose columns the menu shows and hides, each by its label. */
    readonly table: DataTableInstance<Features, Row>;
}

/** Render a menu that shows and hides a table's hideable columns. */
export function DataTableViewOptions<Features extends TableFeatures, Row extends RowData>(
    properties: DataTableViewOptionsProperties<Features, Row>,
): JSX.Element {
    const locale = useLocale();
    const columns = createMemo(() => {
        properties.table.tracked();

        return properties.table.getAllLeafColumns().filter(isHideable);
    });

    return (
        <DropdownMenu>
            <DropdownMenuTrigger
                render={(attributes) => <Button {...attributes} variant="outline" />}
                data-slot="data-table-view-options"
            >
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
                            {labelOf(column)}
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
    /** The column stays out of blocks of cells. */
    readonly enableCellSelection: false;
    /** The column only controls rows. */
    readonly meta: { readonly isControl: true };
} {
    return {
        id: "select",
        header: (context) => <PageSelection table={context.table} />,
        cell: (context) => <RowSelection row={context.row} />,
        enableSorting: false,
        enableHiding: false,
        enableCellSelection: false,
        meta: { isControl: true },
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
    /** The column stays out of blocks of cells. */
    readonly enableCellSelection: false;
    /** The column only controls rows. */
    readonly meta: { readonly isControl: true };
} {
    return {
        id: "expand",
        header: () => <ExpandHeader />,
        cell: (context) => <RowToggle row={context.row} />,
        enableSorting: false,
        enableHiding: false,
        enableCellSelection: false,
        meta: { isControl: true },
    };
}

/** Render the name of the column of expand buttons, for assistive technology alone. */
function ExpandHeader(): JSX.Element {
    const locale = useLocale();

    return <VisuallyHidden>{locale.render(t`Expand`)}</VisuallyHidden>;
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
    const { expanded } = useDataTableSlices();
    const row = properties.row;
    if (!isExpandableRow(row)) {
        return properties.children;
    }
    const isExpanded = (): boolean => {
        expanded();

        return row.getIsExpanded();
    };

    return (
        <span
            data-slot="data-table-toggle"
            {...style.attrs(styles.toggle, indent.by("0px", row.depth))}
        >
            <Show when={row.getCanExpand()} fallback={<span {...style.attrs(styles.selection)} />}>
                <Button
                    variant="ghost"
                    size="icon-sm"
                    aria-expanded={isExpanded() ? "true" : "false"}
                    xstyle={styles.expand}
                    aria-label={locale.render(isExpanded() ? t`Collapse row` : t`Expand row`)}
                    onClick={() => row.toggleExpanded()}
                >
                    <span {...style.attrs(indicator.base, isExpanded() && indicator.open)}>
                        <DisclosureCaret />
                    </span>
                </Button>
            </Show>
            {properties.children}
        </span>
    );
}

/** Render the box that selects every row of the page. */
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
            xstyle={[styles.box, styles.tableQuiet]}
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
    const { rowSelection } = useDataTableSlices();
    const row = properties.row;
    if (!isSelectableRow(row)) {
        return null;
    }
    const isSelected = (): boolean => {
        rowSelection();

        return row.getIsSelected();
    };

    return (
        <Checkbox
            aria-label={locale.render(t`Select row`)}
            checked={isSelected()}
            disabled={!row.getCanSelect()}
            onCheckedChange={(isChecked) => row.toggleSelected(isChecked)}
            xstyle={[styles.box, styles.rowControl]}
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

/** Render a cell. */
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

/** Read a column's name: its meta's label, else its header's text, refusing a column of neither. */
function labelOf<Features extends TableFeatures, Data extends RowData, Value>(
    column: Column<Features, Data, Value>,
): string {
    // take the meta's label, else the header's text, refusing a column of neither
    const header = column.columnDef.header;
    const label = metaOf(column).label ?? (typeof header === "string" ? header : undefined);
    if (label === undefined) {
        throw new TypeError(`data table column '${column.id}' is invalid: it has no label`);
    }

    return label;
}

/** Report whether a column sorts through the sorting feature. */
function isSortable(column: object): column is Column_RowSorting<TableFeatures, RowData> {
    return "getCanSort" in column && typeof column.getCanSort === "function";
}

/** Report whether a column hides through the visibility feature. */
function isHideable<Value extends object>(
    column: Value,
): column is Value &
    Column_ColumnVisibility &
    Pick<Column_CoreProperties<TableFeatures, RowData>, "id"> {
    return "getCanHide" in column && typeof column.getCanHide === "function";
}

/** A table that orders its columns. */
interface OrderedTable {
    /** Read every leaf column in the table's order. */
    readonly getAllLeafColumns: () => readonly (object & { readonly id: string })[];
    /** Set the order of the columns. */
    readonly setColumnOrder: (order: string[]) => void;
}

/** Report whether a table orders its columns. */
function isOrderedTable(table: object): table is OrderedTable {
    return "setColumnOrder" in table;
}

/** Report whether a table selects rows through the selection feature. */
function isSelectableTable(table: object): table is Table_RowSelection<TableFeatures, RowData> {
    return (
        "toggleAllPageRowsSelected" in table &&
        typeof table.toggleAllPageRowsSelected === "function"
    );
}

/** Report whether a row selects through the selection feature. */
function isSelectableRow(row: object): row is Row_RowSelection {
    return "getIsSelected" in row && typeof row.getIsSelected === "function";
}

/** Report whether a row expands through the expanding feature. */
function isExpandableRow(
    row: object,
): row is Row_RowExpanding & Pick<Row_CoreProperties<TableFeatures, RowData>, "depth" | "subRows"> {
    return "getCanExpand" in row && typeof row.getCanExpand === "function";
}

/** Report whether a header belongs to a table whose columns size. */
function isSizedHeader(header: object): header is Header_ColumnSizing {
    return "getSize" in header && typeof header.getSize === "function";
}

/** Report whether a column belongs to a table whose columns size. */
function isSizable(column: object): column is Column_ColumnSizing {
    return "resetSize" in column && typeof column.resetSize === "function";
}

/** Report whether a header belongs to a table whose columns a drag resizes. */
function isResizingHeader(header: object): header is Header_ColumnResizing {
    return "getResizeHandler" in header && typeof header.getResizeHandler === "function";
}

/** Report whether a column belongs to a table whose columns a drag resizes. */
function isResizable(column: object): column is Column_ColumnResizing {
    return "getCanResize" in column && typeof column.getCanResize === "function";
}

/** Report whether a column belongs to a table whose columns pin. */
function isPinning(column: object): column is Column_ColumnPinning {
    return "getIsPinned" in column && typeof column.getIsPinned === "function";
}

/** Report whether a column belongs to a table whose columns pin, with their offsets. */
function isPinnable(column: object): column is Column_ColumnPinning & Column_ColumnSizing {
    return (
        "getIsPinned" in column &&
        typeof column.getIsPinned === "function" &&
        "getStart" in column &&
        typeof column.getStart === "function"
    );
}

/** Report whether a cell belongs to a table that groups rows. */
function isGroupedCell(cell: object): cell is Cell_ColumnGrouping & Partial<Cell_RowAggregation> {
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

/** Report whether a header's column moves: a data column with an accessor, and not pinned. */
function isMovable(header: {
    readonly isPlaceholder: boolean;
    readonly column: object & { readonly id: string };
}): boolean {
    const column = header.column;
    const isData = "accessorFn" in column && column.accessorFn !== undefined;

    return (
        !header.isPlaceholder && isData && !(isPinning(column) && column.getIsPinned() !== false)
    );
}

/** Render a header row: a sortable row where its columns move, else a plain one. */
function HeaderRow(properties: {
    /** Whether the row's columns move along it. */
    readonly isOrdered: boolean;
    /** The row's place among every row of the grid, from one. */
    readonly "aria-rowindex": number;
    /** The header cells. */
    readonly children?: JSX.Element;
}): JSX.Element {
    const locale = useLocale();

    return untrack(() => properties.isOrdered) ? (
        <SortableContainer
            id={COLUMNS}
            orientation="horizontal"
            label={locale.render(t`Columns`)}
            accepts={[COLUMN_TYPE]}
            render={(attributes) => (
                <TableRow
                    {...attributes}
                    xstyle={styles.row}
                    aria-rowindex={properties["aria-rowindex"]}
                />
            )}
        >
            {properties.children}
        </SortableContainer>
    ) : (
        <TableRow xstyle={styles.row} aria-rowindex={properties["aria-rowindex"]}>
            {properties.children}
        </TableRow>
    );
}

/** Render the body: a sortable list where rows move by their handles, else a plain body. */
function BodyRows(properties: {
    /** Whether the rows move. */
    readonly isOrdered: boolean;
    /** The rows. */
    readonly children?: JSX.Element;
}): JSX.Element {
    const locale = useLocale();

    return untrack(() => properties.isOrdered) ? (
        <SortableContainer
            id={ROWS}
            label={locale.render(t`Rows`)}
            accepts={[ROW_TYPE]}
            xstyle={styles.unmarked}
            render={(attributes) => <TableBody {...attributes} />}
        >
            {properties.children}
        </SortableContainer>
    ) : (
        <TableBody>{properties.children}</TableBody>
    );
}

/** Place a moved column in a table's order, before the column it lands before. */
function placeColumn(table: OrderedTable, move: SortableMove): void {
    const order = table.getAllLeafColumns().map((column) => column.id);
    const place = {
        id: idOf(move.id),
        previous: move.previous === undefined ? undefined : idOf(move.previous),
        next: move.next === undefined ? undefined : idOf(move.next),
    };
    table.setColumnOrder(SortableMove.place(order, place.id, place, (id) => id));
}

/** Read an item's id without the prefix that sets rows and columns apart among sortable items. */
function idOf(item: string): string {
    // strip a row's prefix
    if (item.startsWith(ROW_PREFIX)) {
        return item.slice(ROW_PREFIX.length);
    }
    // strip a column's prefix
    else if (item.startsWith(COLUMN_PREFIX)) {
        return item.slice(COLUMN_PREFIX.length);
    }

    throw new TypeError(`data table sortable item '${item}' is neither a row nor a column`);
}

/** Report whether a table's sort or grouping sets its rows' order, which rows then follow. */
function isArranged(table: DataTableState): boolean {
    return (table.sorting ?? []).length > 0 || (table.grouping ?? []).length > 0;
}

/** Render a row's handle, which lists the key that opens the row's menu where rows have one. */
function RowHandle(): JSX.Element {
    const slices = useDataTableSlices();

    return (
        <SortableHandle
            data-slot="data-table-row-handle"
            aria-haspopup={slices.hasRowMenu ? "menu" : undefined}
            aria-keyshortcuts={slices.hasRowMenu ? "Shift+F10" : undefined}
            xstyle={styles.rowControl}
        />
    );
}

/** The column that moves rows: a handle per row, which lifts it by pointer, touch and Space. */
export function orderColumn(): {
    /** The column's id. */
    readonly id: "order";
    /** The column's header, empty. */
    readonly header: () => JSX.Element;
    /** The handle that moves the row. */
    readonly cell: () => JSX.Element;
    /** The column neither sorts nor hides. */
    readonly enableSorting: false;
    /** The column neither sorts nor hides. */
    readonly enableHiding: false;
    /** The column stays out of blocks of cells. */
    readonly enableCellSelection: false;
    /** The column only controls rows. */
    readonly meta: { readonly isControl: true };
} {
    return {
        id: "order",
        header: () => null,
        cell: () => <RowHandle />,
        enableSorting: false,
        enableHiding: false,
        enableCellSelection: false,
        meta: { isControl: true },
    };
}

/** Write a column's summary of its values in a locale, a date column's earliest or latest. */
function summaryText(
    summary: DataTableSummary,
    kind: DataTableEditorKind | undefined,
    values: readonly unknown[],
    locale: ReturnType<typeof useLocale>,
): string {
    // count the filled values, an unchecked box as empty, and read the numbers and dates among them
    const filled = values.filter(
        (value) => value !== undefined && value !== null && value !== "" && value !== false,
    );
    const numbers = filled.filter((value) => typeof value === "number");
    const dates = filled
        .filter((value) => kind === "date" && typeof value === "string")
        .map(String)
        .toSorted((left, right) => left.localeCompare(right));

    // count the filled cells
    if (summary === "count") {
        return locale.render(t`Count ${locale.number(filled.length)}`);
    }
    // take the earliest or latest of dates
    else if (numbers.length === 0 && dates.length > 0 && (summary === "min" || summary === "max")) {
        const date = summary === "min" ? dates[0] : dates.at(-1);

        return locale.render(
            summary === "min" ? t`Earliest ${String(date)}` : t`Latest ${String(date)}`,
        );
    }
    // say a column has no numbers to sum
    else if (numbers.length === 0) {
        return locale.render(t`No numbers`);
    }
    // sum, average, or take the smallest or largest number
    else if (summary === "sum") {
        return locale.render(
            t`Sum ${locale.number(numbers.reduce((total, each) => total + each, 0))}`,
        );
    } else if (summary === "average") {
        const average = numbers.reduce((total, each) => total + each, 0) / numbers.length;

        return locale.render(t`Average ${locale.number(average, { maximumFractionDigits: 2 })}`);
    } else if (summary === "min") {
        return locale.render(t`Min ${locale.number(Math.min(...numbers))}`);
    } else {
        return locale.render(t`Max ${locale.number(Math.max(...numbers))}`);
    }
}

/** Read how many rows a sub row lies under, none for a group's. */
function depthOf(row: object): number {
    return isExpandableRow(row) && !isGroupRow(row) ? row.depth : 0;
}

/** Report whether a column's values stand at its end, as numbers do. */
function isEnd<Features extends TableFeatures, Data extends RowData, Value>(
    column: Column<Features, Data, Value>,
): boolean {
    const meta = metaOf(column);

    return (meta.align ?? (meta.editor?.kind === "number" ? "end" : "start")) === "end";
}

/** Take a key on a data table's grid: toggle, edit, clear, open a menu, or move and extend. */
function keyDown<Features extends TableFeatures, Data extends RowData>(
    event: KeyboardEvent,
    control: DataTableControl<Features, Data>,
    openRowMenu: (id: string, cell: HTMLElement, point: { x: number; y: number }) => boolean,
    direction: Direction,
): void {
    // leave keys from outside the cells, a field's keys, and every key during an edit
    const target = event.target;
    const key = control.focus.current();
    const element = target instanceof Element ? target.closest<HTMLElement>("th, td") : null;
    const isField =
        target instanceof Element && target.matches("input:not([type=checkbox]), textarea, select");
    const isBoxEnter =
        event.key === "Enter" && target instanceof Element && target.matches("[role=checkbox]");
    if (
        (event.defaultPrevented && !isBoxEnter) ||
        element === null ||
        isField ||
        control.edit() !== undefined ||
        key === undefined
    ) {
        return;
    }
    const cell = control.cellAt(key);
    const editor = cell === undefined ? undefined : control.editorOf(cell);
    const isPlain = !event.ctrlKey && !event.metaKey && !event.altKey;
    const arrow = ARROWS[event.key];

    // open the column's menu with Alt and the down arrow, as grids do
    if (event.altKey && event.key === "ArrowDown" && cell === undefined) {
        const menu = element.querySelector<HTMLElement>("[data-slot=data-table-column-menu]");
        if (menu !== null) {
            event.preventDefault();
            menu.click();
        }
    }
    // open the row's menu with the menu key or Shift and F10, under the cell
    else if (
        cell !== undefined &&
        (event.key === "ContextMenu" || (event.shiftKey && event.key === "F10"))
    ) {
        const bounds = element.getBoundingClientRect();
        if (openRowMenu(cell.row.id, element, { x: bounds.left, y: bounds.bottom })) {
            event.preventDefault();
        }
    }
    // toggle a checkbox cell with Enter, as Space does
    else if (cell !== undefined && editor?.kind === "checkbox" && event.key === "Enter") {
        event.preventDefault();
        control.set(cell, cell.getValue() !== true);
    }
    // edit a cell with Enter or F2, as spreadsheets do
    else if (
        cell !== undefined &&
        editor !== undefined &&
        editor.kind !== "checkbox" &&
        (event.key === "Enter" || event.key === "F2")
    ) {
        event.preventDefault();
        control.begin(cell);
    }
    // edit a text or number cell by typing over it
    else if (
        cell !== undefined &&
        (editor?.kind === "text" || editor?.kind === "number") &&
        isPlain &&
        event.key.length === 1 &&
        event.key !== " "
    ) {
        event.preventDefault();
        control.begin(cell, event.key);
    }
    // clear the selected block, else the cell, with Delete or Backspace
    else if (cell !== undefined && (event.key === "Delete" || event.key === "Backspace")) {
        event.preventDefault();
        control.clear();
    }
    // extend the selected block with Shift and an arrow, the focus staying where it began
    else if (cell !== undefined && control.isSelecting() && event.shiftKey && arrow !== undefined) {
        event.preventDefault();
        control.extend(cell, arrow(direction));
    }
    // drop the block back to the focused cell with Escape
    else if (cell !== undefined && control.isSelecting() && event.key === "Escape") {
        control.select(cell);
    }
    // move the focus, selecting the body cell it lands on
    else {
        const moved = control.focus.move(event, direction);
        const landed = moved === undefined ? undefined : control.cellAt(moved);
        if (landed !== undefined) {
            control.select(landed);
        }
    }
}

/** Render the field a cell is edited in: text, a number, a date, or a choice. */
function DataTableCellEditor<Features extends TableFeatures, Data extends RowData>(properties: {
    /** The grid whose edit the field is. */
    readonly control: DataTableControl<Features, Data>;
    /** The column's editor. */
    readonly editor: DataTableEditor;
    /** The cell's value. */
    readonly value: unknown;
    /** The column's name, which labels the field. */
    readonly label: string;
}): JSX.Element {
    // begin from typed text, else from the value, the caret at its end once the field shows
    const locale = useLocale();
    const control = properties.control;
    const editor = untrack(() => properties.editor);
    const initial = untrack(() => control.edit()?.initial);
    let field: HTMLInputElement | HTMLSelectElement | undefined;
    onSettled(() =>
        untrack(() => {
            // focus the field with the caret at the end
            field?.focus();
            if (field instanceof HTMLInputElement && field.type === "text") {
                field.setSelectionRange(field.value.length, field.value.length);
            }
        }),
    );

    // follow the person's choice in a select until Enter or the focus leaving commits it
    const [choice, setChoice] = createSignal(
        untrack(() => (typeof properties.value === "string" ? properties.value : "")),
    );

    // read the field's text: a select's choice, none for an input the browser cannot read
    const textOf = (element: HTMLInputElement | HTMLSelectElement): string | undefined =>
        element instanceof HTMLSelectElement
            ? choice()
            : element.validity.badInput
              ? undefined
              : element.value;

    // end an edit whose row leaves the table with the text it has
    onCleanup(() =>
        untrack(() => {
            if (field !== undefined && control.edit() !== undefined) {
                control.leave(textOf(field));
            }
        }),
    );

    // commit on Enter and move down, on Tab and move across, and cancel on Escape
    const onKeyDown = (
        event: KeyboardEvent & { readonly currentTarget: HTMLInputElement | HTMLSelectElement },
    ) => {
        if (event.key === "Enter") {
            event.preventDefault();
            control.commit(textOf(event.currentTarget), "down");
        } else if (event.key === "Tab") {
            event.preventDefault();
            control.commit(textOf(event.currentTarget), event.shiftKey ? "previous" : "next");
        } else if (event.key === "Escape") {
            event.preventDefault();
            control.cancel();
        }
    };

    // end the edit where the focus leaves it with a value, else mark the text and go on editing
    const onBlur = (
        event: FocusEvent & { readonly currentTarget: HTMLInputElement | HTMLSelectElement },
    ) =>
        untrack(() =>
            control.edit() === undefined ? undefined : control.leave(textOf(event.currentTarget)),
        );
    const isInvalid = () => (control.edit()?.isInvalid === true ? "true" : undefined);

    return (
        <Show
            when={editor.kind === "select"}
            fallback={
                <Input
                    ref={(element) => {
                        // set the text once, the field changing it until the edit ends
                        field = element;
                        element.value =
                            initial ??
                            editor.format(
                                untrack(() => properties.value),
                                locale,
                            );
                    }}
                    type={editor.kind === "date" ? "date" : "text"}
                    inputmode={editor.kind === "number" ? "decimal" : undefined}
                    aria-label={properties.label}
                    aria-invalid={isInvalid()}
                    onKeyDown={onKeyDown}
                    onBlur={onBlur}
                    xstyle={styles.editor}
                />
            }
        >
            <Select
                ref={(element) => (field = element)}
                aria-label={properties.label}
                aria-invalid={isInvalid()}
                value={choice()}
                onValueChange={(value) => setChoice(typeof value === "string" ? value : "")}
                onKeyDown={onKeyDown}
                onBlur={onBlur}
            >
                <SelectItem value="">{locale.render(t`None`)}</SelectItem>
                <For each={editor.options}>
                    {(option) => <SelectItem value={option.value}>{option.label}</SelectItem>}
                </For>
            </Select>
        </Show>
    );
}

/** The reference and handlers that make a header or body cell one place of a data table's grid. */
interface GridCellAttributes extends Record<"ref", (element: HTMLTableCellElement) => void> {
    /** The text of the cell's key, which focusing the cell by its key finds. */
    readonly "data-cell": string;
    /** Follow the focus into the cell, selecting a body cell outside the selected block. */
    readonly onFocusIn: () => void;
    /** Start a block of cells at a body cell, or extend it there with Shift. */
    readonly onMouseDown: (event: MouseEvent) => void;
    /** Extend the block over the body cells a pressed pointer sweeps. */
    readonly onMouseEnter: (event: MouseEvent) => void;
}

/** Make a cell one place of its data table's grid, a body cell a place of its cell selection. */
function useGridCell<Features extends TableFeatures, Data extends RowData>(
    control: DataTableControl<Features, Data>,
    key: () => CellKey,
    cell: () => object | undefined,
): GridCellAttributes {
    // read the grid's focus and take the cell's element
    const focus = control.focus;
    let element: HTMLTableCellElement | undefined;

    // focus the cell's single control, else the cell, unless the focus already rests inside it
    focus.bind(key, () =>
        element === undefined || element.contains(document.activeElement)
            ? undefined
            : focusOf(element),
    );

    // put the focused cell's single control, else the cell, alone in the tab order, as its content renders
    createEffect(
        () => {
            cell();

            return focus.isActive(key());
        },
        (isActive) => {
            if (element === undefined) {
                return;
            }
            const target = focusOf(element);
            element.tabIndex = isActive && target === element ? 0 : -1;
            for (const inner of element.querySelectorAll<HTMLElement>(FOCUSABLE)) {
                inner.tabIndex = isActive && target === inner ? 0 : -1;
            }
        },
    );

    return {
        ref: (created) => {
            element = created;
        },
        get "data-cell"() {
            return keyText(key());
        },
        onFocusIn: () => {
            // hand the focus back to an open edit's field, which only Escape or a value ends
            const edit = control.edit();
            if (edit !== undefined && !control.isEditing(key())) {
                control.focusEdit(edit);

                return;
            }

            // follow the focus in, selecting a body cell the selected block leaves out
            focus.focusIn(key());
            const each = cell();
            const body = control.cellAt(key());
            if (
                each !== undefined &&
                body !== undefined &&
                isSelectableCell(each) &&
                !each.getIsSelected()
            ) {
                control.select(body);
            }
        },
        onMouseDown: (event) => {
            // start or extend the block from the cell, leaving presses on the cell's controls
            const each = cell();
            const isControl =
                event.target instanceof Element && event.target.closest(PRESSED) !== null;
            if (each !== undefined && isSelectableCell(each) && !isControl && event.button === 0) {
                each.getSelectionStartHandler(element?.ownerDocument)(event);
            }
        },
        onMouseEnter: (event) => {
            // extend the block over a cell the pressed pointer reaches
            const each = cell();
            if (each !== undefined && isSelectableCell(each)) {
                each.getSelectionExtendHandler()(event);
            }
        },
    };
}
