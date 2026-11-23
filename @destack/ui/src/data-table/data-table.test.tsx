import { beforeEach, expect, onTestFinished, test } from "@destack/test";
import {
    cellSelectionFeature,
    columnOrderingFeature,
    columnPinningFeature,
    columnVisibilityFeature,
    columnFilteringFeature,
    createColumnHelper,
    createFilteredRowModel,
    createPaginatedRowModel,
    createSortedRowModel,
    filterFn_includesString,
    globalFilteringFeature,
    rowPaginationFeature,
    rowSelectionFeature,
    rowSortingFeature,
    sortFn_alphanumeric,
    sortFn_basic,
    tableFeatures,
} from "./index.ts";
import { createSignal, Errored, flush, type JSX } from "@destack/view";
import { render, stubPopovers, wait } from "@destack/view/test";
import {
    createTable,
    DataTable,
    DataTableBulkBar,
    type DataTableCellChange,
    DataTableColumnHeader,
    DataTableEditor,
    type DataTableRowMove,
    orderColumn,
    DataTableFilter,
    DataTablePagination,
    selectionColumn,
    useDataTableState,
} from "./index.ts";
import {
    dataTableNotesGrouped,
    dataTableNotesVirtual,
    dataTableTasksNested,
} from "./data-table.example.tsx";

/** The marks that isolate interpolated names in rendered messages. */
const ISOLATES = /[⁦-⁩]/gu;

/** An invoice the table lists. */
interface Invoice {
    /** The invoice number. */
    readonly id: string;
    /** The customer billed. */
    readonly customer: string;
    /** The amount in cents. */
    readonly amount: number;
}

/** Twelve invoices, two pages of ten. */
const INVOICES: Invoice[] = Array.from({ length: 12 }, (_, index) => ({
    id: `INV${String(index + 1).padStart(3, "0")}`,
    customer: index % 2 === 0 ? `Ada ${index}` : `Grace ${index}`,
    amount: (12 - index) * 1000,
}));

/** The features of the invoice table. */
const FEATURES = tableFeatures({
    rowSortingFeature,
    sortedRowModel: createSortedRowModel(),
    sortFns: { alphanumeric: sortFn_alphanumeric, basic: sortFn_basic },
    columnFilteringFeature,
    globalFilteringFeature,
    filteredRowModel: createFilteredRowModel(),
    filterFns: { includesString: filterFn_includesString },
    rowPaginationFeature,
    paginatedRowModel: createPaginatedRowModel(),
    rowSelectionFeature,
});

/** The column helper of the invoice table. */
const COLUMN = createColumnHelper<typeof FEATURES, Invoice>();

/** The columns: a selection box, the number, a searchable customer and a sortable amount. */
const COLUMNS = COLUMN.columns([
    selectionColumn<typeof FEATURES, Invoice>(),
    COLUMN.accessor("id", { header: "Invoice", enableSorting: false }),
    COLUMN.accessor("customer", {
        header: (context) => <DataTableColumnHeader column={context.column} />,
        meta: { label: "Customer" },
    }),
    COLUMN.accessor("amount", {
        header: (context) => <DataTableColumnHeader column={context.column} />,
        meta: { label: "Amount" },
        cell: (context) => String(context.getValue() / 100),
        enableGlobalFilter: false,
    }),
]);

/** Render the invoices with a filter and a pagination. */
function drawInvoices(): HTMLElement {
    return render(() => {
        const table = createTable({
            features: FEATURES,
            columns: COLUMNS,
            data: INVOICES,
            getRowId: (row) => row.id,
        });

        return (
            <>
                <DataTableFilter table={table} placeholder="Filter invoices" />
                <DataTable table={table} />
                <DataTablePagination table={table} />
            </>
        );
    }).container;
}

/** Read the invoice numbers of the shown rows. */
function shownIds(container: Element): string[] {
    return [...container.querySelectorAll("tbody tr")].map(
        (row) => row.querySelectorAll("td")[1]?.textContent ?? "",
    );
}

/** Click the button inside a column's header. */
function sortBy(container: Element, header: string): void {
    const button = [...container.querySelectorAll<HTMLElement>("th button")].find(
        (entry) => entry.textContent === header,
    );
    button?.click();
    flush();
}

/** Press a key on the focused element. */
function press(key: string): void {
    document.activeElement?.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
    flush();
}

/** Click the button a pagination names. */
function step(container: Element, label: string): void {
    container.querySelector<HTMLElement>(`[aria-label="${label}"]`)?.click();
    flush();
}

// stub the top layer the column menus open in, which the test DOM lacks
beforeEach(() => stubPopovers());

test("page through rows ten at a time", () => {
    const container = drawInvoices();
    flush();
    const first = shownIds(container);
    const page = [...container.querySelectorAll("[data-slot=data-table-pagination] > span")].map(
        (span) => (span.textContent ?? "").replaceAll(ISOLATES, ""),
    )[1];
    step(container, "Go to the next page");
    expect([first.length, page, shownIds(container)]).toEqual([
        10,
        "Page 1 of 2",
        ["INV011", "INV012"],
    ]);
});

test("sort by a column ascending, descending and back to the rows' order as given, marking it with aria-sort", () => {
    const container = drawInvoices();
    flush();
    const sorts: (string | null | undefined)[] = [];
    const firsts: (string | undefined)[] = [];
    for (let click = 0; click < 3; click += 1) {
        sortBy(container, "Customer");
        sorts.push(container.querySelector("th[data-column=customer]")?.getAttribute("aria-sort"));
        firsts.push(shownIds(container)[0]);
    }
    expect([sorts, firsts]).toEqual([
        ["ascending", "descending", "none"],
        ["INV001", "INV012", "INV001"],
    ]);
});

