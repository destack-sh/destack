import { media } from "@destack/style/media.stylex";
import { color, font, stroke, weight } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import { present } from "@destack/schema";
import { buttonVariants } from "@destack/ui/button";
import { For } from "@destack/view";

import { allowance, currencies, plans } from "../content/generated/pricing";
import { FigureLabel } from "../home/entry/entry";
import { Toggle } from "../home/entry/switch";
import { Glyph } from "../home/figure/glyph";
import { frame } from "../layout/frame.stylex";
import { lattice } from "../layout/lattice.stylex";
import { screen } from "../layout/screen.stylex";
import { type Feature, features } from "./feature";
import { Hint } from "./hint";
import { convert, type Currency, money } from "./money";
import { typeScale, tracking } from "../layout/type.stylex";
import { press } from "../layout/press";

/** The product, where accounts subscribe to a plan. */
const PRODUCT = "https://destack.app/";

/** Where organisations write to ask for the enterprise contract. */
const CONTACT = "mailto:florian@symbol.industries?subject=Destack%20Enterprise";

/** The names of the currencies, as the currency picker lists them. */
const currencyNames: Readonly<Record<Currency, string>> = {
    USD: "US dollars",
    CAD: "Canadian dollars",
    EUR: "euros",
    CHF: "Swiss francs",
    GBP: "British pounds",
    AUD: "Australian dollars",
};

/** The tax each audience's prices state per currency: included for people where the law asks it. */
const taxNotes: Readonly<Record<Audience, Readonly<Record<Currency, string>>>> = {
    personal: {
        USD: "plus tax",
        CAD: "plus tax",
        EUR: "incl. VAT",
        CHF: "incl. VAT",
        GBP: "incl. VAT",
        AUD: "incl. GST",
    },
    organisation: {
        USD: "plus tax",
        CAD: "plus tax",
        EUR: "excl. VAT",
        CHF: "excl. VAT",
        GBP: "excl. VAT",
        AUD: "excl. GST",
    },
};

/** A plan of the generated price list. */
export type Plan = (typeof plans)[number];

/** Who a plan is shown to: a person, or an organisation. */
export type Audience = "personal" | "organisation";

/** What a card can list, in the order it lists it: the software, then what plans add. */
const TOPICS = [
    "apps",
    "selfHosting",
    "cloudHosting",
    "residency",
    "ownAi",
    "managedAi",
    "history",
    "caps",
    "early",
    "domains",
    "machines",
    "support",
] as const;

/** The rows of a card: head, price, action, the line leading its list, and the list. */
const CARD_ROWS = 5;

/** What a listed feature is about. */
type Topic = (typeof TOPICS)[number];

/** Each plan's tagline, the same for people and organisations. */
const taglines: Readonly<Record<Plan["key"], string>> = {
    free: "For your first apps",
    go: "For your everyday stack",
    plus: "For your daily work",
    pro: "For your critical software",
    enterprise: "For your whole organisation",
};

/** What Enterprise adds past Pro, each in the words its sentence lists it with. */
const ENTERPRISE_FEATURES: readonly { phrase: string; feature: Feature }[] = [
    { phrase: "SLA support", feature: features.slaSupport },
    { phrase: "an uptime SLA", feature: features.uptime },
    { phrase: "a dedicated engineer", feature: features.engineer },
    { phrase: "invoice billing", feature: features.invoice },
    { phrase: "a data processing agreement", feature: features.agreement },
];

