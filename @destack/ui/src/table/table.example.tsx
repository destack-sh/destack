import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { For, useLocale } from "@destack/view";
import {
    Table,
    TableBody,
    TableCaption,
    TableCell,
    TableFooter,
    TableHead,
    TableHeader,
    TableRow,
} from "./table.tsx";

/** This month's invoices, in euros. */
const INVOICES = [
    { id: "INV-001", amount: 250 },
    { id: "INV-002", amount: 150 },
    { id: "INV-003", amount: 350 },
];

/** This month's invoices with amounts in the reader's locale. */
export const tableInvoices = defineExample({
    of: Table,
    name: "invoices",
    description: t`This month's invoices with amounts in the reader's locale.`,
    layout: "padded",
    render: () => {
        const locale = useLocale();

        return (
            <Table>
                <TableCaption>Invoices this month</TableCaption>
                <TableHeader>
                    <TableRow>
                        <TableHead scope="col">Invoice</TableHead>
                        <TableHead scope="col">Amount</TableHead>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    <For each={INVOICES}>
                        {(invoice) => (
                            <TableRow>
                                <TableCell>{invoice.id}</TableCell>
                                <TableCell>{locale.money(invoice.amount, "EUR")}</TableCell>
                            </TableRow>
                        )}
                    </For>
                </TableBody>
            </Table>
        );
    },
});

/** The invoices with their total in the table's footer. */
export const tableInvoiceTotal = defineExample({
    of: Table,
    name: "invoiceTotal",
    description: t`The invoices with their total in the table's footer.`,
    layout: "padded",
    render: () => {
        const locale = useLocale();
        const total = INVOICES.reduce((sum, invoice) => sum + invoice.amount, 0);

        return (
            <Table>
                <TableHeader>
                    <TableRow>
                        <TableHead scope="col">Invoice</TableHead>
                        <TableHead scope="col">Amount</TableHead>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    <For each={INVOICES}>
                        {(invoice) => (
                            <TableRow>
                                <TableCell>{invoice.id}</TableCell>
                                <TableCell>{locale.money(invoice.amount, "EUR")}</TableCell>
                            </TableRow>
                        )}
                    </For>
                </TableBody>
                <TableFooter>
                    <TableRow>
                        <TableHead scope="row">Total</TableHead>
                        <TableCell>{locale.money(total, "EUR")}</TableCell>
                    </TableRow>
                </TableFooter>
            </Table>
        );
    },
});