test("filter rows across their searchable cells, showing a message when none match", () => {
    const container = drawInvoices();
    const input = container.querySelector<HTMLInputElement>("[data-slot=data-table-filter]");
    if (input !== null) {
        input.value = "grace";
        input.dispatchEvent(new Event("input", { bubbles: true }));
    }
    flush();
    const matched = shownIds(container).length;
    if (input !== null) {
        input.value = "nobody";
        input.dispatchEvent(new Event("input", { bubbles: true }));
    }
    flush();
    expect([matched, container.querySelector("tbody")?.textContent]).toEqual([6, "No results."]);
});

test("select rows and the whole page, the header box showing a partial selection", () => {
    const container = drawInvoices();
    flush();
    container.querySelector<HTMLElement>("tbody [role=checkbox]")?.click();
    flush();
    const header = container.querySelector<HTMLElement>("thead [role=checkbox]");
    const partial = header?.getAttribute("aria-checked");
    header?.click();
    flush();
    expect([
        partial,
        container.querySelectorAll("tbody tr[data-selected]").length,
        container.querySelector("[data-slot=data-table-pagination] span")?.textContent,
    ]).toEqual(["mixed", 10, "10 of 12 rows selected"]);
});

test("move the focus between cells with the arrow keys, with one tab stop", () => {
    const container = drawInvoices();
    flush();
    container.querySelector<HTMLElement>("tbody td:nth-child(2)")?.focus();
    press("ArrowDown");
    press("ArrowRight");
    const focused = document.activeElement?.textContent;
    const stops = container.querySelectorAll("table [tabindex='0']").length;
    expect([focused, stops]).toEqual(["Grace 1", 1]);
});

/** Read each body row of a container as its level, expansion and cell texts. */
function treeRows(container: Element): (string | null)[][] {
    return [...container.querySelectorAll("tbody tr")].map((row) => [
        row.getAttribute("aria-level"),
        row.getAttribute("aria-expanded"),
        ...[...row.querySelectorAll("td")].map((cell) => cell.textContent),
    ]);
}

test("group rows by a column with each group's count and sum, expanding a group to its rows", () => {
    const { container } = render(() => dataTableNotesGrouped.render?.());
    const collapsed = treeRows(container);
    const role = container.querySelector("table")?.getAttribute("role");
    container.querySelector<HTMLElement>("[aria-label='Expand row']")?.click();
    flush();
    expect([role, collapsed, treeRows(container)]).toEqual([
        "treegrid",
        [
            ["1", "false", "Home(2)", "", "473"],
            ["1", "false", "Work(2)", "", "1516"],
            ["1", "false", "Trips(1)", "", "940"],
        ],
        [
            ["1", "true", "Home(2)", "", "473"],
            ["2", null, "", "Groceries", "18"],
            ["2", null, "", "Rye bread recipe", "455"],
            ["1", "false", "Work(2)", "", "1516"],
            ["1", "false", "Trips(1)", "", "940"],
        ],
    ]);
});

test("expand a row to its sub rows and collapse it again from its button", () => {
    const { container } = render(() => dataTableTasksNested.render?.());
    const expanded = treeRows(container);
    container.querySelector<HTMLElement>("[aria-label='Collapse row']")?.click();
    flush();
    expect([expanded, treeRows(container)]).toEqual([
        [
            ["1", "true", "", "Launch the site"],
            ["2", null, "", "Write the copy"],
            ["2", null, "", "Deploy"],
            ["1", null, "", "Send invoices"],
        ],
        [
            ["1", "false", "", "Launch the site"],
            ["1", null, "", "Send invoices"],
        ],
    ]);
});

test("refuse a template reading a data table's state outside a data table", () => {
    const { container } = render(() => (
        <Errored fallback={(error) => String(error())}>
            <StateTemplate />
        </Errored>
    ));

    expect(container.textContent).toBe(
        "TypeError: a data table template needs a data table around it",
    );
});

/** Render the state of the nearest data table as a header or cell template reads it. */
function StateTemplate(): JSX.Element {
    return <>{JSON.stringify(useDataTableState()())}</>;
}

/** Press a key on the focused element and let a scrolled table render. */
async function pressAndRead(key: string, modifiers: KeyboardEventInit = {}): Promise<string> {
    // press the key and wait for the rows scrolled into view
    document.activeElement?.dispatchEvent(
        new KeyboardEvent("keydown", { key, bubbles: true, ...modifiers }),
    );
    flush();
    await wait(1);
    flush();

    // read a body cell's text, or a header's name on its sort button
    const cell = document.activeElement?.closest("tr")?.querySelector("[data-column=title]");
    const name = cell?.querySelector("[data-slot=data-table-column-header] > button") ?? cell;

    return name?.textContent ?? "";
}

