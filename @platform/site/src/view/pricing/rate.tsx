import * as style from "@destack/style";
import { color, font, stroke, weight } from "@destack/theme/tokens.stylex";

import { allowance, machines, rates } from "../content/generated/pricing";
import { FigureLabel } from "../home/entry/entry";
import { lattice } from "../layout/lattice.stylex";
import { screen } from "../layout/screen.stylex";
import { convert, type Currency, money } from "./money";
import { typeScale, tracking } from "../layout/type.stylex";

/** The millions of CPU milliseconds in a CPU-hour, which CPU time is shown per. */
const CPU_HOUR_MILLIONS = 3.6;

/** A meter: what it measures and per which unit, and its price in US dollars. */
type Meter = { name: string; price: number };

/** The meters at their generated rates, grouped by resource. */
const meterGroups: readonly { name: string; meters: readonly Meter[] }[] = [
    {
        name: "Compute",
        meters: [
            { name: "Memory, per GB-hour", price: rates.memory },
            { name: "CPU time, per CPU-hour", price: rates.cpu * CPU_HOUR_MILLIONS },
            { name: "Requests, per million", price: rates.requests },
        ],
    },
    {
        name: "Database",
        meters: [
            { name: "Storage, per GB-month", price: rates.database },
            { name: "Writes, per million rows", price: rates.rowsWritten },
            { name: "Reads, per million rows", price: rates.rowsRead },
        ],
    },
    {
        name: "Files",
        meters: [
            { name: "Storage, per GB-month", price: rates.files },
            { name: "Writes, per million", price: rates.fileWrites },
            { name: "Reads, per million", price: rates.fileReads },
        ],
    },
    {
        name: "Network",
        meters: [
            { name: "Ingress", price: 0 },
            { name: "Egress", price: 0 },
        ],
    },
    {
        name: "Inference",
        meters: [{ name: "Model calls, per US dollar at list price", price: rates.inference }],
    },
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
            <FigureLabel number={3} isOpen={false} stacked="unit prices" destacked="unit prices" />

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
                    {meterGroups.map((group) => (
                        <tbody>
                            <tr>
                                <th colspan={2} scope="colgroup" {...style.attrs(styles.group)}>
                                    {group.name}
                                </th>
                            </tr>
                            {group.meters.map((meter) => (
                                <tr>
                                    <td {...style.attrs(styles.cell)}>{meter.name}</td>
                                    <td {...style.attrs(styles.cell, styles.end, styles.figure)}>
                                        {rate(meter.price)}
                                    </td>
                                </tr>
                            ))}
                        </tbody>
                    ))}
                </table>
                <p {...style.attrs(styles.note)}>
                    The Free plan includes {fee(allowance.usage)} of usage per person each month,
                    enough for {allowance.database} GB of database, {allowance.files} GB of files,{" "}
                    {allowance.compute} GB-hours of compute and a million requests.
                </p>
            </section>

            {/* the dedicated machines */}
            <section {...style.attrs(lattice.cell, styles.panel, styles.machines)}>
                <h3 {...style.attrs(styles.title)}>Dedicated machines</h3>
                <p {...style.attrs(styles.lede)}>
                    A machine of your own on Pro and Enterprise, billed by the second it runs.
                </p>
                <table {...style.attrs(styles.table)}>
                    <thead>
                        <tr>
                            <th {...style.attrs(styles.head)}>Machine</th>
                            <th {...style.attrs(styles.head, styles.end)}>Per minute</th>
                            <th {...style.attrs(styles.head, styles.end)}>Per hour</th>
                        </tr>
                    </thead>
                    <tbody>
                        {machines.map((machine) => (
                            <tr>
                                <td {...style.attrs(styles.cell)}>
                                    {sizeNames[machine.size]}
                                    <span {...style.attrs(styles.specs)}>
                                        {" "}
                                        · {machine.vcpus} vCPU · {machine.memory} GB ·{" "}
                                        {machine.disk} GB disk
                                    </span>
                                </td>
                                <td {...style.attrs(styles.cell, styles.end, styles.figure)}>
                                    {rate(machine.minutely)}
                                </td>
                                <td {...style.attrs(styles.cell, styles.end, styles.figure)}>
                                    {rate(machine.hourly)}
                                </td>
                            </tr>
                        ))}

                        {/* your own machine, on any plan */}
                        <tr>
                            <td {...style.attrs(styles.cell)}>
                                Your own machine
                                <span {...style.attrs(styles.specs)}> · any size, any plan</span>
                            </td>
                            <td {...style.attrs(styles.cell, styles.end, styles.figure)}>Free</td>
                            <td {...style.attrs(styles.cell, styles.end, styles.figure)}>Free</td>
                        </tr>
                    </tbody>
                </table>
                <p {...style.attrs(styles.note)}>
                    A machine bills only the seconds it is awake. On your own machine, you pay only
                    your provider.
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
        fontSize: typeScale.lead,
        fontWeight: weight.semibold,
        margin: 0,
    },
    lede: {
        color: color.mutedForeground,
        fontSize: typeScale.body,
        margin: 0,
        marginBlockEnd: "1.25rem",
    },
    table: {
        borderCollapse: "collapse",
        fontSize: typeScale.body,
        width: "100%",
    },
    head: {
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        color: color.mutedForeground,
        fontFamily: font.code,
        fontSize: typeScale.label,
        fontWeight: weight.regular,
        letterSpacing: tracking.label,
        paddingBlock: "0.75rem",
        textAlign: "start",
        textTransform: "uppercase",
        whiteSpace: "nowrap",
    },
    end: {
        paddingInlineStart: "1rem",
        textAlign: "end",
    },
    group: {
        color: color.mutedForeground,
        fontFamily: font.code,
        fontSize: typeScale.label,
        fontWeight: weight.regular,
        letterSpacing: tracking.label,
        paddingBlockStart: "1.5rem",
        paddingBlockEnd: "0.5rem",
        textAlign: "start",
        textTransform: "uppercase",
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
        fontSize: typeScale.small,
        lineHeight: 1.5,
        margin: 0,
        marginBlockStart: "1rem",
    },
});
