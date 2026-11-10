import * as style from "@destack/style";
import { color, font, stroke, weight } from "@destack/theme/tokens.stylex";

import { allowance, machines, units } from "../content/generated/pricing";
import { FigureLabel } from "../home/entry/entry";
import { lattice } from "../layout/lattice.stylex";
import { screen } from "../layout/screen.stylex";
import { convert, type Currency, money } from "./money";
import { typeScale, tracking } from "../layout/type.stylex";

/** A unit the page lists, with its price and what it costs Destack to run. */
type Unit = (typeof units)[number];

/** The units by resource, in the order the generator lists them. */
const resources: readonly { name: string; units: readonly Unit[] }[] = [
    ...Map.groupBy(units, (unit) => unit.resource),
].map(([name, grouped]) => ({ name, units: grouped }));

/** The name each machine size is shown under. */
const sizeNames: Readonly<Record<(typeof machines)[number]["size"], string>> = {
    small: "Small",
    medium: "Medium",
    large: "Large",
    xlarge: "Extra large",
};

/** Show the rates as two panels: the usage meters, then the dedicated machines. */
export function Rates(properties: { currency: Currency }) {
    // price an amount in US dollars in the reader's currency as a rate
    const rate = (dollars: number) =>
        dollars === 0
            ? "Free"
            : money(convert(dollars, properties.currency), properties.currency, true);

    return (
        <>
            <FigureLabel number={3} isOpen={false} stacked="unit prices" destacked="unit prices" />

            {/* the usage meters */}
            <section {...style.attrs(lattice.cell, styles.panel, styles.usage)}>
                <h3 {...style.attrs(styles.title)}>Usage</h3>
                <p {...style.attrs(styles.lede)}>The same prices on every plan.</p>

                {/* each resource's units and their prices */}
                {resources.map((resource) => (
                    <section {...style.attrs(styles.resource)}>
                        <h4 {...style.attrs(styles.group)}>{resource.name}</h4>
                        {resource.units.map((unit) => (
                            <div {...style.attrs(styles.unit)}>
                                <span>
                                    {unit.name}
                                    <span {...style.attrs(styles.specs)}> · per {unit.per}</span>
                                </span>
                                <span {...style.attrs(styles.figure, styles.end)}>
                                    {rate(unit.price)}
                                </span>
                            </div>
                        ))}
                    </section>
                ))}
                <p {...style.attrs(styles.note)}>
                    The Free plan includes enough usage each month for {allowance.database} GB of
                    database, {allowance.files} GB of files, {allowance.compute} GB-hours of compute
                    and a million requests.
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
        margin: 0,
        paddingBlockEnd: "0.5rem",
        paddingBlockStart: "1.25rem",
        textAlign: "start",
        textTransform: "uppercase",
    },
    resource: {
        display: "grid",
    },
    unit: {
        alignItems: "center",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        columnGap: "1rem",
        display: "grid",
        fontSize: typeScale.body,
        gridTemplateColumns: "minmax(0, 1fr) auto",
        paddingBlock: "0.75rem",
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
