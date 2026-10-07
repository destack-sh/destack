import { expect, test } from "@destack/test";
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
import { flush } from "@destack/view";
import { draw } from "@destack/view/test";
import {
    createTable,
    DataTable,
    DataTableColumnHeader,
    DataTableFilter,
    DataTablePagination,
    selectionColumn,
} from "./index.ts";
import { dataTableNotesGrouped, dataTableTasksNested } from "./data-table.example.tsx";

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
    return draw(() => {
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
    });
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
    container.querySelector<HTMLInputElement>("tbody input[type=checkbox]")?.click();
    flush();
    const header = container.querySelector<HTMLInputElement>("thead input[type=checkbox]");
    const partial = header?.indeterminate;
    header?.click();
    flush();
    expect([
        partial,
        container.querySelectorAll("tbody tr[data-state=selected]").length,
        container.querySelector("[data-slot=data-table-pagination] span")?.textContent,
    ]).toEqual([true, 10, "10 of 12 rows selected"]);
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
    const container = draw(() => dataTableNotesGrouped.render?.());
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
    const container = draw(() => dataTableTasksNested.render?.());
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