test("move the focus by key onto rows a virtualized table renders only once they scroll into view", async () => {
    // give the scrolling table a box of five rows, which the test DOM does not lay out
    const height = Object.getOwnPropertyDescriptor(HTMLElement.prototype, "clientHeight");
    Object.defineProperty(HTMLElement.prototype, "clientHeight", {
        configurable: true,
        get(this: HTMLElement) {
            return this.dataset["slot"] === "data-table" ? 200 : 0;
        },
    });
    onTestFinished(() => {
        if (height !== undefined) {
            Object.defineProperty(HTMLElement.prototype, "clientHeight", height);
        }
    });

    // measure the row height at forty pixels
    Object.defineProperty(HTMLElement.prototype, "getBoundingClientRect", {
        configurable: true,
        value(this: HTMLElement) {
            return this.dataset["slot"] === "length"
                ? new DOMRect(0, 0, 40, 0)
                : new DOMRect(0, 0, 0, 0);
        },
    });
    onTestFinished(() => {
        Reflect.deleteProperty(HTMLElement.prototype, "getBoundingClientRect");
    });
    stubPopovers();
    const { container } = render(dataTableNotesVirtual);
    flush();
    const scroller = container.querySelector<HTMLElement>("[data-slot=data-table]");
    let scrollTop = 0;
    if (scroller !== null) {
        Object.defineProperty(scroller, "scrollTop", { get: () => scrollTop });
        Object.defineProperty(scroller, "scrollHeight", { value: 1000 * 40 });
        Object.defineProperty(scroller, "clientHeight", { value: 200 });
        scroller.scrollTo = (options?: ScrollToOptions | number) => {
            scrollTop = typeof options === "object" ? (options.top ?? scrollTop) : scrollTop;
            scroller.dispatchEvent(new Event("scroll"));
        };
    }

    // move from the first note's title to the grid's last cell, a page of rows up, and to the header
    container.querySelector<HTMLElement>("tbody tr:first-child td[data-column=title]")?.focus();
    const steps = [
        await pressAndRead("End", { ctrlKey: true }),
        await pressAndRead("PageUp"),
        await pressAndRead("Home", { ctrlKey: true }),
    ];

    // each row takes the focus once it renders, though it was out of view when the key moved there
    expect({ steps, rendered: container.querySelectorAll("tbody tr").length < 100 }).toEqual({
        steps: ["Note 1000", "Note 990", "Title"],
        rendered: true,
    });
});

/** A plant a sheet lists, one of each kind of editable value. */
interface Plant {
    /** The plant's key. */
    readonly id: string;
    /** The plant's name, edited as text. */
    readonly name: string;
    /** How many are planted, edited as a number. */
    readonly count: number;
    /** The day it was planted, edited as a date. */
    readonly planted: string;
    /** Where it grows, chosen among places. */
    readonly place: string;
    /** Whether it needs water, checked by Space. */
    readonly isDry: boolean;
}

/** The plant sheet's features: sorting, paging, selecting rows and cells, arranging columns. */
const SHEET = tableFeatures({
    rowSortingFeature,
    sortedRowModel: createSortedRowModel(),
    sortFns: { alphanumeric: sortFn_alphanumeric, basic: sortFn_basic },
    rowPaginationFeature,
    paginatedRowModel: createPaginatedRowModel(),
    rowSelectionFeature,
    cellSelectionFeature,
    columnOrderingFeature,
    columnVisibilityFeature,
    columnPinningFeature,
});

/** The column helper of the plant sheet. */
const PLANT = createColumnHelper<typeof SHEET, Plant>();

/** The plants of the sheet. */
const PLANTS: Plant[] = [
    { id: "fern", name: "Fern", count: 3, planted: "2026-03-01", place: "shade", isDry: false },
    { id: "rose", name: "Rose", count: 5, planted: "2026-04-12", place: "sun", isDry: true },
    { id: "mint", name: "Mint", count: 2, planted: "2026-02-20", place: "sun", isDry: false },
];

/** List the sheet's one row action, which records the rows it ran on. */
function waterOf(acted: (readonly string[])[], rows: readonly string[]) {
    return [{ id: "water", title: "Water", run: () => void acted.push(rows) }];
}

/** Draw the plant sheet a page at a time, writing changes into its rows and recording them. */
function drawSheet(options: { readonly pageSize?: number; readonly isReadOnly?: boolean } = {}): {
    readonly container: HTMLElement;
    readonly changes: (readonly DataTableCellChange[])[];
    readonly moves: DataTableRowMove[];
    readonly acted: (readonly string[])[];
} {
    // the sheet's columns: a row handle, a selection box, and one column of each editor
    const changes: (readonly DataTableCellChange[])[] = [];
    const moves: DataTableRowMove[] = [];
    const acted: (readonly string[])[] = [];
    const columns = PLANT.columns([
        PLANT.display(orderColumn()),
        selectionColumn<typeof SHEET, Plant>(),
        PLANT.accessor("name", {
            header: (context) => <DataTableColumnHeader column={context.column} />,
            meta: { label: "Name", editor: DataTableEditor.text(), summary: "count" },
        }),
        PLANT.accessor("count", {
            header: (context) => <DataTableColumnHeader column={context.column} />,
            meta: { label: "Count", editor: DataTableEditor.number(), summary: "sum" },
        }),
        PLANT.accessor("planted", {
            header: "Planted",
            meta: { label: "Planted", editor: DataTableEditor.date(), summary: "min" },
        }),
        PLANT.accessor("place", {
            header: "Place",
            meta: {
                label: "Place",
                editor: DataTableEditor.select([
                    { value: "sun", label: "Sun" },
                    { value: "shade", label: "Shade" },
                ]),
            },
        }),
        PLANT.accessor("isDry", {
            header: "Dry",
            meta: { label: "Dry", editor: DataTableEditor.checkbox(), summary: "count" },
        }),
    ]);
    const { container } = render(() => {
        // write each change into the plants, as an owner does
        const [plants, setPlants] = createSignal(PLANTS);
        const write = (each: readonly DataTableCellChange[]) => {
            changes.push(each);
            setPlants((current) =>
                current.map((plant) => {
                    // copy only the plants that change, as an owner keeps the others
                    const changed = each.filter((change) => change.row === plant.id);

                    return changed.length === 0
                        ? plant
                        : {
                              ...plant,
                              ...Object.fromEntries(
                                  changed.map((change) => [change.column, change.value]),
                              ),
                          };
                }),
            );
        };
        const table = createTable({
            features: SHEET,
            columns,
            get data() {
                return plants();
            },
            getRowId: (row) => row.id,
            enableRowSelection: true,
            initialState: { pagination: { pageIndex: 0, pageSize: options.pageSize ?? 10 } },
        });
        const actionsOf = waterOf.bind(undefined, acted);

        return (
            <>
                <DataTableBulkBar table={table} actions={actionsOf} />
                <DataTable
                    table={table}
                    {...(options.isReadOnly === true ? {} : { onCellsChange: write })}
                    onRowMove={(move) => moves.push(move)}
                    rowActions={actionsOf}
                />
            </>
        );
    });
    flush();

    return { container, changes, moves, acted };
}

