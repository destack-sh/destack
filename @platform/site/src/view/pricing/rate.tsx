import * as style from "@destack/style";
import { color, font, stroke, weight } from "@destack/theme/tokens.stylex";

import { allowance, machines, rates } from "../content/generated/pricing";
import { FigureLabel } from "../home/entry/entry";
import { lattice } from "../layout/lattice.stylex";
import { screen } from "../layout/screen.stylex";
import { convert, type Currency, money } from "./money";

/** The millions of CPU milliseconds in a CPU-hour, which CPU time is shown per. */
const CPU_HOUR_MILLIONS = 3.6;

/** A meter: what it measures and per which unit, and its price in US dollars. */
type Meter = { name: string; price: number };

/** The meters at their generated rates: the apps', then the database's, files' and network's. */
const meters: readonly Meter[] = [
    { name: "Compute, per GB-hour", price: rates.memory },
    { name: "Requests, per million", price: rates.requests },
    { name: "CPU time, per CPU-hour", price: rates.cpu * CPU_HOUR_MILLIONS },
    { name: "Database storage, per GB-month", price: rates.database },
    { name: "Database writes, per million rows", price: rates.rowsWritten },
    { name: "Database reads, per million rows", price: rates.rowsRead },
    { name: "File storage, per GB-month", price: rates.files },
    { name: "File writes, per million", price: rates.fileWrites },
    { name: "File reads, per million", price: rates.fileReads },
    { name: "Egress", price: 0 },
];

/** The name each machine size is shown under. */
const sizeNames: Readonly<Record<(typeof machines)[number]["size"], string>> = {
    small: "Small",
    medium: "Medium",
    large: "Large",
    xlarge: "Extra large",
};

/** Show the rates as two panels: the usage meters, then the dedicated machines. */
export function Rates(properties: { currency: Currency }) {
    // price an amount in US dollars in the reader's currency, as a rate or as a whole fee
    const rate = (dollars: number) =>
        dollars === 0
            ? "Free"
            : money(convert(dollars, properties.currency), properties.currency, true);
    const fee = (dollars: number) =>
        money(convert(dollars, properties.currency), properties.currency);

    return (
        <>
            <FigureLabel number={2} isOpen={false} stacked="unit prices" destacked="unit prices" />

            {/* the usage meters */}
            <section {...style.attrs(lattice.cell, styles.panel, styles.usage)}>
                <h3 {...style.attrs(styles.title)}>Usage</h3>
                <p {...style.attrs(styles.lede)}>The same prices on every plan.</p>
                <table {...style.attrs(styles.table)}>
                    <thead>
                        <tr>
                            <th {...style.attrs(styles.head)}>Item</th>
                            <th {...style.attrs(styles.head, styles.end)}>Price</th>
                        </tr>
                    </thead>
                    <tbody>
                        {meters.map((meter) => (
                            <tr>
                                <td {...style.attrs(styles.cell)}>{meter.name}</td>
                                <td {...style.attrs(styles.cell, styles.end, styles.figure)}>
                                    {rate(meter.price)}
                                </td>
                            </tr>
                        ))}
                    </tbody>
                </table>
                <p {...style.attrs(styles.note)}>
                    The Free plan includes {allowance.compute} GB-hours of compute,{" "}
                    {allowance.database} GB of database, {allowance.files} GB of files and{" "}
                    {allowance.egress} GB of egress each month.
                </p>
            </section>

            {/* the dedicated machines */}
            <section {...style.attrs(lattice.cell, styles.panel, styles.machines)}>
                <h3 {...style.attrs(styles.title)}>Dedicated machines</h3>
                <p {...style.attrs(styles.lede)}>
                    A machine of your own on the Pro plan, billed by the hour it runs.
                </p>
                <table {...style.attrs(styles.table)}>
                    <thead>
                        <tr>
                            <th {...style.attrs(styles.head)}>Machine</th>
                            <th {...style.attrs(styles.head, styles.end)}>Per hour</th>
                            <th {...style.attrs(styles.head, styles.end)}>Per month</th>
                        </tr>
                    </thead>
                    <tbody>
                        {machines.map((machine) => (
                            <tr>
                                <td {...style.attrs(styles.cell)}>
                                    {sizeNames[machine.size]}
                                    <span {...style.attrs(styles.specs)}>
                                        {" "}
                                        · {machine.vcpus} vCPU · {machine.memory} GB
                                    </span>
                                </td>
                                <td {...style.attrs(styles.cell, styles.end, styles.figure)}>
                                    {rate(machine.hourly)}
                                </td>
                                <td {...style.attrs(styles.cell, styles.end, styles.figure)}>
                                    {fee(machine.monthly)}
                                </td>
                            </tr>
                        ))}
                    </tbody>
                </table>
                <p {...style.attrs(styles.note)}>
                    The monthly price is a cap: a machine running all month costs no more.
                </p>
            </section>
        </>
    );
}

/** The usage pricing styles. */
const styles = style.create({
    panel: {
        alignContent: "start",
        display: "grid",
        padding: "2rem",
        rowGap: "0.5rem",
    },
    usage: {
        gridColumn: { default: "1 / 7", [screen.belowDesktop]: "1 / -1" },
    },
    machines: {
        gridColumn: { default: "7 / -1", [screen.belowDesktop]: "1 / -1" },
    },
    title: {
        fontSize: "1.25rem",
        fontWeight: weight.semibold,
        letterSpacing: "-0.01em",
        margin: 0,
    },
    lede: {
        color: color.mutedForeground,
        fontSize: "0.9375rem",
        margin: 0,
        marginBlockEnd: "1.25rem",
    },
    table: {
        borderCollapse: "collapse",
        fontSize: "0.9375rem",
        width: "100%",
    },
    head: {
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        color: color.mutedForeground,
        fontFamily: font.code,
        fontSize: "0.6875rem",
        fontWeight: weight.regular,
        letterSpacing: "0.1em",
        paddingBlock: "0.75rem",
        textAlign: "start",
        textTransform: "uppercase",
        whiteSpace: "nowrap",
    },
    end: {
        paddingInlineStart: "1rem",
        textAlign: "end",
    },
    cell: {
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        paddingBlock: "0.875rem",
    },
    figure: {
        fontVariantNumeric: "tabular-nums",
        whiteSpace: "nowrap",
    },
    specs: {
        color: color.mutedForeground,
    },
    note: {
        color: color.mutedForeground,
        fontSize: "0.8125rem",
        lineHeight: 1.5,
        margin: 0,
        marginBlockStart: "1rem",
    },
});
