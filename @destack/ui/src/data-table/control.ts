import type { Direction } from "@destack/locale";
import { present } from "@destack/schema";
import {
    type Accessor,
    createEffect,
    createMemo,
    createSignal,
    flush,
    untrack,
} from "@destack/view";
import type {
    Cell,
    Cell_CellSelection,
    Cell_ColumnGrouping,
    CellData,
    Column,
    CellSelectionBounds,
    CellSelectionDirection,
    Row,
    Row_ColumnGrouping,
    Row_Core,
    RowData,
    Table_CellSelection,
    TableFeatures,
    TableState_CellSelection,
} from "@tanstack/table-core";
import {
    Focus,
    type GridMove,
    gridMoveOf,
    type GridPlace,
    type KeyboardDelegate,
} from "../focus/index.ts";
import type { SortableMove } from "../sortable/index.ts";
import type { DataTableInstance } from "./table.ts";

/** Whether each editor's field shows any text, which a refused paste opens with its text. */
const SHOWS_TEXT: Readonly<Record<DataTableEditorKind, boolean>> = {
    text: true,
    number: true,
    date: false,
    select: false,
    checkbox: false,
};

/** A summary a column's footer shows of its values, after Airtable's summary bar. */
export type DataTableSummary = "count" | "sum" | "average" | "min" | "max";

/** The kinds of values a column edits in place, after Airtable's field types. */
export type DataTableEditorKind = "text" | "number" | "date" | "select" | "checkbox";

/** A value read from text, or the mark of text that is no value of the column's kind. */
export type DataTableValue =
    | { readonly kind: "valid"; readonly value: unknown }
    | { readonly kind: "invalid" };

/** A choice of a select column. */
export interface DataTableOption {
    /** The value a cell takes. */
    readonly value: string;
    /** The choice's name. */
    readonly label: string;
}

/** How a column shows and edits its values: its kind, and how its values read from text. */
export interface DataTableEditor {
    /** The kind of value. */
    readonly kind: DataTableEditorKind;
    /** The choices of a select column, none for the other kinds. */
    readonly options: readonly DataTableOption[];
    /** Read a value from text, as typed or pasted. */
    readonly parse: (text: string) => DataTableValue;
    /** Write a value as text, as copied or edited. */
    readonly format: (value: unknown) => string;
}

/** The editors of a data table's columns. */
export const DataTableEditor = {
    /** Edit text. */
    text(): DataTableEditor {
        return {
            kind: "text",
            options: [],
            parse: (text) => ({ kind: "valid", value: text }),
            format: textOf,
        };
    },

    /** Edit a number, empty text as null, refusing text that is no number. */
    number(): DataTableEditor {
        return {
            kind: "number",
            options: [],
            parse: (text) => {
                const value = Number(text.trim());

                return text.trim() === ""
                    ? { kind: "valid", value: null }
                    : Number.isNaN(value)
                      ? { kind: "invalid" }
                      : { kind: "valid", value };
            },
            format: textOf,
        };
    },

    /** Edit a date as YYYY-MM-DD, empty text as null, refusing a day the calendar lacks. */
    date(): DataTableEditor {
        return {
            kind: "date",
            options: [],
            parse: (text) => {
                // read empty text as no date
                const date = text.trim();
                if (date === "") {
                    return { kind: "valid", value: null };
                }

                // read the date back from its day in UTC, which a day past its month's end fails
                const day = new Date(`${date}T00:00:00Z`);
                const isDay =
                    DATE.test(date) &&
                    !Number.isNaN(day.getTime()) &&
                    day.toISOString().startsWith(date);

                return isDay ? { kind: "valid", value: date } : { kind: "invalid" };
            },
            format: textOf,
        };
    },

    /** Edit a choice among options, read from its value or its name, empty text as null. */
    select(options: readonly DataTableOption[]): DataTableEditor {
        return {
            kind: "select",
            options,
            parse: (text) => {
                const option = options.find(
                    (each) => each.value === text.trim() || each.label === text.trim(),
                );

                return text.trim() === ""
                    ? { kind: "valid", value: null }
                    : option === undefined
                      ? { kind: "invalid" }
                      : { kind: "valid", value: option.value };
            },
            format: (value) => options.find((each) => each.value === value)?.label ?? textOf(value),
        };
    },

    /** Edit a yes or no, toggled by a click, read from true, yes, 1, false, no, 0 or nothing. */
    checkbox(): DataTableEditor {
        return {
            kind: "checkbox",
            options: [],
            parse: (text) => {
                const word = text.trim().toLowerCase();

                return ["true", "yes", "1"].includes(word)
                    ? { kind: "valid", value: true }
                    : ["false", "no", "0", ""].includes(word)
                      ? { kind: "valid", value: false }
                      : { kind: "invalid" };
            },
            format: (value) => (value === true ? "true" : "false"),
        };
    },
};