/** Focus a body cell by its row's and column's ids. */
function focusCell(container: HTMLElement, row: string, column: string): HTMLElement {
    const cell =
        container.querySelector<HTMLElement>(`tr[data-row=${row}] td[data-column=${column}]`) ??
        undefined;
    if (cell === undefined) {
        throw new TypeError(`no cell '${row}' '${column}'`);
    }
    cell.focus();
    flush();

    return cell;
}

/** Press a key on the focused element, with modifiers. */
function pressKey(name: string, modifiers: KeyboardEventInit = {}): void {
    document.activeElement?.dispatchEvent(
        new KeyboardEvent("keydown", { key: name, bubbles: true, cancelable: true, ...modifiers }),
    );
    flush();
}

/** Run a clipboard event on the focused element, reading and writing the given text. */
function clipboard(type: "copy" | "paste", text = ""): string {
    // stand in for the clipboard's data, which the test DOM lacks
    let written = text;
    const event = new Event(type, { bubbles: true, cancelable: true });
    Object.defineProperty(event, "clipboardData", {
        value: {
            getData: () => written,
            setData: (_type: string, value: string) => {
                written = value;
            },
        },
    });
    document.activeElement?.dispatchEvent(event);
    flush();

    return written;
}

test("edit a cell by typing over it, Enter writing it and moving down, Escape leaving it", () => {
    // type over Fern's name, then open Rose's count with F2 and leave it with Escape
    const sheet = drawSheet();
    focusCell(sheet.container, "fern", "name");
    pressKey("O");
    const field = document.activeElement;
    if (!(field instanceof HTMLInputElement)) {
        throw new TypeError("typing opened no field");
    }
    field.value = "Oak fern";
    pressKey("Enter");
    const below = document.activeElement?.closest("td")?.textContent;
    focusCell(sheet.container, "rose", "count");
    pressKey("F2");
    pressKey("Escape");
    const escaped = document.activeElement?.closest("td")?.dataset["column"];

    expect({ changes: sheet.changes, below, escaped }).toEqual({
        changes: [[{ row: "fern", column: "name", value: "Oak fern" }]],
        below: "Rose",
        escaped: "count",
    });
});

test("commit a number with Tab and move across, refuse text that is none, and toggle a box", () => {
    // try a word in the last plant's count, then 4 by Tab, then click the second plant's dry box
    const sheet = drawSheet();
    focusCell(sheet.container, "mint", "count");
    pressKey("Enter");
    const field = document.activeElement;
    if (!(field instanceof HTMLInputElement)) {
        throw new TypeError("enter opened no field");
    }
    field.value = "four";
    pressKey("Tab");
    flush();
    const refused = [
        document.activeElement === field,
        field.getAttribute("aria-invalid"),
        field.value,
    ];
    field.value = "4";
    pressKey("Tab");
    const across = document.activeElement?.closest("td")?.dataset["column"];
    sheet.container
        .querySelectorAll<HTMLElement>("tbody td[data-column=isDry] [role=checkbox]")[1]
        ?.click();
    flush();

    expect({ changes: sheet.changes, refused, across }).toEqual({
        changes: [
            [{ row: "mint", column: "count", value: 4 }],
            [{ row: "rose", column: "isDry", value: false }],
        ],
        refused: [true, "true", "four"],
        across: "planted",
    });
});

test("toggle a checkbox cell with Enter from the keyboard", () => {
    // walk to Rose's dry box and press Enter
    const sheet = drawSheet();
    focusCell(sheet.container, "rose", "isDry");
    pressKey("Enter");

    expect(sheet.changes).toEqual([[{ row: "rose", column: "isDry", value: false }]]);
});

test("copy a range of cells as tab-separated text, and paste text into editable cells", () => {
    // select Fern's and Rose's names and counts with shift and the arrows, copy, then paste below
    const sheet = drawSheet();
    focusCell(sheet.container, "fern", "name");
    pressKey("ArrowRight", { shiftKey: true });
    pressKey("ArrowDown", { shiftKey: true });
    const marked = sheet.container.querySelectorAll("td[data-in-range]").length;
    const copied = clipboard("copy");
    focusCell(sheet.container, "mint", "name");
    clipboard("paste", '"Sweet\nbasil"\t7');

    // the quoted name arrives with its line break whole, and both cells change in one call
    expect({ marked, copied, changes: sheet.changes }).toEqual({
        marked: 4,
        copied: "Fern\t3\nRose\t5",
        changes: [
            [
                { row: "mint", column: "name", value: "Sweet\nbasil" },
                { row: "mint", column: "count", value: 7 },
            ],
        ],
    });
});