/** The support each level gives, as a feature. */
const supportFeatures: Readonly<Record<Plan["support"], Feature>> = {
    community: features.communitySupport,
    email: features.emailSupport,
    priority: features.prioritySupport,
    sla: features.slaSupport,
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
                    {taxNotes[properties.audience][properties.currency]}
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

/** Set the plans side by side, Pro leading on to Enterprise for organisations. */
export function Plans(properties: { audience: Audience; currency: Currency }) {
    // show every plan beside the one below it, the enterprise contract offered past Pro
    const shown = plans.filter((plan) => !plan.isContract);

    return (
        <>
            <For each={shown}>
                {(plan, index) => (
                    <PlanCard
                        plan={plan}
                        below={shown[index() - 1]}
                        tagline={taglines[plan.key]}
                        count={shown.length}
                        currency={properties.currency}
                        isPooled={properties.audience === "organisation"}
                        enterprise={
                            plan.key !== "pro"
                                ? "absent"
                                : properties.audience === "organisation"
                                  ? "shown"
                                  : "kept"
                        }
                    />
                )}
            </For>
        </>
    );
}

/** Draw one plan's card: its name, tagline, fee, action and, on its topic's row, what it adds. */
function PlanCard(properties: {
    plan: Plan;
    below: Plan | undefined;
    tagline: string;
    count: number;
    currency: Currency;
    isPooled: boolean;
    enterprise: "shown" | "kept" | "absent";
}) {
    // read the plan's fee, and whether it is the free plan, a contract, or the one most people pick
    const plan = properties.plan;
    const fee = () => plan.fees[properties.currency];
    const isFree = plan.fees.USD === 0;
    const isPicked = plan.key === "plus";

    // list what the plan adds over the plan below it, or everything on the first plan
    const below = properties.below;
    const shown = below === undefined ? [] : featuresOf(below);
    const added = featuresOf(plan).filter((feature) => !shown.includes(feature));

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
                    <span {...style.attrs(styles.period)}>
                        {isFree || !properties.isPooled ? "/ month" : "/ person / month"}
                    </span>
                </p>
                <Usage
                    plan={plan}
                    fee={fee()}
                    currency={properties.currency}
                    isPooled={properties.isPooled}
                />
            </div>

            {/* start for free, subscribe, or write to us */}
            <a
                href={isFree ? "/#present-participle" : PRODUCT}
                {...style.attrs(
                    buttonVariants({ variant: isPicked ? "default" : "outline", size: "lg" }),
                    press.key,
                    styles.action,
                )}
            >
                {isFree ? "Start for free" : `Get ${plan.name}`}
            </a>

            {/* what the plan adds over the plan below it, then Enterprise past Pro for organisations */}
            <p {...style.attrs(styles.lead)}>
                {below === undefined ? "All of Destack:" : `Everything in ${below.name}, plus:`}
            </p>
            <div {...style.attrs(styles.extras)}>
                {added.map((feature) => (
                    <FeatureRow feature={feature} />
                ))}
                {properties.enterprise === "absent" ? undefined : (
                    <Enterprise
                        currency={properties.currency}
                        isShown={properties.enterprise === "shown"}
                    />
                )}
            </div>
        </article>
    );
}

/** Draw Enterprise past Pro, or keep its space hidden so the audiences share one height. */
function Enterprise(properties: { currency: Currency; isShown: boolean }) {
    // read the contract's least monthly usage
    const contract = present(
        plans.find((plan) => plan.isContract),
        "the enterprise contract",
    );

    return (
        <section
            aria-hidden={properties.isShown ? undefined : "true"}
            inert={!properties.isShown}
            {...style.attrs(styles.enterprise, !properties.isShown && styles.kept)}
        >
            {/* the spending it starts at, on a rule */}
            <p {...style.attrs(styles.threshold)}>
                {money(contract.fees[properties.currency], properties.currency)} a month and up
            </p>

            {/* name, what it adds and the action */}
            <h3 {...style.attrs(styles.name)}>{contract.name}</h3>
            <p {...style.attrs(styles.usage)}>
                {ENTERPRISE_FEATURES.map(({ phrase, feature }, index) => (
                    <>
                        {index === 0
                            ? ""
                            : index === ENTERPRISE_FEATURES.length - 1
                              ? " and "
                              : ", "}
                        <Hint text={phrase} detail={feature.detail} />
                    </>
                ))}
            </p>
            <a
                href={CONTACT}
                {...style.attrs(
                    buttonVariants({ variant: "outline", size: "lg" }),
                    press.key,
                    styles.action,
                )}
            >
                Talk to us
            </a>
        </section>
    );
}

/** Set the usage a plan includes each month under its fee, pooled across an organisation. */
function Usage(properties: { plan: Plan; fee: number; currency: Currency; isPooled: boolean }) {
    // name the usage the fee includes, or the free plan's, pooled for an organisation
    const isFree = () => properties.fee === 0;
    const amount = () =>
        isFree()
            ? money(convert(allowance.usage, properties.currency), properties.currency, true)
            : money(properties.fee, properties.currency);
    const included = () => `${amount()} of ${properties.isPooled ? "pooled " : ""}usage included`;

    // explain the pool for an organisation, then what happens past the included usage
    const pool = () =>
        properties.isPooled ? "Each member's fee is usage, pooled across your organisation. " : "";
    const detail = () =>
        isFree()
            ? `Enough for ${allowance.database} GB of database, ${allowance.files} GB of files, ${allowance.compute} GB-hours of compute and a million requests a month${properties.isPooled ? " for each member, up to 5" : ""}. Past it your spaces slow down and stop taking writes, and nothing is billed.`
            : properties.plan.isUncapped
              ? `${pool()}Past it, usage is billed at the prices below, up to a monthly limit you set.`
              : `${pool()}Past it your spaces slow down and stop taking writes, and nothing more is billed.`;
    const past = () =>
        properties.plan.isUncapped
            ? "then pay as you go"
            : isFree() && properties.isPooled
              ? "per person, then capped"
              : "then capped";

    return (
        <p {...style.attrs(styles.usage)}>
            <Hint text={included()} detail={detail()} />
            <br />
            {past()}
        </p>
    );
}