/** What a column declares beside its definition: its name, editor and summary. */
export interface DataTableColumnMeta {
    /** The column's name, which menus, editors and drags read, its header's text by default. */
    readonly label?: string;
    /** Whether the column only selects, expands or moves rows, as narrow as its control. */
    readonly isControl?: boolean;
    /** How the column's cells show and edit their values, none for plain values. */
    readonly editor?: DataTableEditor;
    /** The summary its footer shows, none for no summary. */
    readonly summary?: DataTableSummary;
}

/** Give every column's meta the data table's fields, after TanStack's typed meta. */
declare module "@tanstack/table-core" {
    interface ColumnMeta<
        in out TFeatures extends TableFeatures,
        in out TData extends RowData,
        TValue extends CellData = CellData,
    > extends DataTableColumnMeta {}
}

/** A cell's new value, which an edit or a paste hands the owner. */
export interface DataTableCellChange {
    /** The row's id. */
    readonly row: string;
    /** The column's id. */
    readonly column: string;
    /** The value. */
    readonly value: unknown;
}

/** A row moving between two rows, which the owner applies to its rows' order. */
export type DataTableRowMove = Pick<SortableMove, "id" | "previous" | "next">;

/** Where the focus goes once an edit commits: down, to the next or previous cell, or back. */
export type DataTableStep = "down" | "next" | "previous" | "stay";

/** The rows a page jump moves by with Page Up and Page Down. */
const PAGE_ROWS = 10;

/** The pattern a written date follows. */
const DATE = /^\d{4}-\d{2}-\d{2}$/u;

/** The controls of a header that stay out of the grid's tab order: its menu. */
const SKIPPED = "[data-slot=data-table-column-menu]";

/** The elements inside a cell that take the focus in its place. */
export const FOCUSABLE = `a[href], button:not(${SKIPPED}), input, select, textarea, [tabindex]:not(td, th, ${SKIPPED})`;

/** The key of a cell of a data table's grid: a header's by its place, a body cell's by its ids. */
export type CellKey =
    | { readonly kind: "head"; readonly row: number; readonly column: number }
    | { readonly kind: "body"; readonly row: string; readonly column: string };

/** A paste a cell refused: the cell and the text it could not read. */
export interface DataTableRefusal {
    /** The id of the cell's row. */
    readonly row: string;
    /** The id of the cell's column. */
    readonly column: string;
    /** The pasted text. */
    readonly text: string;
}

/** The cell being edited: its row and column, its first text, and whether its text fails. */
export interface DataTableEdit {
    /** The row's id. */
    readonly row: string;
    /** The column's id. */
    readonly column: string;
    /** The text the edit began with, none to begin from the cell's value. */
    readonly initial: string | undefined;
    /** Whether the last text committed is no value of the column's kind. */
    readonly isInvalid: boolean;
}

/** A run of selected columns in a row, by their places among the shown columns. */
export interface DataTableSpan {
    /** The first column. */
    readonly left: number;
    /** The last column. */
    readonly right: number;
}

/** What a data table's grid reads of its table and its element. */
export interface DataTableControlOptions<Features extends TableFeatures, Data extends RowData> {
    /** The table. */
    readonly table: DataTableInstance<Features, Data>;
    /** The body rows shown, the current page's in order. */
    readonly rows: Accessor<readonly Row<Features, Data>[]>;
    /** The ids of the shown columns in order, the last header row's. */
    readonly columns: Accessor<readonly string[]>;
    /** The columns each cell of each header row spans. */
    readonly headers: Accessor<readonly (readonly number[])[]>;
    /** The element around the grid, which focusing a cell by its key searches. */
    readonly root: Accessor<HTMLElement | undefined>;
    /** Bring a body row into view by its place, as a virtual table scrolls to it. */
    readonly reveal: (row: number) => void;
    /** Hand the owner changed cells, in one call per edit or paste, none for a read-only table. */
    readonly onCellsChange: ((changes: readonly DataTableCellChange[]) => void) | undefined;
}

/** The grid of a data table: its focused cell, its edit, its clipboard and its cell selection. */
export class DataTableControl<
    Features extends TableFeatures,
    Data extends RowData,