test("refuse a paste with a value a cell cannot read, editing that cell's pasted text", () => {
    // paste a name and a word into Mint's name and count
    const sheet = drawSheet();
    focusCell(sheet.container, "mint", "name");
    clipboard("paste", "Basil\tseven");
    const field = sheet.container.querySelector<HTMLInputElement>("[data-editing] input");

    // nothing changes, and Mint's count opens with the word marked invalid and focused
    expect({
        changes: sheet.changes,
        editing: field?.closest("td")?.getAttribute("data-column"),
        text: field?.value,
        invalid: field?.getAttribute("aria-invalid"),
        focused: document.activeElement === field,
    }).toEqual({ changes: [], editing: "count", text: "seven", invalid: "true", focused: true });
});

test("refuse a paste a date, select or checkbox cell cannot read, marking the cell and saying why", () => {
    // paste an impossible date, an unknown place and a maybe into Rose's cells, one at a time
    const sheet = drawSheet();
    const refuse = (column: string, text: string) => {
        focusCell(sheet.container, "rose", column);
        clipboard("paste", text);
        const cell = document.activeElement?.closest("td");

        return [
            cell?.dataset["column"],
            cell?.getAttribute("aria-invalid"),
            sheet.container.querySelectorAll("[data-editing]").length,
            sheet.container
                .querySelector("[data-slot=data-table-refusal]")
                ?.textContent.replace(/[⁦-⁩]/gu, ""),
        ];
    };
    const refused = [
        refuse("planted", "2026-13-45"),
        refuse("place", "Nowhere"),
        refuse("isDry", "maybe"),
    ];

    // the cell stays unchanged, opens no field, and the table says what it refused where
    pressKey("ArrowDown");
    const left = sheet.container.querySelector("[data-slot=data-table-refusal]")?.textContent;

    expect({ changes: sheet.changes, refused, left }).toEqual({
        changes: [],
        refused: [
            ["planted", "true", 0, "Can't paste “2026-13-45” into Planted"],
            ["place", "true", 0, "Can't paste “Nowhere” into Place"],
            ["isDry", "true", 0, "Can't paste “maybe” into Dry"],
        ],
        left: "",
    });
});

test("sum a column across pages, count filled and checked cells, and take the earliest date", () => {
    const sheet = drawSheet({ pageSize: 2 });

    expect(
        [...sheet.container.querySelectorAll("tfoot td")].map((cell) =>
            (cell.textContent ?? "").replaceAll(ISOLATES, ""),
        ),
    ).toEqual(["", "", "Count 3", "Sum 10", "Earliest 2026-02-20", "", "Count 1"]);
});

test("move a row by its handle with the keys, handing the owner the rows it lands between", () => {
    // lift Fern by its handle and step it below Rose
    const sheet = drawSheet();
    sheet.container.querySelector<HTMLElement>("[data-slot=data-table-row-handle]")?.focus();
    pressKey(" ");
    pressKey("ArrowDown");
    pressKey(" ");

    expect(sheet.moves).toEqual([{ id: "fern", previous: "rose", next: "mint" }]);
});

test("act on a right-clicked row with its selection, and on the selection from the bulk bar", () => {
    // select Fern and Rose, right-click Rose and water, then water from the bar
    const sheet = drawSheet();
    for (const box of [
        ...sheet.container.querySelectorAll<HTMLElement>(
            "tbody td[data-column=select] [role=checkbox]",
        ),
    ].slice(0, 2)) {
        box.click();
        flush();
    }
    const rose = sheet.container.querySelectorAll<HTMLElement>("tbody tr")[1]?.querySelector("td");
    rose?.dispatchEvent(
        new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 5, clientY: 5 }),
    );
    flush();
    [...document.querySelectorAll<HTMLElement>("[role=menuitem]")]
        .find((item) => item.textContent === "Water")
        ?.click();
    flush();
    const bar = sheet.container.querySelector("[data-slot=data-table-bulk-bar]");
    const count = (bar?.querySelector("span")?.textContent ?? "").replaceAll(ISOLATES, "");
    [...(bar?.querySelectorAll<HTMLElement>("button") ?? [])]
        .find((button) => button.textContent === "Water")
        ?.click();

    expect({ count, acted: sheet.acted }).toEqual({
        count: "2 rows selected",
        acted: [
            ["fern", "rose"],
            ["fern", "rose"],
        ],
    });
});

test("sort from a column's menu, mark each sort's place among several, and move a column right", () => {
    // sort Count descending from its menu, add Name by shift, then move Name right
    const sheet = drawSheet();
    const menuOf = (column: string) =>
        sheet.container.querySelector<HTMLElement>(
            `th[data-column=${column}] [aria-haspopup=menu]`,
        );
    const choose = (column: string, item: string) => {
        // open the column's menu and choose one of its items
        menuOf(column)?.click();
        flush();
        [
            ...sheet.container.querySelectorAll<HTMLElement>(
                `th[data-column=${column}] [role=menuitem]`,
            ),
        ]
            .find((each) => each.textContent === item)
            ?.click();
        flush();
    };
    choose("count", "Sort descending");
    [...sheet.container.querySelectorAll<HTMLElement>("th button")]
        .find((button) => button.textContent === "Name")
        ?.dispatchEvent(new MouseEvent("click", { bubbles: true, shiftKey: true }));
    flush();
    const marks = [
        ...sheet.container.querySelectorAll(
            "[data-slot=data-table-column-header] > [data-slot=button]",
        ),
    ].map((button) => button.textContent);
    const order = [
        ...sheet.container.querySelectorAll<HTMLElement>("tbody tr td[data-column=name]"),
    ].map((cell) => cell.textContent);
    choose("name", "Move right");
    const columns = [...sheet.container.querySelectorAll<HTMLElement>("thead th")].map(
        (head) => head.dataset["column"],
    );
    const cells = [...sheet.container.querySelectorAll<HTMLElement>("tbody tr:first-child td")].map(
        (cell) => cell.dataset["column"],
    );

    // the body's cells follow the headers
    expect({ marks, order, columns, cells }).toEqual({
        marks: ["Name2", "Count1"],
        order: ["Rose", "Fern", "Mint"],
        columns: ["order", "select", "count", "name", "planted", "place", "isDry"],
        cells: ["order", "select", "count", "name", "planted", "place", "isDry"],
    });
});

