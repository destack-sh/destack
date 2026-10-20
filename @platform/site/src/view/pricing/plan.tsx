import { media } from "@destack/style/media.stylex";
import { color, font, shadow, stroke, weight } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import { present } from "@destack/schema";
import { buttonVariants } from "@destack/ui/button";
import { For } from "@destack/view";

import { allowance, currencies, plans } from "../content/generated/pricing";
import { FigureLabel } from "../home/entry/entry";
import { Label } from "../home/entry/label";
import { Toggle } from "../home/entry/switch";
import { Glyph, type GlyphName } from "../home/figure/glyph";
import { Tile } from "../home/figure/tile";
import { chrome } from "../home/figure/chrome.stylex";
import { frame } from "../layout/frame.stylex";
import { lattice } from "../layout/lattice.stylex";
import { Hint } from "./hint";
import { type Currency, money } from "./money";

/** The product, where accounts subscribe to a plan. */
const PRODUCT = "https://destack.app/";

/** The names of the currencies, as the currency picker lists them. */
const currencyNames: Readonly<Record<Currency, string>> = {
    USD: "US dollars",
    CAD: "Canadian dollars",
    EUR: "euros",
    CHF: "Swiss francs",
    GBP: "British pounds",
    AUD: "Australian dollars",
};

/** The tax each currency's prices state: included where consumer prices must include it. */
const taxNotes: Readonly<Record<Currency, string>> = {
    USD: "plus tax",
    CAD: "plus tax",
    EUR: "incl. VAT",
    CHF: "incl. VAT",
    GBP: "incl. VAT",
    AUD: "incl. GST",
};

/** The days in a year, the retention shown as a year. */
const YEAR_DAYS = 365;

/** A plan of the generated price list. */
type Plan = (typeof plans)[number];

/** Who a plan is shown to: a person, or an organisation. */
export type Audience = "personal" | "organisation";

/** The specs every card lists on the same rows, under its usage: retention and support. */
const SPECS = ["retention", "support"] as const;

/** What a plan unlocks, listed where it first appears and stacking from there. */
const EXTRAS = ["early", "domains", "machines"] as const;

/** The rows of a card: head, price, action, one per spec, and the extras taking the rest. */
const CARD_ROWS = 3 + SPECS.length + 1;

/** What a benefit is about. */
type Topic = (typeof SPECS)[number] | (typeof EXTRAS)[number];

/** One benefit a card lists: its icon, its words, and what they mean. */
type Benefit = { icon: GlyphName; text: string; detail: string };

/** Each plan's tagline, the same for people and organisations. */
const taglines: Readonly<Record<Plan["key"], string>> = {
    free: "For trying it out",
    go: "For the apps you use and make",
    plus: "For the apps you rely on",
    pro: "For running everything on Destack",
};

/** The plans each audience is shown. */
const audiencePlans: Readonly<Record<Audience, readonly Plan["key"][]>> = {
    personal: ["free", "go", "plus", "pro"],
    organisation: ["free", "plus", "pro"],
};

/** The parts of the stack every plan includes, each with its tile and what it does. */
const stackParts: readonly { name: string; icon: string; detail: string }[] = [
    { name: "Branching", icon: "source", detail: "Every app branches, with previews" },
    { name: "Access", icon: "auth", detail: "Roles for people, guests and agents" },
    { name: "Vault", icon: "vault", detail: "Secrets, encrypted and shared" },
    { name: "Audit", icon: "audit", detail: "Every change, by whom and when" },
    { name: "Telemetry", icon: "telemetry", detail: "Traces, logs and alerts" },
    { name: "Your own AI", icon: "ai", detail: "Subscriptions, keys or local models" },
    { name: "Self-hosting", icon: "computer", detail: "Run Destack on your own machines" },
    { name: "Remote access", icon: "cloud", detail: "Reach your machines anywhere" },
];

/** The support each level gives, as a benefit. */
const supportBenefits: Readonly<Record<Plan["support"], Benefit>> = {
    community: {
        icon: "people",
        text: "Community support",
        detail: "Ask on Discord and GitHub.",
    },
    email: {
        icon: "mail",
        text: "Email support",
        detail: "Write to the people who build Destack.",
    },
    priority: {
        icon: "bolt",
        text: "Priority support",
        detail: "Your questions go to the front of the queue.",
    },
};

