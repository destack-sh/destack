import { expect, test } from "@destack/test";
import { flush } from "solid-js";
import { DataTable, type DataTableColumn } from "./index.ts";
import { draw } from "@destack/view/test";

/** An invoice the table lists. */
interface Invoice {
    /** The invoice number. */
    readonly id: string;
    /** The customer billed. */
    readonly customer: string;
    /** The amount in cents. */
    readonly amount: number;
}

/** Twelve invoices, two pages at the default size. */
const INVOICES: readonly Invoice[] = Array.from({ length: 12 }, (_, index) => ({
    id: `INV${String(index + 1).padStart(3, "0")}`,
    customer: index % 2 === 0 ? `Ada ${index}` : `Grace ${index}`,
    amount: (12 - index) * 1000,
}));

/** The columns: a searchable customer and a sortable amount. */
const COLUMNS: readonly DataTableColumn<Invoice>[] = [
    { id: "id", header: "Invoice", cell: (row) => row.id },
    {
        id: "customer",
        header: "Customer",
        cell: (row) => row.customer,
        filterValue: (row) => row.customer,
        sortValue: (row) => row.customer,
    },
    {
        id: "amount",
        header: "Amount",
        cell: (row) => String(row.amount / 100),
        sortValue: (row) => row.amount,
    },
];

/** Render the invoices, collecting selection changes. */
function drawInvoices(changes: (readonly string[])[]): HTMLElement {
    return draw(() => (
        <DataTable
            rows={INVOICES}
            columns={COLUMNS}
            rowId={(row) => row.id}
            isSelectable
            onSelectionChange={(ids) => changes.push(ids)}
        />
    ));
}

/** Read the invoice numbers of the shown rows. */
function shownIds(container: Element): string[] {
    return [...container.querySelectorAll("tbody tr")].map(
        (row) => row.querySelectorAll("td")[1]?.textContent ?? "",
    );
}

/** Read the footer's texts. */
function footer(container: Element): string[] {
    return [...container.querySelectorAll("[data-slot=data-table-footer] span")].map(
        (span) => span.textContent ?? "",
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

test("page through rows ten at a time", () => {
    const container = drawInvoices([]);
    const first = [shownIds(container).length, footer(container)];
    container
        .querySelectorAll<HTMLButtonElement>("[data-slot=data-table-footer] button")[1]
        ?.click();
    flush();
    expect([first, shownIds(container), footer(container)]).toEqual([
        [10, ["0 of 12 rows selected", "Page 1 of 2"]],
        ["INV011", "INV012"],
        ["0 of 12 rows selected", "Page 2 of 2"],
    ]);
});

test("sort by a column ascending, descending and back to the rows' own order, marking it with aria-sort", () => {
    const container = drawInvoices([]);
    sortBy(container, "Amount");
    const ascending = [
        shownIds(container)[0],
        container.querySelector("[aria-sort]")?.getAttribute("aria-sort"),
    ];
    sortBy(container, "Amount");
    const descending = [
        shownIds(container)[0],
        container.querySelector("[aria-sort]")?.getAttribute("aria-sort"),
    ];
    sortBy(container, "Amount");
    expect([
        ascending,
        descending,
        [shownIds(container)[0], container.querySelector("[aria-sort]")],
    ]).toEqual([
        ["INV012", "ascending"],
        ["INV001", "descending"],
        ["INV001", null],
    ]);
});

test("filter rows by their searchable cells, showing a message when none match", () => {
    const container = drawInvoices([]);
    const input = container.querySelector("input[type=search]");
    if (input instanceof HTMLInputElement) {
        input.value = "grace";
        input.dispatchEvent(new InputEvent("input", { bubbles: true }));
    }
    flush();
    const graces = [shownIds(container).length, footer(container)[1]];
    if (input instanceof HTMLInputElement) {
        input.value = "zebra";
        input.dispatchEvent(new InputEvent("input", { bubbles: true }));
    }
    flush();
    expect([graces, container.querySelector("tbody")?.textContent]).toEqual([
        [6, "Page 1 of 1"],
        "No results",
    ]);
});

test("select rows and the whole page, the header box showing a partial selection", () => {
    const changes: (readonly string[])[] = [];
    const container = drawInvoices(changes);
    const boxes = () => [...container.querySelectorAll<HTMLInputElement>("input[type=checkbox]")];
    boxes()[1]?.click();
    flush();
    const partial = [boxes()[0]?.indeterminate, boxes()[0]?.checked, footer(container)[0]];
    boxes()[0]?.click();
    flush();
    expect([
        partial,
        footer(container)[0],
        changes.at(-1)?.length,
        container.querySelectorAll("tr[data-state=selected]").length,
    ]).toEqual([[true, false, "1 of 12 rows selected"], "10 of 12 rows selected", 10, 10]);
});
