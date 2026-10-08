import { expect, onTestFinished, test } from "@destack/test";
import {
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
} from "@tanstack/table-core";
import { Errored, flush, type JSX } from "@destack/view";
import { render, stubPopovers } from "@destack/view/test";
import {
    createTable,
    DataTable,
    DataTableColumnHeader,
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
        header: (context) => <DataTableColumnHeader column={context.column} title="Customer" />,
    }),
    COLUMN.accessor("amount", {
        header: (context) => <DataTableColumnHeader column={context.column} title="Amount" />,
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

test("page through rows ten at a time", () => {
    const container = drawInvoices();
    flush();
    const first = shownIds(container);
    const page = container.querySelector("[data-slot=data-table-pagination]")?.textContent;
    step(container, "Go to the next page");
    expect([first.length, page?.includes("Page 1 of 2"), shownIds(container)]).toEqual([
        10,
        true,
        ["INV011", "INV012"],
    ]);
});

test("sort by a column ascending, descending and back to the rows' own order, marking it with aria-sort", () => {
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
        container.querySelectorAll("tbody tr[data-state=selected]").length,
        container.querySelector("[data-slot=data-table-pagination] span")?.textContent,
    ]).toEqual(["mixed", 10, "10 of 12 rows selected"]);
});

test("move the focus between cells with the arrow keys, keeping one tab stop", () => {
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
    return <>{String(useDataTableState()())}</>;
}

/** Press a key on the focused element, let a scrolled table render, and read the title of the focused cell's row. */
async function pressAndRead(key: string, modifiers: KeyboardEventInit = {}): Promise<string> {
    // press the key and wait for the rows scrolled into view
    document.activeElement?.dispatchEvent(
        new KeyboardEvent("keydown", { key, bubbles: true, ...modifiers }),
    );
    flush();
    await new Promise((resolve) => {
        setTimeout(resolve);
    });
    flush();

    return (
        document.activeElement?.closest("tr")?.querySelector("[data-column=title]")?.textContent ??
        ""
    );
}

test("move the focus by key onto rows a virtualized table renders only once they scroll into view", async () => {
    // give the scrolling table a box of five rows, which the test DOM does not lay out
    const height = Object.getOwnPropertyDescriptor(HTMLElement.prototype, "offsetHeight");
    Object.defineProperty(HTMLElement.prototype, "offsetHeight", {
        configurable: true,
        get(this: HTMLElement) {
            return this.dataset["slot"] === "data-table" ? 200 : 0;
        },
    });
    onTestFinished(() => {
        if (height !== undefined) {
            Object.defineProperty(HTMLElement.prototype, "offsetHeight", height);
        }
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
