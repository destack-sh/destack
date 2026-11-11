import { present } from "@destack/schema";
import { media } from "@destack/style/media.stylex";
import * as style from "@destack/style";
import { color, font, stroke, weight } from "@destack/theme/tokens.stylex";

import { machines, units } from "../content/generated/pricing";
import { FigureLabel } from "../home/entry/entry";
import { Glyph, type GlyphName } from "../home/figure/glyph";
import { frame } from "../layout/frame.stylex";
import { lattice } from "../layout/lattice.stylex";
import { typeScale, tracking } from "../layout/type.stylex";
import { FairUseHint } from "./hint";
import { convert, type Currency, money } from "./money";

/** A unit the page lists, with its price in US dollars. */
type Unit = (typeof units)[number];

/** The resources the cards list, one a column, each with its icon. */
const RESOURCES: readonly { name: Unit["resource"]; icon: GlyphName }[] = [
    { name: "Compute", icon: "chip" },
    { name: "Database", icon: "database" },
    { name: "Files", icon: "folder" },
    { name: "Network", icon: "transfer" },
];

/** The name each machine size is shown under. */
const sizeNames: Readonly<Record<(typeof machines)[number]["size"], string>> = {
    small: "Small",
    medium: "Medium",
    large: "Large",
    xlarge: "Extra large",
};

/** Show the rates: a card per resource, then the dedicated machines across the frame. */
export function Rates(properties: { currency: Currency }) {
    // price an amount in US dollars in the reader's currency as a rate
    const rate = (dollars: number) =>
        dollars === 0
            ? "Free"
            : money(convert(dollars, properties.currency), properties.currency, true);

    return (
        <>
            <FigureLabel number={3} isOpen={false} stacked="unit prices" destacked="unit prices">
                <span {...style.attrs(styles.caption)}>The same on every plan</span>
            </FigureLabel>

            {/* a card per resource */}
            {RESOURCES.map((resource) => (
                <ResourceCard resource={resource} rate={rate} />
            ))}

            {/* the dedicated machines */}
            <section {...style.attrs(lattice.cell, styles.band)}>
                <div {...style.attrs(styles.intro)}>
                    <h3 {...style.attrs(styles.title)}>
                        <span aria-hidden="true" {...style.attrs(styles.icon)}>
                            <Glyph name="server" size={18} weight={1.75} />
                        </span>
                        Dedicated machines
                    </h3>
                    <p {...style.attrs(styles.lede)}>
                        A machine of your own on Pro and Enterprise, billed by the second it is
                        awake.
                    </p>
                </div>
                <table {...style.attrs(styles.table)}>
                    <thead>
                        <tr>
                            <th {...style.attrs(styles.head)}>Machine</th>
                            <th {...style.attrs(styles.head, styles.end)}>vCPU</th>
                            <th {...style.attrs(styles.head, styles.end)}>Memory</th>
                            <th {...style.attrs(styles.head, styles.end)}>Disk</th>
                            <th {...style.attrs(styles.head, styles.end)}>Per hour</th>
                        </tr>
                    </thead>
                    <tbody>
                        {machines.map((machine) => (
                            <tr>
                                <td {...style.attrs(styles.cell)}>{sizeNames[machine.size]}</td>
                                <td {...style.attrs(styles.cell, styles.end, styles.figure)}>
                                    {machine.vcpus}
                                </td>
                                <td {...style.attrs(styles.cell, styles.end, styles.figure)}>
                                    {machine.memory} GB
                                </td>
                                <td {...style.attrs(styles.cell, styles.end, styles.figure)}>
                                    {machine.disk} GB
                                </td>
                                <td {...style.attrs(styles.cell, styles.end, styles.figure)}>
                                    {rate(machine.hourly)}
                                </td>
                            </tr>
                        ))}

                        {/* your own machine, on any plan */}
                        <tr>
                            <td {...style.attrs(styles.cell)}>Your own machine</td>
                            <td {...style.attrs(styles.cell, styles.end, styles.muted)}>Any</td>
                            <td {...style.attrs(styles.cell, styles.end, styles.muted)}>Any</td>
                            <td {...style.attrs(styles.cell, styles.end, styles.muted)}>Any</td>
                            <td {...style.attrs(styles.cell, styles.end, styles.figure)}>Free</td>
                        </tr>
                    </tbody>
                </table>
            </section>
        </>
    );
}

