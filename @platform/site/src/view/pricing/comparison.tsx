import { media } from "@destack/style/media.stylex";
import { color, font, stroke, weight } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";

import { FigureLabel } from "../home/entry/entry";
import { Glyph } from "../home/figure/glyph";
import { measure } from "../layout/measure.stylex";
import { lattice, paper } from "../layout/lattice.stylex";
import { typeScale, tracking } from "../layout/type.stylex";
import { plans } from "../content/generated/pricing";
import { type Feature, features } from "./feature";
import { FeatureHint } from "./hint";
import { type Currency, money } from "./money";
import { type Audience, type Plan } from "./plan";

/** What a plan gives on a row: included or not, or a short value. */
type Cell = boolean | string;

/** One row of the comparison: the feature it lists, and each plan's cell. */
type Row = {
    feature: Feature;
    cell: (plan: Plan, currency: Currency, audience: Audience) => Cell;
};

/** The support each level gives, as the comparison lists it. */
const supportNames: Readonly<Record<Plan["support"], string>> = {
    email: "Email",
    priority: "Priority",
    sla: "SLA",
};

/** The groups of the comparison, each a heading over its rows. */
const groups: readonly { name: string; rows: readonly Row[] }[] = [
    {
        name: "Usage",
        rows: [
            {
                feature: features.included,
                cell: (plan, currency) =>
                    plan.fees.USD === 0
                        ? "Limited"
                        : plan.isContract
                          ? `From ${money(plan.fees[currency], currency)}`
                          : money(plan.fees[currency], currency),
            },
            { feature: features.uncapped, cell: (plan) => plan.isUncapped },
            {
                feature: features.retention,
                cell: (plan) => (plan.isCustomRetention ? "Custom" : "Default"),
            },
            {
                feature: features.pooled,
                cell: (plan, _currency, audience) => plan.isContract || audience === "organisation",
            },
        ],
    },
    {
        name: "Software",
        rows: [
            { feature: features.apps, cell: () => true },
            { feature: features.members, cell: () => true },
            { feature: features.agents, cell: () => true },
            { feature: features.integrations, cell: () => true },
            { feature: features.branches, cell: () => true },
            { feature: features.access, cell: () => true },
            { feature: features.secrets, cell: () => true },
            { feature: features.audit, cell: () => true },
            { feature: features.observability, cell: () => true },
            { feature: features.early, cell: (plan) => plan.isEarlyAccess },
        ],
    },
    {
        name: "Hosting",
        rows: [
            { feature: features.selfHosting, cell: () => true },
            { feature: features.cloudHosting, cell: () => true },
            { feature: features.residency, cell: () => true },
            { feature: features.tunnels, cell: () => true },
            { feature: features.domains, cell: (plan) => plan.isCustomDomains },
            { feature: features.dedicated, cell: (plan) => plan.isDedicated },
            { feature: features.uptime, cell: (plan) => plan.isContract },
        ],
    },
    {
        name: "AI",
        rows: [
            { feature: features.ownAi, cell: () => true },
            { feature: features.managedAi, cell: () => true },
        ],
    },
    {
        name: "Support",
        rows: [
            { feature: features.support, cell: (plan) => supportNames[plan.support] },
            { feature: features.engineer, cell: (plan) => plan.isContract },
            { feature: features.invoice, cell: (plan) => plan.isContract },
            { feature: features.agreement, cell: (plan) => plan.isContract },
        ],
    },
];

/** Label the comparison as the second figure. */
export function ComparisonLabel() {
    return (
        <FigureLabel number={2} isOpen={false} stacked="compare plans" destacked="compare plans" />
    );
}

/** Compare the plans group by group, the same rows and columns for people and organisations. */
export function Comparison(properties: { audience: Audience; currency: Currency }) {
    return (
        <div {...style.attrs(lattice.cell, styles.frame)}>
            <table {...style.attrs(styles.table)}>
                {/* one column per plan, pinned while the rows scroll past */}
                <thead {...style.attrs(styles.section, paper.patch, styles.sticky)}>
                    <tr {...style.attrs(styles.row)}>
                        <th {...style.attrs(styles.name, styles.corner)}>
                            <span {...style.attrs(styles.hidden)}>Feature</span>
                        </th>
                        {plans.map((plan) => (
                            <th {...style.attrs(styles.plan, styles.head)}>{plan.name}</th>
                        ))}
                    </tr>
                </thead>

                {/* each group under its heading, a row's name above its cells on phones */}
                {groups.map((group) => (
                    <tbody {...style.attrs(styles.section)}>
                        <tr>
                            <th scope="rowgroup" {...style.attrs(styles.group)}>
                                {group.name}
                            </th>
                        </tr>
                        {group.rows.map((row) => (
                            <tr {...style.attrs(styles.row)}>
                                <th scope="row" {...style.attrs(styles.name)}>
                                    <span {...style.attrs(styles.feature)}>
                                        <span aria-hidden="true" {...style.attrs(styles.icon)}>
                                            <Glyph
                                                name={row.feature.icon}
                                                size={16}
                                                weight={1.75}
                                            />
                                        </span>
                                        <FeatureHint feature={row.feature} />
                                    </span>
                                </th>
                                {plans.map((plan) => (
                                    <td {...style.attrs(styles.plan)}>
                                        <CellValue
                                            cell={row.cell(
                                                plan,
                                                properties.currency,
                                                properties.audience,
                                            )}
                                        />
                                    </td>
                                ))}
                            </tr>
                        ))}
                    </tbody>
                ))}
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
        paddingBlockEnd: "2rem",
        paddingBlockStart: "0.5rem",
        paddingInline: measure.inset,
    },
    table: {
        display: "block",
        fontSize: typeScale.body,
        // scroll the rows under the pinned head on the table's own layer
        willChange: "transform",
        width: "100%",
    },
    section: {
        display: "block",
    },
    sticky: {
        position: "sticky",
        top: 0,
        zIndex: 1,
    },
    group: {
        color: color.mutedForeground,
        display: "block",
        fontFamily: font.code,
        fontSize: typeScale.label,
        fontWeight: weight.regular,
        letterSpacing: tracking.label,
        paddingBlockEnd: "0.25rem",
        paddingBlockStart: "2rem",
        textAlign: "start",
        textTransform: "uppercase",
    },
    feature: {
        alignItems: "center",
        display: "flex",
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
            default: "minmax(0, 7fr) repeat(5, minmax(0, 3fr))",
            [media.maxMd]: "repeat(5, minmax(0, 1fr))",
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