/** Label the plans as the first figure, with the currency and audience the reader switches. */
export function PlansLabel(properties: {
    audience: Audience;
    currency: Currency;
    onAudience: (audience: Audience) => void;
    onCurrency: (currency: Currency) => void;
}) {
    return (
        <FigureLabel
            number={1}
            isOpen={properties.audience === "organisation"}
            stacked="personal plans"
            destacked="organisation plans"
        >
            <span {...style.attrs(styles.controls)}>
                {/* the currency the prices show in, guessed from the reader's region, and its tax */}
                <span {...style.attrs(styles.currency)}>
                    In
                    <span {...style.attrs(styles.picker)}>
                        <select
                            aria-label="Currency"
                            title={`Prices in ${currencyNames[properties.currency]}`}
                            value={properties.currency}
                            onChange={(event) =>
                                properties.onCurrency(currencyIn(event.currentTarget.value))
                            }
                            {...style.attrs(styles.select)}
                        >
                            {currencies.map((currency) => (
                                <option value={currency}>{currency}</option>
                            ))}
                        </select>
                        <svg aria-hidden="true" viewBox="0 0 24 24" {...style.attrs(styles.caret)}>
                            <path d="m6 9 6 6 6-6" />
                        </svg>
                    </span>
                    {taxNotes[properties.currency]}
                </span>
                <Toggle
                    isOn={properties.audience === "organisation"}
                    off={["personal"]}
                    on={["organisation"]}
                    label="Plans for organisations"
                    title={
                        properties.audience === "organisation"
                            ? "Show personal plans"
                            : "Show organisation plans"
                    }
                    isCharging={false}
                    onToggle={() =>
                        properties.onAudience(
                            properties.audience === "organisation" ? "personal" : "organisation",
                        )
                    }
                />
            </span>
        </FigureLabel>
    );
}

/** Set an audience's plans side by side, each listing what it adds on rows shared by all. */
export function Plans(properties: { audience: Audience; currency: Currency }) {
    // show the plans the audience is offered
    const shown = () =>
        plans.filter((plan) => audiencePlans[properties.audience].includes(plan.key));

    return (
        <>
            <For each={shown()}>
                {(plan, index) => (
                    <PlanCard
                        plan={plan}
                        previous={shown()[index() - 1]}
                        tagline={taglines[plan.key]}
                        count={shown().length}
                        currency={properties.currency}
                    />
                )}
            </For>

            {/* the full stack, the same on every plan */}
            <div {...style.attrs(lattice.cell, styles.stack)}>
                <div {...style.attrs(styles.stackCard)}>
                    <Label>The full stack, on every plan</Label>
                    <ul {...style.attrs(styles.parts)}>
                        {stackParts.map((part) => (
                            <li {...style.attrs(styles.part)}>
                                <Tile icon={part.icon} size="large" />
                                <span {...style.attrs(styles.partWords)}>
                                    <span {...style.attrs(styles.partName)}>{part.name}</span>
                                    <span {...style.attrs(styles.partDetail)}>{part.detail}</span>
                                </span>
                            </li>
                        ))}
                    </ul>
                </div>
            </div>
        </>
    );
}

/** Draw one plan's card: its name, tagline, fee, action and, on its topic's row, what it adds. */
function PlanCard(properties: {
    plan: Plan;
    previous: Plan | undefined;
    tagline: string;
    count: number;
    currency: Currency;
}) {
    // read the plan's fee, and whether it is the free plan or the one most people pick
    const plan = properties.plan;
    const fee = () => plan.fees[properties.currency];
    const isFree = plan.fees.USD === 0;
    const isPicked = plan.key === "plus";

    // list every extra the plan unlocks, marking those the plan shown before has too
    const extras = () =>
        EXTRAS.flatMap((topic) => {
            const benefit = benefitOf(plan, topic);
            const isInherited =
                properties.previous !== undefined &&
                benefitOf(properties.previous, topic) !== undefined;

            return benefit === undefined ? [] : [{ benefit, isInherited }];
        });

    return (
        <article
            {...style.attrs(
                lattice.cell,
                styles.card,
                properties.count === 3 ? styles.third : styles.quarter,
                isPicked && styles.picked,
            )}
        >
            {/* name and tagline */}
            <div {...style.attrs(styles.head)}>
                <h2 {...style.attrs(styles.name)}>{plan.name}</h2>
                <p {...style.attrs(styles.tagline)}>{properties.tagline}</p>
            </div>

            {/* fee, and the usage it includes */}
            <div {...style.attrs(styles.price)}>
                <p {...style.attrs(styles.fee)}>
                    <span {...style.attrs(styles.amount)}>{money(fee(), properties.currency)}</span>
                    <span {...style.attrs(styles.period)}>/ month</span>
                </p>
                <Usage plan={plan} fee={fee()} currency={properties.currency} />
            </div>

            {/* start for free, or subscribe */}
            <a
                href={isFree ? "/#present-participle" : PRODUCT}
                {...style.attrs(
                    buttonVariants({ variant: isPicked ? "default" : "outline", size: "lg" }),
                    styles.action,
                )}
            >
                {isFree ? "Start for free" : `Get ${plan.name}`}
            </a>

            {/* retention and support, on the same rows on every card */}
            {SPECS.map((topic, index) => (
                <BenefitRow
                    benefit={present(benefitOf(plan, topic), "spec")}
                    isFirst={index === 0}
                    isInherited={false}
                />
            ))}

            {/* what the plan unlocks, the plan before's muted and its own in full */}
            <div {...style.attrs(styles.extras)}>
                {extras().map((extra) => (
                    <BenefitRow
                        benefit={extra.benefit}
                        isFirst={false}
                        isInherited={extra.isInherited}
                    />
                ))}
            </div>
        </article>
    );
}

