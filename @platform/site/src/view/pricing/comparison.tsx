import { media } from "@destack/style/media.stylex";
import { color, font, stroke, weight } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";

import { FigureLabel } from "../home/entry/entry";
import { Glyph } from "../home/figure/glyph";
import { frame } from "../layout/frame.stylex";
import { lattice } from "../layout/lattice.stylex";
import { typeScale, tracking } from "../layout/type.stylex";
import { plans } from "../content/generated/pricing";
import { Hint } from "./hint";
import { type Currency, money } from "./money";
import { type Audience, type Plan, audiencePlans } from "./plan";

/** What a plan gives on a row: included or not, or a short value. */
type Cell = boolean | string;

/** One row of the comparison: what it lists, what that means, and each plan's cell. */
type Row = { name: string; detail: string; cell: (plan: Plan, currency: Currency) => Cell };

/** The support each level gives, as the comparison lists it. */
const supportNames: Readonly<Record<Plan["support"], string>> = {
    community: "Community",
    email: "Email",
    priority: "Priority",
    sla: "SLA",
};

/** The rows of the comparison by group: the software, usage, hosting, support and contract. */
const groups: readonly { name: string; rows: readonly Row[] }[] = [
    {
        name: "Software",
        rows: [
            {
                name: "Standard apps and packages",
                detail: "The apps and packages Destack makes, with no seats or licences to pay.",
                cell: () => true,
            },
            {
                name: "Branching with previews",
                detail: "Every app branches, with a preview for each branch.",
                cell: () => true,
            },
            {
                name: "Roles for people, guests and agents",
                detail: "One set of roles for everyone who uses your apps.",
                cell: () => true,
            },
            { name: "Vault", detail: "Secrets, encrypted and shared.", cell: () => true },
            { name: "Audit log", detail: "Every change, by whom and when.", cell: () => true },
            { name: "Telemetry", detail: "Traces, logs and alerts.", cell: () => true },
            {
                name: "Bring your own AI",
                detail: "Use your subscriptions, your API keys or local models.",
                cell: () => true,
            },
            {
                name: "Managed AI",
                detail: "Use models through us, with no keys or accounts to manage, billed as usage.",
                cell: () => true,
            },
        ],
    },
    {
        name: "Usage",
        rows: [
            {
                name: "Usage included each month",
                detail: "Usage is billed at the unit prices below.",
                cell: (plan, currency) =>
                    !plan.isUncapped
                        ? "Free allowance"
                        : plan.isContract
                          ? `${money(plan.fees[currency], currency)}+`
                          : money(plan.fees[currency], currency),
            },
            {
                name: "Past the included usage",
                detail: "On Free your spaces slow down; on paid plans you pay as you go.",
                cell: (plan) => (plan.isUncapped ? "Pay as you go" : "Slows down"),
            },
            {
                name: "Retention",
                detail: "How long logs, traces and activity stay: their default spans, or as long as you like, counted as storage.",
                cell: (plan) => (plan.isCustomRetention ? "Custom" : "Default"),
            },
        ],
    },
    {
        name: "Hosting",
        rows: [
            {
                name: "Shared machines",
                detail: "Your spaces run on machines shared with other accounts.",
                cell: () => true,
            },
            {
                name: "Self-hosting",
                detail: "Run Destack on your own machines.",
                cell: () => true,
            },
            {
                name: "Tunneling",
                detail: "Reach your machines from anywhere.",
                cell: () => true,
            },
            {
                name: "Dedicated machines",
                detail: "A machine of your own, billed by the hour up to a monthly cap.",
                cell: (plan) => plan.isDedicated,
            },
            {
                name: "Custom domains",
                detail: "Serve your spaces and your profile under your own domain names.",
                cell: (plan) => plan.isCustomDomains,
            },
        ],
    },
    {
        name: "Support",
        rows: [
            {
                name: "Support",
                detail: "Community on Discord and GitHub, email, priority, or guaranteed response times.",
                cell: (plan) => supportNames[plan.support],
            },
            {
                name: "Early access",
                detail: "Try new apps and features before everyone else.",
                cell: (plan) => plan.isEarlyAccess,
            },
            {
                name: "Onboarding and migration",
                detail: "We move your data and tools over, and help build your first apps.",
                cell: (plan) => plan.isContract,
            },
        ],
    },
    {
        name: "Contract",
        rows: [
            {
                name: "Uptime SLA",
                detail: "An uptime we promise in your contract, with credits when we miss it.",
                cell: (plan) => plan.isContract,
            },
            {
                name: "Invoice billing",
                detail: "Pay by invoice and bank transfer, with purchase orders.",
                cell: (plan) => plan.isContract,
            },
            {
                name: "Data processing agreement",
                detail: "A signed agreement on how we process your data, for your privacy reviews.",
                cell: (plan) => plan.isContract,
            },
        ],
    },
];