/** Draw a resource's card: its first unit as the headline, then the rest, or Free when all are. */
function ResourceCard(properties: {
    resource: (typeof RESOURCES)[number];
    rate: (dollars: number) => string;
}) {
    // read the resource's units, the first one leading
    const listed = units.filter((unit) => unit.resource === properties.resource.name);
    const first = present(listed[0], `the first ${properties.resource.name} unit`);
    const rest = listed.slice(1);
    const isFree = listed.every((unit) => unit.price === 0);

    // list the units of a free resource in one phrase, such as "Ingress and egress"
    const names = listed.map((unit, index) => (index === 0 ? unit.name : unit.name.toLowerCase()));

    return (
        <section {...style.attrs(lattice.cell, styles.card)}>
            <h3 {...style.attrs(styles.title)}>
                <span aria-hidden="true" {...style.attrs(styles.icon)}>
                    <Glyph name={properties.resource.icon} size={18} weight={1.75} />
                </span>
                {properties.resource.name}
            </h3>

            {/* free for all of a free resource, within fair use */}
            {isFree ? (
                <div {...style.attrs(styles.lead)}>
                    <p {...style.attrs(styles.fee)}>
                        <span {...style.attrs(styles.amount)}>Free</span>
                    </p>
                    <p {...style.attrs(styles.per)}>
                        <FairUseHint
                            text={`${names.join(" and ")}, any amount`}
                            detail="Data in and out of your spaces costs nothing."
                        />
                    </p>
                </div>
            ) : (
                <>
                    {/* the first unit as the headline */}
                    <div {...style.attrs(styles.lead)}>
                        <p {...style.attrs(styles.fee)}>
                            <span {...style.attrs(styles.amount)}>
                                {properties.rate(first.price)}
                            </span>
                            <span {...style.attrs(styles.period)}>/ {first.per}</span>
                        </p>
                        <p {...style.attrs(styles.per)}>{first.name}</p>
                    </div>

                    {/* the other units */}
                    {rest.map((unit) => (
                        <div {...style.attrs(styles.unit)}>
                            <span>{unit.name}</span>
                            <span {...style.attrs(styles.figure)}>
                                {properties.rate(unit.price)}
                            </span>
                            <span {...style.attrs(styles.per)}>per {unit.per}</span>
                        </div>
                    ))}
                </>
            )}
        </section>
    );
}

/** The usage pricing styles. */
const styles = style.create({
    caption: {
        color: color.mutedForeground,
        fontFamily: font.code,
        fontSize: typeScale.label,
        letterSpacing: tracking.label,
        textTransform: "uppercase",
        whiteSpace: "nowrap",
    },
    card: {
        alignContent: "start",
        display: "grid",
        gridColumn: { default: "span 3", [media.maxLg]: "span 6", [media.maxMd]: "1 / -1" },
        padding: `2rem ${frame.inset}`,
        rowGap: "0.25rem",
    },
    title: {
        alignItems: "center",
        display: "flex",
        fontSize: typeScale.lead,
        fontWeight: weight.semibold,
        gap: "0.625rem",
        lineHeight: 1.15,
        margin: 0,
    },
    icon: {
        color: color.primary,
        display: "flex",
    },
    lead: {
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        display: "grid",
        paddingBlockEnd: "1rem",
        paddingBlockStart: "1.25rem",
        rowGap: "0.5rem",
    },
    fee: {
        alignItems: "baseline",
        display: "flex",
        gap: "0.375rem",
        margin: 0,
    },
    amount: {
        fontSize: typeScale.title,
        fontVariantNumeric: "tabular-nums",
        fontWeight: weight.medium,
        letterSpacing: tracking.display,
        lineHeight: 1,
    },
    period: {
        color: color.mutedForeground,
        fontSize: typeScale.body,
        whiteSpace: "nowrap",
    },
    unit: {
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        columnGap: "1rem",
        display: "grid",
        fontSize: typeScale.body,
        gridTemplateColumns: "minmax(0, 1fr) auto",
        paddingBlock: "0.75rem",
        rowGap: "0.125rem",
    },
    per: {
        color: color.mutedForeground,
        fontSize: typeScale.small,
        gridColumn: "1 / -1",
        margin: 0,
    },
    band: {
        columnGap: { default: stroke.border, [media.maxLg]: 0 },
        display: "grid",
        gridColumn: "1 / -1",
        gridTemplateColumns: { default: "subgrid", [media.maxLg]: "minmax(0, 1fr)" },
        paddingBlock: "2rem",
        rowGap: "1.5rem",
    },
    intro: {
        alignContent: "start",
        display: "grid",
        gridColumn: { default: "span 3", [media.maxLg]: "1 / -1" },
        paddingInline: frame.inset,
        rowGap: "0.5rem",
    },
    lede: {
        color: color.mutedForeground,
        fontSize: typeScale.body,
        lineHeight: 1.5,
        margin: 0,
    },
    table: {
        borderCollapse: "collapse",
        fontSize: typeScale.body,
        gridColumn: { default: "span 9", [media.maxLg]: "1 / -1" },
        marginInline: frame.inset,
        width: `calc(100% - 2 * ${frame.inset})`,
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
        paddingBlockEnd: "0.75rem",
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
    muted: {
        color: color.mutedForeground,
    },
});