test("hide a column and pin another from their menus, the body's cells following the headers", () => {
    // hide Count and pin Name to the end
    const sheet = drawSheet();
    for (const [column, item] of [
        ["count", "Hide"],
        ["name", "Pin to end"],
    ] as const) {
        sheet.container
            .querySelector<HTMLElement>(`th[data-column=${column}] [aria-haspopup=menu]`)
            ?.click();
        flush();
        [
            ...sheet.container.querySelectorAll<HTMLElement>(
                `th[data-column=${column}] [role=menuitem]`,
            ),
        ]
            .find((each) => each.textContent === item)
            ?.click();
        flush();
    }
    const heads = [...sheet.container.querySelectorAll<HTMLElement>("thead th")].map(
        (head) => head.dataset["column"],
    );
    const cells = [...sheet.container.querySelectorAll<HTMLElement>("tbody tr:first-child td")].map(
        (cell) => cell.dataset["column"],
    );

    expect({ heads, cells }).toEqual({
        heads: ["order", "select", "planted", "place", "isDry", "name"],
        cells: ["order", "select", "planted", "place", "isDry", "name"],
    });
});

/** Read what a data table says of a refused paste, without its text's direction marks. */
function refusalOf(container: HTMLElement): string | undefined {
    return container
        .querySelector("[data-slot=data-table-refusal]")
        ?.textContent.replace(/[⁦-⁩]/gu, "");
}

/** Choose an item of a column's menu by the column's id. */
function chooseInHeaderMenu(container: HTMLElement, column: string, item: string): void {
    // open the menu from its header's button, then press the item
    container.querySelector<HTMLElement>(`th[data-column=${column}] [aria-haspopup=menu]`)?.click();
    flush();
    [...container.querySelectorAll<HTMLElement>(`th[data-column=${column}] [role=menuitem]`)]
        .find((each) => each.textContent === item)
        ?.click();
    flush();
}

test("leave a press on a header's sort button to the button, lifting no column however far it moves", () => {
    // press Name's sort button and drag the pointer far across the header row
    const sheet = drawSheet();
    const button = sheet.container.querySelector<HTMLElement>("th[data-column=name] button");
    if (button === null) {
        throw new TypeError("the Name header has no sort button");
    }
    const lifted: boolean[] = [];
    for (const [type, x] of [
        ["pointerdown", 10],
        ["pointermove", 400],
        ["pointerup", 400],
    ] as const) {
        button.dispatchEvent(
            new PointerEvent(type, {
                bubbles: true,
                cancelable: true,
                clientX: x,
                clientY: 5,
                pointerId: 1,
            }),
        );
        flush();
        lifted.push(sheet.container.querySelector("th[data-dragging]") !== null);
    }

    expect(lifted).toEqual([false, false, false]);
});

test("focus the header that takes a hidden column's place once its menu hides it", async () => {
    // hide Count from its menu
    const sheet = drawSheet();
    chooseInHeaderMenu(sheet.container, "count", "Hide");
    await Promise.resolve();

    expect(document.activeElement?.closest("th")?.dataset["column"]).toBe("planted");
});

test("stop a column's header from moving once it pins, focused, and let it move again once it unpins", async () => {
    // drag Name's header before pinning it, while pinned, and after unpinning it
    const sheet = drawSheet();
    const lifts = () => {
        const head = sheet.container.querySelector<HTMLElement>("th[data-column=name]");
        const lifted: boolean[] = [];
        for (const [type, x] of [
            ["pointerdown", 10],
            ["pointermove", 400],
            ["pointerup", 400],
        ] as const) {
            head?.dispatchEvent(
                new PointerEvent(type, {
                    bubbles: true,
                    cancelable: true,
                    clientX: x,
                    clientY: 5,
                    pointerId: 1,
                }),
            );
            flush();
            lifted.push(sheet.container.querySelector("th[data-dragging]") !== null);
        }

        return lifted.includes(true);
    };
    const before = lifts();
    chooseInHeaderMenu(sheet.container, "name", "Pin to start");
    await Promise.resolve();
    const focused = document.activeElement?.closest("th")?.dataset["column"];
    const pinned = lifts();
    chooseInHeaderMenu(sheet.container, "name", "Unpin");

    // the pinned header takes the focus back from its menu
    expect([before, focused, pinned, lifts()]).toEqual([true, "name", false, true]);
});

test("end an invalid edit whose column hides, the focus moving through the grid again", () => {
    // type a word into Rose's count, leave the table, hide Count, then focus Fern's name
    const sheet = drawSheet();
    const outside = document.createElement("button");
    document.body.append(outside);
    onTestFinished(() => outside.remove());
    focusCell(sheet.container, "rose", "count");
    pressKey("x");
    outside.focus();
    flush();
    chooseInHeaderMenu(sheet.container, "count", "Hide");
    const name = focusCell(sheet.container, "fern", "name");

    expect({
        changes: sheet.changes,
        editing: sheet.container.querySelectorAll("[data-editing]").length,
        focused: document.activeElement === name,
    }).toEqual({ changes: [], editing: 0, focused: true });
});

