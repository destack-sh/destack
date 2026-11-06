import { media } from "@destack/style/media.stylex";
import { color, font, stroke, weight } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import { present } from "@destack/schema";
import { buttonVariants } from "@destack/ui/button";
import { For } from "@destack/view";

import { allowance, currencies, plans } from "../content/generated/pricing";
import { FigureLabel } from "../home/entry/entry";
import { Toggle } from "../home/entry/switch";
import { Glyph, type GlyphName } from "../home/figure/glyph";
import { frame } from "../layout/frame.stylex";
import { lattice } from "../layout/lattice.stylex";
import { type Feature, features } from "./feature";
import { Hint } from "./hint";
import { type Currency, money } from "./money";
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

/** The specs every card lists on the same rows, under its usage. */
const SPECS = ["apps", "hosting", "ai", "managed", "history", "support"] as const;

/** What a plan unlocks, listed where it first appears and stacking from there. */
const EXTRAS = [
    "caps",
    "early",
    "domains",
    "machines",
    "uptime",
    "engineer",
    "invoice",
    "agreement",
] as const;

/** The rows of a card: head, price, action, one per spec, and the extras taking the rest. */
const CARD_ROWS = 3 + SPECS.length + 1;

/** What a benefit is about. */
type Topic = (typeof SPECS)[number] | (typeof EXTRAS)[number];

/** One benefit a card lists: its icon and its feature. */
type Benefit = { icon: GlyphName; feature: Feature };

/** Each plan's tagline, the same for people and organisations. */
const taglines: Readonly<Record<Plan["key"], string>> = {
    free: "For your first apps",
    go: "For your personal stack",
    plus: "For your daily work",
    pro: "For your critical software",
    enterprise: "For your whole organisation",
};

/** The plans each audience is shown. */
export const audiencePlans: Readonly<Record<Audience, readonly Plan["key"][]>> = {
    personal: ["free", "go", "plus", "pro"],
    organisation: ["free", "plus", "pro", "enterprise"],
};