/** Draw one feature: its icon and its name. */
function FeatureRow(properties: { feature: Feature }) {
    return (
        <p {...style.attrs(styles.benefit)}>
            <span aria-hidden="true" {...style.attrs(styles.icon)}>
                <Glyph name={properties.feature.icon} size={16} weight={1.75} />
            </span>
            <Hint text={properties.feature.name} detail={properties.feature.detail} />
        </p>
    );
}

/** Return what a plan gives, topic by topic. */
function featuresOf(plan: Plan): Feature[] {
    return TOPICS.flatMap((topic) => {
        const feature = featureOf(plan, topic);

        return feature === undefined ? [] : [feature];
    });
}

/** Return what a plan gives on a topic, or nothing where it gives nothing there. */
function featureOf(plan: Plan, topic: Topic): Feature | undefined {
    // read which apps, where the plan runs and which AI it uses, the same on every plan
    if (topic === "apps") {
        return features.apps;
    } else if (topic === "selfHosting") {
        return features.selfHosting;
    } else if (topic === "cloudHosting") {
        return features.cloudHosting;
    } else if (topic === "residency") {
        return features.residency;
    } else if (topic === "ownAi") {
        return features.ownAi;
    } else if (topic === "managedAi") {
        return features.managedAi;
    }
    // keep events for their default spans, or as long as each space chooses
    else if (topic === "history") {
        return plan.isCustomRetention ? features.customRetention : features.defaultRetention;
    }
    // read what the plan unlocks
    else if (topic === "domains") {
        return plan.isCustomDomains ? features.domains : undefined;
    } else if (topic === "caps") {
        return plan.isUncapped ? features.uncapped : undefined;
    } else if (topic === "early") {
        return plan.isEarlyAccess ? features.early : undefined;
    } else if (topic === "machines") {
        return plan.isDedicated ? features.dedicated : undefined;
    }
    // read who helps
    else {
        return supportFeatures[plan.support];
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
        fontSize: typeScale.label,
        gap: "0.5rem",
        letterSpacing: tracking.label,
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
        fontSize: typeScale.lead,
        fontWeight: weight.semibold,
        lineHeight: 1.15,
        margin: 0,
    },
    tagline: {
        color: color.mutedForeground,
        fontSize: typeScale.body,
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
    usage: {
        color: color.mutedForeground,
        fontSize: typeScale.small,
        lineHeight: 1.6,
        margin: 0,
    },
    enterprise: {
        display: "grid",
        marginBlockStart: "1.25rem",
        rowGap: "0.75rem",
    },
    kept: {
        display: { default: "grid", [screen.belowDesktop]: "none" },
        visibility: "hidden",
    },
    threshold: {
        alignItems: "center",
        color: color.mutedForeground,
        columnGap: "0.75rem",
        display: "flex",
        fontFamily: font.code,
        fontSize: typeScale.label,
        letterSpacing: tracking.label,
        margin: 0,
        marginBlockEnd: "0.5rem",
        textTransform: "uppercase",
        whiteSpace: "nowrap",
        "::after": {
            borderBlockStartColor: color.border,
            borderBlockStartStyle: "solid",
            borderBlockStartWidth: stroke.border,
            content: "''",
            flexGrow: 1,
        },
    },
    action: {
        justifyContent: "center",
        marginBlock: "0.5rem",
        textDecorationLine: "none",
        width: "100%",
    },
    lead: {
        color: color.mutedForeground,
        fontSize: typeScale.small,
        margin: 0,
        paddingBlockStart: "1rem",
    },
    benefit: {
        alignItems: "center",
        display: "flex",
        fontSize: typeScale.body,
        gap: "0.75rem",
        margin: 0,
        whiteSpace: "nowrap",
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
});