test("drop the selected cells as a sort reorders the rows, copying the focused cell alone", () => {
    // select Fern's name down to Rose's count, then sort by count from its menu
    const sheet = drawSheet();
    focusCell(sheet.container, "fern", "name");
    pressKey("ArrowRight", { shiftKey: true });
    pressKey("ArrowDown", { shiftKey: true });
    sheet.container
        .querySelector<HTMLElement>("th[data-column=count] [aria-haspopup=menu]")
        ?.click();
    flush();
    [...sheet.container.querySelectorAll<HTMLElement>("th[data-column=count] [role=menuitem]")]
        .find((each) => each.textContent === "Sort descending")
        ?.click();
    flush();
    const marked = sheet.container.querySelectorAll("td[data-in-range]").length;
    focusCell(sheet.container, "rose", "name");

    expect({ marked, copied: clipboard("copy") }).toEqual({ marked: 0, copied: "Rose" });
});

test("leave an edit the focus moves out of, writing it without taking the focus back", () => {
    // open Fern's name, type, then move the focus outside the table
    const sheet = drawSheet();
    const outside = document.createElement("button");
    document.body.append(outside);
    onTestFinished(() => outside.remove());
    focusCell(sheet.container, "fern", "name");
    pressKey("Enter");
    const field = document.activeElement;
    if (!(field instanceof HTMLInputElement)) {
        throw new TypeError("enter opened no field");
    }
    field.value = "Lady fern";
    outside.focus();
    flush();

    expect({ changes: sheet.changes, focused: document.activeElement === outside }).toEqual({
        changes: [[{ row: "fern", column: "name", value: "Lady fern" }]],
        focused: true,
    });
});

test("choose a select cell's option and commit it with Enter, the focus staying on the last row", () => {
    // open Mint's place, choose shade, and commit it
    const sheet = drawSheet();
    focusCell(sheet.container, "mint", "place");
    pressKey("Enter");
    const field = document.activeElement;
    if (!(field instanceof HTMLSelectElement)) {
        throw new TypeError("enter opened no choice");
    }
    field.value = "shade";
    field.dispatchEvent(new Event("change", { bubbles: true }));
    flush();
    pressKey("Enter");

    expect({
        changes: sheet.changes,
        focused: document.activeElement?.closest("td")?.dataset["column"],
    }).toEqual({ changes: [[{ row: "mint", column: "place", value: "shade" }]], focused: "place" });
});

test("leave rows in their place while a sort orders them", () => {
    // sort by name, then try to lift Fern by its handle
    const sheet = drawSheet();
    [...sheet.container.querySelectorAll<HTMLElement>("th button")]
        .find((button) => button.textContent === "Name")
        ?.click();
    flush();
    sheet.container.querySelector<HTMLElement>("[data-slot=data-table-row-handle]")?.focus();
    pressKey(" ");
    pressKey("ArrowDown");
    pressKey(" ");

    expect(sheet.moves).toEqual([]);
});

test("edit a cell whose row a sort moves, the focus stepping to the row below it before the edit", () => {
    // sort by count, Rose first, then set Rose's count below the others by typing over it
    const sheet = drawSheet();
    [...sheet.container.querySelectorAll<HTMLElement>("th button")]
        .find((button) => button.textContent === "Count")
        ?.click();
    flush();
    const before = [...sheet.container.querySelectorAll<HTMLElement>("tbody tr")].map(
        (row) => row.dataset["row"],
    );
    focusCell(sheet.container, "rose", "count");
    pressKey("1");
    pressKey("Enter");
    flush();
    const after = [...sheet.container.querySelectorAll<HTMLElement>("tbody tr")].map(
        (row) => row.dataset["row"],
    );

    // the descending sort moves Rose last, and the focus lands on Fern, the row below Rose before
    expect({
        before,
        after,
        focused: document.activeElement?.closest("tr")?.dataset["row"],
        stops: sheet.container.querySelectorAll("[tabindex='0']").length,
    }).toEqual({
        before: ["rose", "fern", "mint"],
        after: ["fern", "mint", "rose"],
        focused: "fern",
        stops: 1,
    });
});

test("show a read-only table's yes or no as a disabled box, and edit nothing", () => {
    // press Enter and Space on a read-only sheet
    const sheet = drawSheet({ isReadOnly: true });
    focusCell(sheet.container, "fern", "name");
    pressKey("Enter");
    const boxes = [
        ...sheet.container.querySelectorAll<HTMLElement>(
            "tbody td[data-column=isDry] [role=checkbox]",
        ),
    ].map((box) => [box.getAttribute("aria-checked"), box.hasAttribute("disabled")]);

    expect({ field: document.activeElement?.tagName, boxes }).toEqual({
        field: "TD",
        boxes: [
            ["false", true],
            ["true", true],
            ["false", true],
        ],
    });
});

test("drop a refused paste's marks once the focus leaves the grid, and once its column hides", () => {
    // refuse a word in Rose's count, leave the table, then hide Count from its menu
    const sheet = drawSheet();
    const outside = document.createElement("button");
    document.body.append(outside);
    onTestFinished(() => outside.remove());
    focusCell(sheet.container, "rose", "count");
    clipboard("paste", "seven");
    pressKey("Escape");
    outside.focus();
    flush();
    const cell = sheet.container.querySelector("tr[data-row=rose] td[data-column=count]");
    const left = [cell?.getAttribute("aria-invalid"), refusalOf(sheet.container)];
    chooseInHeaderMenu(sheet.container, "count", "Hide");

    expect({ left, hidden: refusalOf(sheet.container) }).toEqual({
        left: [null, ""],
        hidden: "",
    });
});