> implements KeyboardDelegate<CellKey> {
    /** The focused cell, the grid's one tab stop. */
    readonly focus: Focus<CellKey>;
    /** The cell being edited, none between edits. */
    readonly edit: Accessor<DataTableEdit | undefined>;
    /** The paste the focused cell refused, shown while that cell is shown and has the focus. */
    readonly refusal: Accessor<DataTableRefusal | undefined>;
    /** Replace the refused paste. */
    readonly #setRefusal: (refusal: DataTableRefusal | undefined) => void;
    /** What the grid reads. */
    readonly #options: DataTableControlOptions<Features, Data>;
    /** Set the cell being edited. */
    readonly #setEdit: (edit: DataTableEdit | undefined) => void;
    /** Each shown row's place among the body rows, by id. */
    readonly #rowPlaces: Accessor<ReadonlyMap<string, number>>;
    /** Each shown column's place, by id. */
    readonly #columnPlaces: Accessor<ReadonlyMap<string, number>>;
    /** The selected blocks by places among every page's rows and the shown columns. */
    readonly #bounds: Accessor<readonly CellSelectionBounds[]>;

    /** Lay out a table's headers and rows as a grid, and follow its edit and cell selection. */
    constructor(options: DataTableControlOptions<Features, Data>) {
        // read the places of the rows and columns, and the selected blocks once per change
        this.#options = options;
        this.#rowPlaces = createMemo(
            () => new Map(options.rows().map((row, index) => [row.id, index] as const)),
        );
        this.#columnPlaces = createMemo(
            () => new Map(options.columns().map((id, index) => [id, index] as const)),
        );
        this.#bounds = createMemo(
            () => {
                options.table.tracked();
                const table: object = options.table;

                return isSelectingTable(table) ? table.getCellSelectionBounds() : [];
            },
            { equals: isSameBounds },
        );

        // start without an edit, the focus moving through the grid's places
        [this.edit, this.#setEdit] = createSignal<DataTableEdit | undefined>(undefined, {
            ownedWrite: true,
        });
        this.focus = new Focus<CellKey>({
            delegate: this,
            mode: "roving",
            keyText,
            reveal: (key) => {
                const row = key.kind === "body" ? this.#rowPlaces().get(key.row) : undefined;
                if (row !== undefined) {
                    options.reveal(row);
                }
            },
        });

        // show a refused paste while its cell is shown and has the focus
        const [refused, setRefused] = createSignal<DataTableRefusal | undefined>(undefined, {
            ownedWrite: true,
        });
        this.#setRefusal = setRefused;
        this.refusal = createMemo(() => {
            // read the refused cell against the focused one
            const each = refused();
            const focused = this.focus.current();
            const isOnCell =
                each !== undefined &&
                this.focus.isFocused() &&
                focused?.kind === "body" &&
                focused.row === each.row &&
                focused.column === each.column;

            return isOnCell && this.cellAt(focused) !== undefined ? each : undefined;
        });

        // end an edit whose cell stops showing, as its column hides or its row goes
        createEffect(
            () => {
                const edit = this.edit();

                return (
                    edit !== undefined &&
                    this.cellAt({ kind: "body", row: edit.row, column: edit.column }) === undefined
                );
            },
            (isGone) => {
                if (isGone) {
                    this.#setEdit(undefined);
                }
            },
        );
    }

    /** Read the cell a key press moves the focus to. */
    target(event: KeyboardEvent, from: CellKey, direction: Direction): CellKey | undefined {
        // read the move and the place it starts from
        const move = gridMoveOf(event, direction);
        const place = this.placeOf(from);
        if (move === undefined || place === undefined) {
            return undefined;
        }

        // land on the cell the move reaches, the row's last for a column past its end
        const landing = landingOf(move, place, this.#count() - 1);
        const width = this.#width(landing.row);
        if (width === undefined || (landing.column !== "last" && landing.column < 0)) {
            return undefined;
        }

        // step up or down by the leaf column the cell starts at, which header cells span
        const isVertical = ["up", "down", "pageUp", "pageDown"].includes(move);
        const across =
            isVertical && landing.column !== "last"
                ? this.#columnIn(landing.row, this.#offsetOf(place))
                : landing.column;
        const column = across === "last" ? width - 1 : Math.min(across, width - 1);

        return this.keyAt({ row: landing.row, column });
    }

    /** Read the first cell, the first header's. */
    first(): CellKey | undefined {
        return this.keyAt({ row: 0, column: 0 });
    }

    /** Report whether a cell stands in the grid. */
    has(key: CellKey): boolean {
        return this.placeOf(key) !== undefined;
    }

    /** Read a cell's place in the grid, the header rows first, none for a cell not shown. */
    placeOf(key: CellKey): GridPlace | undefined {
        // read a header's place as its key has it
        const headers = this.#options.headers();
        if (key.kind === "head") {
            const width = headers[key.row]?.length;

            return width !== undefined && key.column < width
                ? { row: key.row, column: key.column }
                : undefined;
        }

        // else read a body cell's place from its row's and column's
        const row = this.#rowPlaces().get(key.row);
        const column = this.#columnPlaces().get(key.column);

        return row === undefined || column === undefined
            ? undefined
            : { row: headers.length + row, column };
    }

    /** Read the key of the cell at a place in the grid, none past its edges. */
    keyAt(place: GridPlace): CellKey | undefined {
        // read a header's key by its place
        const headers = this.#options.headers();
        const width = headers[place.row]?.length;
        if (width !== undefined) {
            return place.column < width
                ? { kind: "head", row: place.row, column: place.column }
                : undefined;
        }

        // else read a body cell's key by its row's and column's ids
        const row = this.#options.rows()[place.row - headers.length];
        const column = this.#options.columns()[place.column];

        return row === undefined || column === undefined
            ? undefined
            : { kind: "body", row: row.id, column };
    }

    /** List a row's shown cells in the order of the shown columns. */
    cellsOf(row: Row_Core<Features, Data>): Cell<Features, Data>[] {
        const cells = row.getAllCellsByColumnId();

        return this.#options
            .columns()
            .map((id) => present(cells[id], `the cell of row '${row.id}' in column '${id}'`));
    }

    /** Read the body cell of a key, none for a header's or a cell not shown. */
    cellAt(key: CellKey): Cell<Features, Data> | undefined {
        // find the key's row and column among those shown
        const index = key.kind === "body" ? this.#rowPlaces().get(key.row) : undefined;
        const row = index === undefined ? undefined : this.#options.rows()[index];
        const isShown = key.kind === "body" && this.#columnPlaces().has(key.column);

        return isShown ? row?.getAllCellsByColumnId()[key.column] : undefined;
    }

    /** Read a body cell's editor, none for a read-only table, column, group row or summary. */
    editorOf(cell: Cell<Features, Data>): DataTableEditor | undefined {
        const isGroup = isGroupRow(cell.row) || isGroupCell(cell);

        return this.#options.onCellsChange === undefined || isGroup
            ? undefined
            : metaOf(cell.column).editor;
    }

    /** Report whether the table selects blocks of cells. */
    isSelecting(): boolean {
        const table: object = this.#options.table;

        return isSelectingTable(table);
    }

    /** List the runs of a row's columns the selected blocks take, none for a row outside them. */
    spansOf(row: Row_Core<Features, Data>): readonly DataTableSpan[] {
        const index = row.getDisplayIndex();

        return this.#bounds()
            .filter((bounds) => index >= bounds.minRowIndex && index <= bounds.maxRowIndex)
            .map((bounds) => ({ left: bounds.minColumnIndex, right: bounds.maxColumnIndex }));
    }

    /** Read a column's place among the shown columns, none for a column not shown. */
    columnPlaceOf(column: string): number | undefined {
        return this.#columnPlaces().get(column);
    }

    /** Select a body cell alone, which a block extends from, leaving cells that do not select. */
    select(cell: Cell<Features, Data>): void {
        const table: object = this.#options.table;
        if (isSelectingTable(table) && isSelectableCell(cell) && cell.getCanSelect()) {
            table.setFocusedCell(cell.row.id, cell.column.id);
        }
    }

    /** Extend the selected block a cell in a direction, starting it at a cell where none is. */
    extend(cell: Cell<Features, Data>, direction: CellSelectionDirection): void {
        // start a block at a cell that selects where none is selected
        const table: object = this.#options.table;
        if (!isSelectingTable(table) || !isSelectableCell(cell) || !cell.getCanSelect()) {
            return;
        }
        if (this.#bounds().length === 0) {
            table.setFocusedCell(cell.row.id, cell.column.id);
        }

        // extend it, and bring the row it reaches into view
        table.extendCellSelection(direction);
        const reached = table.store.state.cellSelection.at(-1)?.focusRowId;
        const row = reached === undefined ? undefined : this.#rowPlaces().get(reached);
        if (row !== undefined) {
            this.#options.reveal(row);
        }
    }

    /** Report whether a cell is the one being edited. */
    isEditing(key: CellKey): boolean {
        const edit = this.edit();

        return key.kind === "body" && edit?.row === key.row && edit.column === key.column;
    }

    /** Begin editing a body cell, from its value or from typed text, its field focused at once. */
    begin(cell: Cell<Features, Data>, initial?: string): void {
        // render and focus the field now, so the keys typed next land in it
        this.#setEdit({ row: cell.row.id, column: cell.column.id, initial, isInvalid: false });
        this.focusCell({ kind: "body", row: cell.row.id, column: cell.column.id });
    }

    /** Commit an edit's text and move the focus a step, else mark the text invalid and go on. */
    commit(text: string | undefined, step: DataTableStep): void {
        // read the edit's value, refusing text of another kind, refusing a cell no longer shown
        const { edit, cell, value } = this.#read(text);
        if (cell === undefined) {
            throw new TypeError(`data table cell '${edit.row}' '${edit.column}' is not shown`);
        } else if (value.kind === "invalid") {
            this.#setEdit({ ...edit, isInvalid: true });

            return;
        }

        // hand the owner a changed value, end the edit, and move the focus a step
        const key = { kind: "body", row: edit.row, column: edit.column } as const;
        const target = this.#stepFrom(key, step);
        this.#change(cell, value.value);
        this.#setEdit(undefined);
        this.focusCell(target);
    }

    /** End an edit the focus left with a value, else mark its text invalid and go on editing. */
    leave(text: string | undefined): void {
        // leave the edit of a cell no longer shown to the effect that ends such edits
        const { edit, cell, value } = this.#read(text);
        if (cell === undefined) {
            return;
        }
        // mark text of another kind, the edit open
        else if (value.kind === "invalid") {
            this.#setEdit({ ...edit, isInvalid: true });
        }
        // commit the value and end the edit, the focus left alone
        else {
            this.#change(cell, value.value);
            this.#setEdit(undefined);
        }
    }

    /** Leave an edit without a change, the focus back on its cell. */
    cancel(): void {
        // end the edit, refusing a cancel without one
        const edit = this.edit();
        if (edit === undefined) {
            throw new TypeError("data table cancel without an edit");
        }
        this.#setEdit(undefined);
        this.focusCell({ kind: "body", row: edit.row, column: edit.column });
    }

    /** Set a body cell's value, as Space or a click does to a checkbox. */
    set(cell: Cell<Features, Data>, value: unknown): void {
        this.#options.onCellsChange?.([{ row: cell.row.id, column: cell.column.id, value }]);
    }

    /** Clear the selected block's editable cells, else the focused cell, after spreadsheets. */
    clear(): void {
        const changes = this.#cellsIn(this.#block()).flatMap((cell) => {
            const value = this.editorOf(cell)?.parse("");

            return value?.kind === "valid"
                ? [{ row: cell.row.id, column: cell.column.id, value: value.value }]
                : [];
        });
        if (changes.length > 0) {
            this.#options.onCellsChange?.(changes);
        }
    }

    /** Copy the selected block, else the focused cell, as tab-separated text. */
    copy(event: ClipboardEvent): void {
        // leave the copy of a field's text to the field
        const block = this.#block();
        if (this.edit() !== undefined || block === undefined || event.clipboardData === null) {
            return;
        }

        // write each row of the block as values between tabs, quoted where they break the text
        const rows = this.#options.table.getRowsInDisplayOrder();
        const lines = [];
        for (let row = block.minRowIndex; row <= block.maxRowIndex; row += 1) {
            const each = rows[row];
            const cells = each === undefined ? [] : this.cellsOf(each);
            const values = cells
                .slice(block.minColumnIndex, block.maxColumnIndex + 1)
                .map(textOfCell);
            lines.push(values.map(quote).join("\t"));
        }
        event.clipboardData.setData("text/plain", lines.join("\n"));
        event.preventDefault();
    }

    /** Paste tab-separated text from the block's corner, refusing text a cell cannot read. */
    paste(event: ClipboardEvent): void {
        // leave a paste into a field to the field, a read-only table alone, and paste text only
        const block = this.#block();
        const text = event.clipboardData?.getData("text/plain") ?? "";
        if (
            this.edit() !== undefined ||
            this.#options.onCellsChange === undefined ||
            block === undefined ||
            text === ""
        ) {
            return;
        }

        // fill the block with one value, else lay the values out from its corner
        const values = parseTable(text);
        const isFill = values.length === 1 && values[0]?.length === 1;
        const height = isFill ? block.maxRowIndex - block.minRowIndex + 1 : values.length;
        const rows = this.#options.table.getRowsInDisplayOrder();
        const changes: DataTableCellChange[] = [];
        let refused:
            | {
                  readonly cell: Cell<Features, Data>;
                  readonly editor: DataTableEditor;
                  readonly text: string;
              }
            | undefined;
        for (let line = 0; line < height; line += 1) {
            // read each line's values into the cells at their places, where they edit and parse
            const row = rows[block.minRowIndex + line];
            const cells = row === undefined ? [] : this.cellsOf(row);
            const width = isFill
                ? block.maxColumnIndex - block.minColumnIndex + 1
                : (values[line]?.length ?? 0);
            for (let offset = 0; offset < width; offset += 1) {
                const cell = cells[block.minColumnIndex + offset];
                const value = isFill ? values[0]?.[0] : values[line]?.[offset];
                const editor = cell === undefined ? undefined : this.editorOf(cell);
                const parsed = value === undefined ? undefined : editor?.parse(value);
                // change a cell whose editor reads the value
                if (cell !== undefined && parsed?.kind === "valid") {
                    changes.push({ row: cell.row.id, column: cell.column.id, value: parsed.value });
                }
                // refuse the paste at the first value a cell's editor cannot read
                else if (
                    cell !== undefined &&
                    editor !== undefined &&
                    value !== undefined &&
                    parsed?.kind === "invalid"
                ) {
                    refused ??= { cell, editor, text: value };
                }
            }
        }
        event.preventDefault();

        // change nothing for a value a cell cannot read, focusing that cell
        if (refused !== undefined) {
            const { cell, editor, text: pasted } = refused;
            const key = { kind: "body", row: cell.row.id, column: cell.column.id } as const;
            this.focusCell(key);
            this.#setRefusal({ row: key.row, column: key.column, text: pasted });

            // open the pasted text to fix in place where the cell's field shows any text
            if (SHOWS_TEXT[editor.kind]) {
                this.#setEdit({
                    row: key.row,
                    column: key.column,
                    initial: pasted,
                    isInvalid: true,
                });
            }
        }
        // hand the owner every changed cell at once
        else if (changes.length > 0) {
            this.#options.onCellsChange(changes);
        }
    }

    /** Move the focus to a cell and its element's control, bringing a virtual row into view. */
    focusCell(key: CellKey): void {
        // forget a refused paste of another cell
        const refusal = untrack(this.refusal);
        if (
            refusal !== undefined &&
            (key.kind !== "body" || key.row !== refusal.row || key.column !== refusal.column)
        ) {
            this.#setRefusal(undefined);
        }

        // move the focus and render the cell
        this.focus.enter(key);
        this.focus.reveal(key);
        flush();

        // focus the cell's element where it rendered, unless the focus already rests inside it
        const selector = `[data-cell="${keyText(key)}"]`;
        const cell = this.#options.root()?.querySelector<HTMLElement>(selector);
        if (cell !== null && cell !== undefined && !cell.contains(document.activeElement)) {
            focusOf(cell).focus();
        }
    }

    /** Run a change to a column, then focus its header, else the header that takes its place. */
    changeColumn(column: string, run: () => void): void {
        // read the column's place before the change, which a hidden column's neighbour takes
        const place = this.#options.columns().indexOf(column);
        run();
        queueMicrotask(() => {
            // render the change, then focus the header in the row above the cells
            flush();
            const columns = this.#options.columns();
            const index = columns.indexOf(column);
            const at = index === -1 ? Math.min(place, columns.length - 1) : index;

            // focus nothing in a table left without columns
            if (at >= 0) {
                this.focusCell({
                    kind: "head",
                    row: this.#options.headers().length - 1,
                    column: at,
                });
            }
        });
    }

    /** Read the edit, its cell while shown, and its text as the column's value, none unreadable. */
    #read(text: string | undefined): {
        readonly edit: DataTableEdit;
        readonly cell: Cell<Features, Data> | undefined;
        readonly value: DataTableValue;
    } {
        // read the edited cell, refusing a missing edit and a shown cell without an editor
        const edit = this.edit();
        if (edit === undefined) {
            throw new TypeError("data table commit without an edit");
        }
        const cell = this.cellAt({ kind: "body", row: edit.row, column: edit.column });
        const editor = cell === undefined ? undefined : this.editorOf(cell);
        if (cell !== undefined && editor === undefined) {
            throw new TypeError(`data table cell '${edit.row}' '${edit.column}' has no editor`);
        }

        // read the text, taking text a field cannot read as invalid
        const value: DataTableValue =
            editor === undefined || text === undefined ? { kind: "invalid" } : editor.parse(text);

        return { edit, cell, value };
    }

    /** Hand the owner a cell's value where it changes. */
    #change(cell: Cell<Features, Data>, value: unknown): void {
        // forget the refused paste the edit fixed, and hand the owner a new value
        this.#setRefusal(undefined);
        if (value !== cell.getValue()) {
            this.set(cell, value);
        }
    }

    /** Read the key a step from a cell lands on, the cell itself past the grid's edge. */
    #stepFrom(key: CellKey, step: DataTableStep): CellKey {
        // read the cell's place, and the place the step lands on
        const place = present(this.placeOf(key), `the place of cell '${keyText(key)}'`);
        const target =
            step === "down"
                ? { row: place.row + 1, column: place.column }
                : step === "next"
                  ? { row: place.row, column: place.column + 1 }
                  : step === "previous"
                    ? { row: place.row, column: place.column - 1 }
                    : place;

        return this.keyAt(target) ?? key;
    }

    /** Read the block the clipboard reads: the first selected block, else the focused body cell. */
    #block(): CellSelectionBounds | undefined {
        // take the one selected block, as the table's options allow one only
        const [first] = this.#bounds();
        if (first !== undefined) {
            return first;
        }

        // else take the focused body cell alone
        const focused = this.focus.current();
        const cell = focused === undefined ? undefined : this.cellAt(focused);
        const row = cell?.row.getDisplayIndex();
        const column = cell === undefined ? undefined : this.columnPlaceOf(cell.column.id);

        return row === undefined || column === undefined
            ? undefined
            : {
                  minRowIndex: row,
                  maxRowIndex: row,
                  minColumnIndex: column,
                  maxColumnIndex: column,
              };
    }

    /** List the cells of a block, none without one. */
    #cellsIn(block: CellSelectionBounds | undefined): Cell<Features, Data>[] {
        // read each row of the block, every page's rows in order
        if (block === undefined) {
            return [];
        }
        const rows = this.#options.table
            .getRowsInDisplayOrder()
            .slice(block.minRowIndex, block.maxRowIndex + 1);

        return rows.flatMap((row) =>
            this.cellsOf(row).slice(block.minColumnIndex, block.maxColumnIndex + 1),
        );
    }

    /** The number of rows, the headers' included. */
    #count(): number {
        return this.#options.headers().length + this.#options.rows().length;
    }

    /** Read the leaf column a cell starts at: past the columns the header cells before it span. */
    #offsetOf(place: GridPlace): number {
        const spans = this.#options.headers()[place.row];

        return spans === undefined
            ? place.column
            : spans.slice(0, place.column).reduce((total, span) => total + span, 0);
    }

    /** Read the cell of a row that spans a leaf column. */
    #columnIn(row: number, offset: number): number {
        // find the header cell whose span takes the leaf column, a body row's column itself
        const spans = this.#options.headers()[row];
        if (spans === undefined) {
            return offset;
        }
        let start = 0;
        const found = spans.findIndex((span) => {
            start += span;

            return offset < start;
        });

        return found === -1 ? spans.length - 1 : found;
    }

    /** Read the number of cells in a row of the grid, none past its ends. */
    #width(row: number): number | undefined {
        const headers = this.#options.headers();

        return row < headers.length
            ? headers[row]?.length
            : row < this.#count()
              ? this.#options.columns().length
              : undefined;
    }
}

