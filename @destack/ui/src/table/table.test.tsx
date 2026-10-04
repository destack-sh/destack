import { expect, test } from "@destack/test";
import {
    Table,
    TableBody,
    TableCaption,
    TableCell,
    TableFooter,
    TableHead,
    TableHeader,
    TableRow,
} from "./index.ts";
import { draw, markup } from "@destack/view/test";

test("render a native table inside a container that scrolls sideways", () => {
    const container = draw(() => (
        <Table>
            <TableCaption>Invoices this month</TableCaption>
            <TableHeader>
                <TableRow>
                    <TableHead scope="col">Invoice</TableHead>
                    <TableHead scope="col">Amount</TableHead>
                </TableRow>
            </TableHeader>
            <TableBody>
                <TableRow>
                    <TableCell>INV001</TableCell>
                    <TableCell>€250.00</TableCell>
                </TableRow>
            </TableBody>
            <TableFooter>
                <TableRow>
                    <TableCell>Total</TableCell>
                    <TableCell>€250.00</TableCell>
                </TableRow>
            </TableFooter>
        </Table>
    ));
    expect(markup(container)).toBe(
        '<div data-slot="table-container"><table data-slot="table">' +
            '<caption data-slot="table-caption">Invoices this month</caption>' +
            '<thead data-slot="table-header"><tr data-slot="table-row"><th data-slot="table-head" scope="col">Invoice</th><th data-slot="table-head" scope="col">Amount</th></tr></thead>' +
            '<tbody data-slot="table-body"><tr data-slot="table-row"><td data-slot="table-cell">INV001</td><td data-slot="table-cell">€250.00</td></tr></tbody>' +
            '<tfoot data-slot="table-footer"><tr data-slot="table-row"><td data-slot="table-cell">Total</td><td data-slot="table-cell">€250.00</td></tr></tfoot></table></div>',
    );
});