test("drop a refused paste's marks once an edit fixes it, at the grid's edge too", () => {
    // refuse a word in Mint's count, the last row, then type a number and press Enter
    const sheet = drawSheet();
    focusCell(sheet.container, "mint", "count");
    clipboard("paste", "seven");
    const field = sheet.container.querySelector<HTMLInputElement>("[data-editing] input");
    if (field === null) {
        throw new TypeError("the refused paste opened no field");
    }
    field.value = "7";
    pressKey("Enter");
    const cell = sheet.container.querySelector("tr[data-row=mint] td[data-column=count]");

    expect({
        changes: sheet.changes,
        invalid: cell?.getAttribute("aria-invalid"),
        said: refusalOf(sheet.container),
    }).toEqual({
        changes: [[{ row: "mint", column: "count", value: 7 }]],
        invalid: null,
        said: "",
    });
});

test("leave a paste of no text alone, and leave invalid text open as the focus leaves", () => {
    // paste nothing into Fern's name, then type a word into Rose's count and leave the table
    const sheet = drawSheet();
    const outside = document.createElement("button");
    document.body.append(outside);
    onTestFinished(() => outside.remove());
    focusCell(sheet.container, "fern", "name");
    clipboard("paste", "");
    focusCell(sheet.container, "rose", "count");
    pressKey("x");
    outside.focus();
    flush();
    const field = sheet.container.querySelector<HTMLInputElement>("[data-editing] input");
    const left = [field?.value, field?.getAttribute("aria-invalid"), document.activeElement];

    // move the focus to another cell, which hands it back to the field
    focusCell(sheet.container, "fern", "name");

    expect({ changes: sheet.changes, left, back: document.activeElement === field }).toEqual({
        changes: [],
        left: ["x", "true", outside],
        back: true,
    });
});

test("clear the selected cells with Delete, each to its column's empty value", () => {
    // select Fern's name and count, then delete
    const sheet = drawSheet();
    focusCell(sheet.container, "fern", "name");
    pressKey("ArrowRight", { shiftKey: true });
    pressKey("Delete");

    expect(sheet.changes).toEqual([
        [
            { row: "fern", column: "name", value: "" },
            { row: "fern", column: "count", value: null },
        ],
    ]);
});

test("open a column's menu with Alt and the down arrow, and a row's menu with Shift and F10", () => {
    // open Count's menu from its header, then Rose's row menu from its name
    const sheet = drawSheet();
    const head = sheet.container.querySelector<HTMLElement>("th[data-column=count]");
    head?.querySelector<HTMLElement>("[data-slot=button]")?.focus();
    pressKey("ArrowDown", { altKey: true });
    const columnMenu = head?.querySelector("[aria-haspopup=menu]")?.getAttribute("aria-expanded");
    pressKey("Escape");
    focusCell(sheet.container, "rose", "name");
    pressKey("F10", { shiftKey: true });
    const items = [
        ...document.querySelectorAll("[data-slot=data-table-row-menu] [role=menuitem]"),
    ].map((item) => item.textContent);

    expect({ columnMenu, items }).toEqual({ columnMenu: "true", items: ["Water"] });
});

test("open a row's menu from its grip while a sort orders the rows, the grip refusing only the drag", () => {
    // sort by Name, then click Rose's grip
    const sheet = drawSheet();
    sheet.container.querySelector<HTMLElement>("th[data-column=name] [data-slot=button]")?.click();
    flush();
    const grip = sheet.container.querySelector<HTMLElement>(
        "tr[data-row=rose] [data-slot=data-table-row-handle]",
    );
    grip?.click();
    flush();
    const items = [
        ...document.querySelectorAll("[data-slot=data-table-row-menu] [role=menuitem]"),
    ].map((item) => item.textContent);

    expect({ disabled: grip?.getAttribute("aria-disabled"), items }).toEqual({
        disabled: "true",
        items: ["Water"],
    });
});

test("offer moving a column left or right only where a movable column stands that way", () => {
    // open Name's menu, the first data column, and read its move items
    const sheet = drawSheet();
    sheet.container
        .querySelector<HTMLElement>("th[data-column=name] [aria-haspopup=menu]")
        ?.click();
    flush();
    const moves = [
        ...sheet.container.querySelectorAll<HTMLElement>("th[data-column=name] [role=menuitem]"),
    ]
        .filter((item) => item.textContent.startsWith("Move"))
        .map((item) => [item.textContent, item.getAttribute("aria-disabled")]);

    expect(moves).toEqual([
        ["Move left", "true"],
        ["Move right", null],
    ]);
});

test("select a block of cells by sweeping the pointer across them", () => {
    // press Fern's name and sweep to Rose's count, then let go
    const sheet = drawSheet();
    const cellOf = (row: string, column: string) =>
        sheet.container.querySelector(`tr[data-row=${row}] td[data-column=${column}]`);
    cellOf("fern", "name")?.dispatchEvent(
        new MouseEvent("mousedown", { bubbles: true, button: 0 }),
    );
    cellOf("rose", "count")?.dispatchEvent(new MouseEvent("mouseenter", { button: 0 }));
    document.dispatchEvent(new MouseEvent("mouseup", { bubbles: true }));
    flush();
    const marked = [...sheet.container.querySelectorAll<HTMLElement>("td[data-in-range]")].map(
        (cell) => `${String(cell.closest("tr")?.dataset["row"])}:${String(cell.dataset["column"])}`,
    );

    expect(marked).toEqual(["fern:name", "fern:count", "rose:name", "rose:count"]);
});