/** Read a column's meta, empty for a column that declares none. */
export function metaOf<Features extends TableFeatures, Data extends RowData, Value>(
    column: Column<Features, Data, Value>,
): DataTableColumnMeta {
    return column.columnDef.meta ?? {};
}

/** Write a cell's key as text with its ids encoded, as its element's `data-cell` has it. */
export function keyText(key: CellKey): string {
    return [key.kind, key.row, key.column].map((part) => encodeURIComponent(part)).join(":");
}

/** Find the element that takes a cell's focus: its single control, else the cell. */
export function focusOf(cell: HTMLElement): HTMLElement {
    const controls = [...cell.querySelectorAll<HTMLElement>(FOCUSABLE)].filter(
        (each) => each.closest("[role=menu]") === null,
    );
    const [control] = controls;

    return controls.length === 1 && control !== undefined ? control : cell;
}

/** Read the row and column a grid move lands the focus on, the last column of a row as `last`. */
function landingOf(
    move: GridMove,
    place: GridPlace,
    last: number,
): { readonly row: number; readonly column: number | "last" } {
    // step a cell along a row or a column
    const { row, column } = place;
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
    // jump to the ends of the row or of the grid
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

/** Report whether a table selects blocks of cells through the cell selection feature. */
function isSelectingTable(table: object): table is Table_CellSelection<TableFeatures, RowData> & {
    readonly store: { readonly state: TableState_CellSelection };
} {
    return "extendCellSelection" in table && typeof table.extendCellSelection === "function";
}

/** Report whether a cell selects through the cell selection feature. */
export function isSelectableCell(cell: object): cell is Cell_CellSelection {
    return (
        "getSelectionStartHandler" in cell && typeof cell.getSelectionStartHandler === "function"
    );
}

/** Report whether two lists of selected blocks have the same edges. */
function isSameBounds(
    previous: readonly CellSelectionBounds[],
    next: readonly CellSelectionBounds[],
): boolean {
    return (
        previous.length === next.length &&
        previous.every((bounds, index) => {
            const other = next[index];

            return (
                other !== undefined &&
                bounds.minRowIndex === other.minRowIndex &&
                bounds.maxRowIndex === other.maxRowIndex &&
                bounds.minColumnIndex === other.minColumnIndex &&
                bounds.maxColumnIndex === other.maxColumnIndex
            );
        })
    );
}

/** Report whether a row stands for a group of rows. */
export function isGroupRow(row: object): boolean {
    return isGroupingRow(row) && row.getIsGrouped();
}

/** Report whether a row belongs to a table that groups rows. */
function isGroupingRow(row: object): row is Row_ColumnGrouping {
    return "getIsGrouped" in row && typeof row.getIsGrouped === "function";
}

/** Report whether a cell sums a group's rows or stands empty in a group's row. */
export function isGroupCell(cell: object): boolean {
    return isGroupingCell(cell) && (cell.getIsAggregated() || cell.getIsPlaceholder());
}

/** Report whether a cell belongs to a table that groups rows. */
function isGroupingCell(cell: object): cell is Cell_ColumnGrouping & {
    readonly getIsAggregated: () => boolean;
} {
    return "getIsAggregated" in cell && typeof cell.getIsAggregated === "function";
}

/** Write a value as a table's field, quoted where it has a tab, a line break or a quote. */
function quote(value: string): string {
    return /[\t\n\r"]/u.test(value) ? `"${value.replaceAll('"', '""')}"` : value;
}

/** Read tab-separated text into lines of values, quoted values whole, after spreadsheets. */
function parseTable(text: string): string[][] {
    // read character by character, a quote opening a value that ends at its closing quote
    const lines: string[][] = [[]];
    let value = "";
    let isQuoted = false;
    for (let index = 0; index < text.length; index += 1) {
        const character = text.charAt(index);
        const line = lines.at(-1) ?? [];

        // take a quoted value's characters, a doubled quote as one
        if (isQuoted) {
            if (character === '"' && text.charAt(index + 1) === '"') {
                value += '"';
                index += 1;
            } else if (character === '"') {
                isQuoted = false;
            } else {
                value += character;
            }
        }
        // open a quoted value at its start
        else if (character === '"' && value === "") {
            isQuoted = true;
        }
        // end a value at a tab, and a line at a line break
        else if (character === "\t") {
            line.push(value);
            value = "";
        } else if (character === "\n" || character === "\r") {
            line.push(value);
            value = "";
            lines.push([]);
            if (character === "\r" && text.charAt(index + 1) === "\n") {
                index += 1;
            }
        }
        // take any other character
        else {
            value += character;
        }
    }

    // end the last value, leaving out the empty line after a final line break
    lines.at(-1)?.push(value);
    const last = lines.at(-1);

    return last?.length === 1 && last[0] === "" && lines.length > 1 ? lines.slice(0, -1) : lines;
}

/** Write a cell's value as text: through its column's editor, else as plain text. */
function textOfCell<Features extends TableFeatures, Data extends RowData>(
    cell: Cell<Features, Data>,
): string {
    const editor = metaOf(cell.column).editor;

    return editor === undefined ? textOf(cell.getValue()) : editor.format(cell.getValue());
}

/** Write a value as plain text, nothing for an empty value. */
function textOf(value: unknown): string {
    return value === undefined || value === null
        ? ""
        : typeof value === "string" || typeof value === "number" || typeof value === "boolean"
          ? String(value)
          : JSON.stringify(value);
}