/** Set the usage a plan includes each month under its fee. */
function Usage(properties: { plan: Plan; fee: number; currency: Currency }) {
    return properties.plan.isUncapped ? (
        <p {...style.attrs(styles.usage)}>
            <Hint
                text={`${money(properties.fee, properties.currency)} of usage included`}
                detail="Past the included amount, usage is billed at the prices below, up to a monthly limit you set."
            />
            <br />
            then billed up to your limit
        </p>
    ) : (
        <p {...style.attrs(styles.usage)}>
            <Hint
                text={`${allowance.database} GB database, ${allowance.files} GB files`}
                detail={`Includes ${allowance.egress} GB of egress. Past the allowance your spaces slow down, and nothing is billed.`}
            />
            <br />
            {allowance.compute} GB-hours of compute
        </p>
    );
}

/** Draw one benefit: its icon and its words, the first spec set apart from the action above. */
function BenefitRow(properties: { benefit: Benefit; isFirst: boolean; isInherited: boolean }) {
    return (
        <p
            {...style.attrs(
                styles.benefit,
                properties.isFirst && styles.first,
                properties.isInherited && styles.inherited,
            )}
        >
            <span aria-hidden="true" {...style.attrs(styles.icon)}>
                <Glyph name={properties.benefit.icon} size={16} weight={1.75} />
            </span>
            <Hint text={properties.benefit.text} detail={properties.benefit.detail} />
        </p>
    );
}

/** Return what a plan gives on a topic, or nothing where it gives nothing there. */
function benefitOf(plan: Plan, topic: Topic): Benefit | undefined {
    // read how long the plan keeps events
    if (topic === "retention") {
        const span = plan.retention >= YEAR_DAYS ? "1-year" : `${plan.retention}-day`;

        return {
            icon: "clock",
            text: `${span} retention`,
            detail: "How long activity, history, logs and analytics stay. Your content never expires.",
        };
    }
    // read what the plan unlocks
    else if (topic === "domains") {
        return plan.isCustomDomains
            ? {
                  icon: "globe",
                  text: "Custom domains",
                  detail: "Serve your spaces and your profile under your own domain names.",
              }
            : undefined;
    } else if (topic === "early") {
        return plan.isEarlyAccess
            ? {
                  icon: "star",
                  text: "Early access",
                  detail: "Try new apps and features before everyone else.",
              }
            : undefined;
    } else if (topic === "machines") {
        return plan.isDedicated
            ? {
                  icon: "chip",
                  text: "Dedicated machines",
                  detail: "A machine of your own, billed by the hour up to a monthly cap.",
              }
            : undefined;
    }
    // read who helps
    else {
        return supportBenefits[plan.support];
    }
}

/** Return the currency a picker's value names, refusing one Destack does not bill in. */
function currencyIn(value: string): Currency {
    const currency = currencies.find((each) => each === value);
    if (currency === undefined) {
        throw new TypeError(`unknown currency ${value}`);
    }

    return currency;
}

