import { media } from "@destack/style/media.stylex";
import { color, font, stroke, weight } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";

import { FigureLabel } from "../home/entry/entry";
import { Glyph } from "../home/figure/glyph";
import { frame } from "../layout/frame.stylex";
import { lattice } from "../layout/lattice.stylex";
import { typeScale, tracking } from "../layout/type.stylex";
import { allowance, plans } from "../content/generated/pricing";
import { type Feature, features } from "./feature";
import { Hint } from "./hint";
import { convert, type Currency, money } from "./money";
import { type Plan } from "./plan";

/** What a plan gives on a row: included or not, or a short value. */
type Cell = boolean | string;

/** One row of the comparison: the feature it lists, and each plan's cell. */
type Row = { feature: Feature; cell: (plan: Plan, currency: Currency) => Cell };

/** The support each level gives, as the comparison lists it. */
const supportNames: Readonly<Record<Plan["support"], string>> = {
    community: "Community",
    email: "Email",
    priority: "Priority",
    sla: "SLA",
};

/** The rows of the comparison, by what they are about: usage, software, hosting, AI and support. */
const rows: readonly Row[] = [
    // usage
    {
        feature: features.included,
        cell: (plan, currency) =>
            plan.fees.USD === 0
                ? money(convert(allowance.usage, currency), currency, true)
                : money(plan.fees[currency], currency),
    },
    {
        feature: features.retention,
        cell: (plan) => (plan.isCustomRetention ? "Custom" : "Default"),
    },
    {
        feature: features.overage,
        cell: (plan) => (plan.isUncapped ? "Pay as you go" : "Capped"),
    },

    // software
    {
        feature: features.members,
        cell: (plan) => (plan.fees.USD === 0 ? "Up to 5" : "Any number"),
    },
    { feature: features.pooled, cell: () => true },
    { feature: features.guests, cell: () => true },
    { feature: features.agents, cell: () => true },
    { feature: features.apps, cell: () => true },
    { feature: features.integrations, cell: () => true },
    { feature: features.branches, cell: () => true },
    { feature: features.access, cell: () => true },
    { feature: features.secrets, cell: () => true },
    { feature: features.audit, cell: () => true },
    { feature: features.observability, cell: () => true },

    // hosting
    { feature: features.selfHosting, cell: () => true },
    { feature: features.cloudHosting, cell: () => true },
    { feature: features.residency, cell: () => true },
    { feature: features.tunnels, cell: () => true },
    { feature: features.domains, cell: (plan) => plan.isCustomDomains },
    { feature: features.dedicated, cell: (plan) => plan.isDedicated },

    // ai
    { feature: features.ownAi, cell: () => true },
    { feature: features.managedAi, cell: () => true },

    // support
    { feature: features.support, cell: (plan) => supportNames[plan.support] },
    { feature: features.early, cell: (plan) => plan.isEarlyAccess },
];

/** Label the comparison as the second figure. */
export function ComparisonLabel() {
    return (
        <FigureLabel number={2} isOpen={false} stacked="compare plans" destacked="compare plans" />
    );
}

/** The plans the comparison lists, the enterprise contract offered apart from them. */
const offered = plans.filter((plan) => !plan.isContract);

/** Compare the plans: what every plan includes as one strip, then the rows where they differ. */
export function Comparison(properties: { currency: Currency }) {
    // split the rows into those every plan includes and those that tell the plans apart
    const shared = rows.filter((row) =>
        offered.every((plan) => row.cell(plan, properties.currency) === true),
    );
    const differing = () => rows.filter((row) => !shared.includes(row));

    return (
        <div {...style.attrs(lattice.cell, styles.frame)}>
            {/* what every plan includes */}
            <p {...style.attrs(styles.lead)}>In every plan</p>
            <ul {...style.attrs(styles.shared)}>
                {shared.map((row) => (
                    <li {...style.attrs(styles.feature)}>
                        <span aria-hidden="true" {...style.attrs(styles.icon)}>
                            <Glyph name={row.feature.icon} size={16} weight={1.75} />
                        </span>
                        <Hint text={row.feature.name} detail={row.feature.detail} />
                    </li>
                ))}
            </ul>

            <table {...style.attrs(styles.table)}>
                {/* one column per plan, pinned while the rows scroll past */}
                <thead {...style.attrs(styles.section, styles.sticky)}>
                    <tr {...style.attrs(styles.row)}>
                        <th {...style.attrs(styles.name, styles.corner)}>
                            <span {...style.attrs(styles.hidden)}>Feature</span>
                        </th>
                        {offered.map((plan) => (
                            <th {...style.attrs(styles.plan, styles.head)}>{plan.name}</th>
                        ))}
                    </tr>
                </thead>

                {/* the rows that tell the plans apart, a row's name above its cells on phones */}
                <tbody {...style.attrs(styles.section)}>
                    {differing().map((row) => (
                        <tr {...style.attrs(styles.row)}>
                            <th scope="row" {...style.attrs(styles.name)}>
                                <Hint text={row.feature.name} detail={row.feature.detail} />
                            </th>
                            {offered.map((plan) => (
                                <td {...style.attrs(styles.plan)}>
                                    <CellValue cell={row.cell(plan, properties.currency)} />
                                </td>
                            ))}
                        </tr>
                    ))}
                </tbody>
            </table>
        </div>
    );
}