/** The support each level gives, as a benefit. */
const supportBenefits: Readonly<Record<Plan["support"], Benefit>> = {
    community: { icon: "people", feature: features.communitySupport },
    email: { icon: "mail", feature: features.emailSupport },
    priority: { icon: "bolt", feature: features.prioritySupport },
    sla: { icon: "help", feature: features.slaSupport },
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

/** Set an audience's plans side by side, each listing what it adds on rows shared by all. */
export function Plans(properties: { audience: Audience; currency: Currency }) {
    // show the plans the audience is offered
    const shown = () =>
        plans.filter((plan) => audiencePlans[properties.audience].includes(plan.key));

    return (
        <>
            <For each={shown()}>
                {(plan) => (
                    <PlanCard
                        plan={plan}
                        tagline={taglines[plan.key]}
                        count={shown().length}
                        currency={properties.currency}
                    />
                )}
            </For>
        </>
    );
}

/** Draw one plan's card: its name, tagline, fee, action and, on its topic's row, what it adds. */
function PlanCard(properties: { plan: Plan; tagline: string; count: number; currency: Currency }) {
    // read the plan's fee, and whether it is the free plan, a contract, or the one most people pick
    const plan = properties.plan;
    const fee = () => plan.fees[properties.currency];
    const isFree = plan.fees.USD === 0;
    const isPicked = plan.key === "plus";

    // list every extra the plan unlocks
    const extras = () =>
        EXTRAS.flatMap((topic) => {
            const benefit = benefitOf(plan, topic);

            return benefit === undefined ? [] : [benefit];
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
                    <span {...style.attrs(styles.amount)}>
                        {money(fee(), properties.currency)}
                        {plan.isContract && "+"}
                    </span>
                    {!plan.isContract && <span {...style.attrs(styles.period)}>/ month</span>}
                </p>
                <Usage plan={plan} fee={fee()} currency={properties.currency} />
            </div>

            {/* start for free, subscribe, or write to us */}
            <a
                href={isFree ? "/#present-participle" : plan.isContract ? CONTACT : PRODUCT}
                {...style.attrs(
                    buttonVariants({ variant: isPicked ? "default" : "outline", size: "lg" }),
                    press.key,
                    styles.action,
                )}
            >
                {isFree ? "Start for free" : plan.isContract ? "Talk to us" : `Get ${plan.name}`}
            </a>

            {/* apps, hosting, both kinds of AI, history and support, on the same rows on every card */}
            {SPECS.map((topic, index) => (
                <BenefitRow
                    benefit={present(benefitOf(plan, topic), "spec")}
                    isFirst={index === 0}
                />
            ))}

            {/* what the plan unlocks */}
            <div {...style.attrs(styles.extras)}>
                {extras().map((benefit) => (
                    <BenefitRow benefit={benefit} isFirst={false} />
                ))}
            </div>
        </article>
    );
}

/** Set the usage a plan includes each month under its fee. */
function Usage(properties: { plan: Plan; fee: number; currency: Currency }) {
    // state a contract's least monthly usage, the usage a fee includes, or the free allowance
    return properties.plan.isContract ? (
        <p {...style.attrs(styles.usage)}>
            <Hint
                text="a month in usage"
                detail="Your contract commits to at least this much usage a month, billed at the prices below."
            />
            <br />
            then pay as you go
        </p>
    ) : properties.plan.isUncapped ? (
        <p {...style.attrs(styles.usage)}>
            <Hint
                text={`${money(properties.fee, properties.currency)} of usage included`}
                detail="Past the included amount, usage is billed at the prices below, up to a monthly limit you set."
            />
            <br />
            then pay as you go
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
function BenefitRow(properties: { benefit: Benefit; isFirst: boolean }) {
    return (
        <p {...style.attrs(styles.benefit, properties.isFirst && styles.first)}>
            <span aria-hidden="true" {...style.attrs(styles.icon)}>
                <Glyph name={properties.benefit.icon} size={16} weight={1.75} />
            </span>
            <Hint
                text={properties.benefit.feature.name}
                detail={properties.benefit.feature.detail}
            />
        </p>
    );
}

/** Return what a plan gives on a topic, or nothing where it gives nothing there. */
function benefitOf(plan: Plan, topic: Topic): Benefit | undefined {
    // read which apps, where the plan runs and which AI it uses, the same on every plan
    if (topic === "apps") {
        return { icon: "grid", feature: features.apps };
    } else if (topic === "hosting") {
        return { icon: "monitor", feature: features.selfHosting };
    } else if (topic === "ai") {
        return { icon: "plug", feature: features.ownAi };
    } else if (topic === "managed") {
        return { icon: "cloud", feature: features.managedAi };
    }
    // keep events for their default spans, or as long as each space chooses
    else if (topic === "history") {
        return {
            icon: "clock",
            feature: plan.isCustomRetention ? features.customRetention : features.defaultRetention,
        };
    }
    // read what the plan unlocks
    else if (topic === "domains") {
        return plan.isCustomDomains ? { icon: "globe", feature: features.domains } : undefined;
    } else if (topic === "caps") {
        return plan.isUncapped ? { icon: "up", feature: features.uncapped } : undefined;
    } else if (topic === "early") {
        return plan.isEarlyAccess ? { icon: "star", feature: features.early } : undefined;
    } else if (topic === "machines") {
        return plan.isDedicated ? { icon: "chip", feature: features.dedicated } : undefined;
    } else if (topic === "uptime") {
        return plan.isContract ? { icon: "check", feature: features.uptime } : undefined;
    } else if (topic === "engineer") {
        return plan.isContract ? { icon: "person", feature: features.engineer } : undefined;
    } else if (topic === "invoice") {
        return plan.isContract ? { icon: "file", feature: features.invoice } : undefined;
    } else if (topic === "agreement") {
        return plan.isContract ? { icon: "lock", feature: features.agreement } : undefined;
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
