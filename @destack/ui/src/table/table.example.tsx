import { defineExample } from "@destack/package/declare";
import { useLocale } from "@destack/locale/solid";
import { For } from "solid-js";
import {
    Table,
    TableBody,
    TableCaption,
    TableCell,
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
    description: "this month's invoices with amounts in the reader's locale",
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