/** Draw a cell: a check where included, a dash where not, or its value. */
function CellValue(properties: { cell: Cell }) {
    // draw a check, a dash, or the value
    return (
        <>
            {properties.cell === true ? (
                <span role="img" aria-label="Included" {...style.attrs(styles.check)}>
                    <Glyph name="check" size={16} weight={2} />
                </span>
            ) : properties.cell === false ? (
                <span role="img" aria-label="Not included" {...style.attrs(styles.dash)}>
                    –
                </span>
            ) : (
                <span {...style.attrs(styles.value)}>{properties.cell}</span>
            )}
        </>
    );
}

/** The comparison styles. */
const styles = style.create({
    frame: {
        gridColumn: "1 / -1",
        paddingBlockEnd: "2.5rem",
        paddingBlockStart: "1.5rem",
        paddingInline: frame.inset,
    },
    table: {
        display: "block",
        fontSize: typeScale.body,
        width: "100%",
    },
    section: {
        display: "block",
    },
    sticky: {
        backgroundColor: color.background,
        position: "sticky",
        top: 0,
        zIndex: 1,
    },
    lead: {
        color: color.mutedForeground,
        fontFamily: font.code,
        fontSize: typeScale.label,
        letterSpacing: tracking.label,
        margin: 0,
        textTransform: "uppercase",
    },
    shared: {
        columnGap: "1.5rem",
        display: "grid",
        gridTemplateColumns: {
            default: "repeat(4, minmax(0, 1fr))",
            [media.maxMd]: "repeat(2, minmax(0, 1fr))",
        },
        listStyle: "none",
        margin: 0,
        marginBlockEnd: "2.5rem",
        marginBlockStart: "1.25rem",
        padding: 0,
        rowGap: "0.875rem",
    },
    feature: {
        alignItems: "center",
        display: "flex",
        fontSize: typeScale.body,
        gap: "0.75rem",
        minWidth: 0,
    },
    icon: {
        color: color.mutedForeground,
        display: "flex",
        flexShrink: 0,
    },
    row: {
        alignItems: "center",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        display: "grid",
        gridTemplateColumns: {
            default: "minmax(0, 8fr) repeat(4, minmax(0, 3fr))",
            [media.maxMd]: "repeat(4, minmax(0, 1fr))",
        },
        paddingBlock: "0.75rem",
        rowGap: "0.5rem",
    },
    name: {
        fontWeight: weight.regular,
        gridColumn: { default: "auto", [media.maxMd]: "1 / -1" },
        textAlign: "start",
    },
    corner: {
        display: { default: "block", [media.maxMd]: "none" },
    },
    head: {
        fontWeight: weight.semibold,
    },
    plan: {
        fontSize: { default: typeScale.body, [media.maxMd]: typeScale.small },
        minWidth: 0,
        paddingInline: "0.25rem",
        textAlign: "center",
    },
    hidden: {
        clipPath: "inset(50%)",
        height: "1px",
        overflow: "hidden",
        position: "absolute",
        whiteSpace: "nowrap",
        width: "1px",
    },
    check: {
        color: color.primary,
        display: "inline-flex",
        verticalAlign: "middle",
    },
    dash: {
        color: color.mutedForeground,
    },
    value: {
        fontVariantNumeric: "tabular-nums",
    },
});