/** Label the comparison as the second figure. */
export function ComparisonLabel() {
    return (
        <FigureLabel number={2} isOpen={false} stacked="compare plans" destacked="compare plans" />
    );
}

/** Compare an audience's plans row by row, grouped by what the rows are about. */
export function Comparison(properties: { audience: Audience; currency: Currency }) {
    // show the plans the audience is offered
    const shown = () =>
        plans.filter((plan) => audiencePlans[properties.audience].includes(plan.key));

    // list each group's rows some shown plan has, leaving out groups without any
    const listed = () =>
        groups.flatMap((group) => {
            const rows = group.rows.filter((row) =>
                shown().some((plan) => row.cell(plan, properties.currency) !== false),
            );

            return rows.length === 0 ? [] : [{ name: group.name, rows }];
        });

    return (
        <div {...style.attrs(lattice.cell, styles.frame)}>
            <table {...style.attrs(styles.table)}>
                {/* one column per plan */}
                <thead {...style.attrs(styles.section)}>
                    <tr {...style.attrs(styles.row)}>
                        <th {...style.attrs(styles.name, styles.corner)}>
                            <span {...style.attrs(styles.hidden)}>Feature</span>
                        </th>
                        {shown().map((plan) => (
                            <th {...style.attrs(styles.plan, styles.head)}>{plan.name}</th>
                        ))}
                    </tr>
                </thead>

                {/* each group, its title, then its rows, a row's name above its cells on phones */}
                {listed().map((group) => (
                    <tbody {...style.attrs(styles.section)}>
                        <tr {...style.attrs(styles.section)}>
                            <th
                                colspan={shown().length + 1}
                                scope="colgroup"
                                {...style.attrs(styles.group)}
                            >
                                {group.name}
                            </th>
                        </tr>
                        {group.rows.map((row) => (
                            <tr {...style.attrs(styles.row)}>
                                <th scope="row" {...style.attrs(styles.name)}>
                                    <Hint text={row.name} detail={row.detail} />
                                </th>
                                {shown().map((plan) => (
                                    <td {...style.attrs(styles.plan)}>
                                        <CellValue cell={row.cell(plan, properties.currency)} />
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
    if (properties.cell === true) {
        return (
            <span role="img" aria-label="Included" {...style.attrs(styles.check)}>
                <Glyph name="check" size={16} weight={2} />
            </span>
        );
    } else if (properties.cell === false) {
        return (
            <span role="img" aria-label="Not included" {...style.attrs(styles.dash)}>
                –
            </span>
        );
    } else {
        return <span {...style.attrs(styles.value)}>{properties.cell}</span>;
    }
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
    group: {
        color: color.mutedForeground,
        display: "block",
        fontFamily: font.code,
        fontSize: typeScale.label,
        fontWeight: weight.regular,
        letterSpacing: tracking.label,
        paddingBlockEnd: "0.75rem",
        paddingBlockStart: "2rem",
        textAlign: "start",
        textTransform: "uppercase",
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