/** The plan styles. */
const styles = style.create({
    controls: {
        alignItems: "center",
        display: "flex",
        gap: "1.25rem",
    },
    currency: {
        alignItems: "center",
        color: color.mutedForeground,
        display: "inline-flex",
        fontFamily: font.code,
        fontSize: "0.6875rem",
        gap: "0.5rem",
        letterSpacing: "0.1em",
        textTransform: "uppercase",
        whiteSpace: "nowrap",
    },
    picker: {
        alignItems: "center",
        borderColor: {
            default: color.border,
            ":hover": { default: null, [media.hover]: color.mutedForeground },
        },
        borderRadius: "5px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        color: color.foreground,
        display: "inline-flex",
        position: "relative",
    },
    select: {
        appearance: "none",
        backgroundColor: "transparent",
        borderWidth: 0,
        color: "inherit",
        cursor: "pointer",
        font: "inherit",
        letterSpacing: "inherit",
        paddingBlock: "0.25rem",
        paddingInlineEnd: "1.375rem",
        paddingInlineStart: "0.5rem",
    },
    caret: {
        fill: "none",
        height: "0.75rem",
        insetInlineEnd: "0.375rem",
        pointerEvents: "none",
        position: "absolute",
        stroke: "currentColor",
        strokeLinecap: "round",
        strokeLinejoin: "round",
        strokeWidth: 2.5,
        width: "0.75rem",
    },
    card: {
        display: "grid",
        gridRow: `span ${CARD_ROWS}`,
        gridTemplateRows: "subgrid",
        padding: `2rem ${frame.inset} 2rem`,
        rowGap: "0.875rem",
    },
    quarter: {
        gridColumn: { default: "span 3", [media.maxLg]: "span 6", [media.maxMd]: "1 / -1" },
    },
    third: {
        gridColumn: { default: "span 4", [media.maxLg]: "span 6", [media.maxMd]: "1 / -1" },
    },
    picked: {
        backgroundColor: `color-mix(in srgb, ${color.primary} 5%, ${color.background})`,
    },
    head: {
        alignContent: "start",
        display: "grid",
        rowGap: "0.375rem",
    },
    name: {
        fontSize: "1.5rem",
        fontWeight: weight.semibold,
        letterSpacing: "-0.02em",
        lineHeight: 1.15,
        margin: 0,
    },
    tagline: {
        color: color.mutedForeground,
        fontSize: "0.9375rem",
        margin: 0,
    },
    price: {
        display: "grid",
        paddingBlockStart: "0.5rem",
        rowGap: "0.875rem",
    },
    fee: {
        alignItems: "baseline",
        display: "flex",
        gap: "0.375rem",
        margin: 0,
    },
    amount: {
        fontSize: "2.875rem",
        fontVariantNumeric: "tabular-nums",
        fontWeight: weight.semibold,
        letterSpacing: "-0.04em",
        lineHeight: 1,
    },
    period: {
        color: color.mutedForeground,
        fontSize: "0.9375rem",
        whiteSpace: "nowrap",
    },
    usage: {
        color: color.mutedForeground,
        fontSize: "0.875rem",
        lineHeight: 1.6,
        margin: 0,
    },
    action: {
        justifyContent: "center",
        marginBlock: "0.5rem",
        textDecorationLine: "none",
        width: "100%",
    },
    first: {
        paddingBlockStart: "1rem",
    },
    benefit: {
        alignItems: "center",
        display: "flex",
        fontSize: "0.9375rem",
        gap: "0.75rem",
        margin: 0,
        whiteSpace: "nowrap",
    },
    inherited: {
        color: color.mutedForeground,
    },
    icon: {
        color: color.mutedForeground,
        display: "flex",
        flexShrink: 0,
    },
    extras: {
        alignContent: "start",
        display: "grid",
        rowGap: "0.75rem",
    },
    stack: {
        gridColumn: "1 / -1",
        paddingBlock: "1.25rem",
        paddingInline: frame.inset,
    },
    stackCard: {
        backgroundColor: color.card,
        borderColor: color.border,
        borderRadius: chrome.radius,
        borderStyle: "solid",
        borderWidth: stroke.border,
        boxShadow: shadow.raised,
        display: "grid",
        paddingBlock: "1.25rem",
        paddingInline: "1.5rem",
        rowGap: "1rem",
    },
    parts: {
        columnGap: "1.5rem",
        display: "grid",
        gridTemplateColumns: {
            default: "repeat(4, minmax(0, 1fr))",
            [media.maxLg]: "repeat(auto-fit, minmax(15rem, 1fr))",
        },
        listStyleType: "none",
        margin: 0,
        padding: 0,
        rowGap: "1.125rem",
    },
    part: {
        alignItems: "center",
        columnGap: "0.75rem",
        display: "flex",
        minWidth: 0,
    },
    partWords: {
        display: "grid",
        minWidth: 0,
        rowGap: "0.125rem",
    },
    partName: {
        fontSize: "0.9375rem",
        fontWeight: weight.semibold,
        lineHeight: 1.25,
    },
    partDetail: {
        color: color.mutedForeground,
        fontSize: "0.8125rem",
        lineHeight: 1.35,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
});
