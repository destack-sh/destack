import * as style from "@destack/style";
import { createSignal, onSettled } from "@destack/view";
import { createEventListener } from "@destack/view/primitives/event-listener";
import { Metadata } from "@destack/view/document";

import { Entry, type Form } from "../home/entry/entry";
import { Tally } from "../home/participle/tally";
import { firstPicks } from "../home/participle/wall";
import type { Tool } from "../home/tool/tool";
import { commandEvents } from "../layout/command";
import { frame } from "../layout/frame.stylex";
import { lattice } from "../layout/lattice.stylex";
import { screen } from "../layout/screen.stylex";
import { type Currency, currencyOf } from "../pricing/money";
import { type Audience, Plans, PlansLabel } from "../pricing/plan";
import { Rates } from "../pricing/rate";
import "../home/figure/figure.css";

/** The noun the page defines, in Destack's sense. */
const pricing: Form = {
    syllables: ["pric", "ing"],
    pronunciation: "/ˈpraɪsɪŋ/",
    partOfSpeech: "noun",
    sense: {
        definition: "own the software, rent the hardware",
        highlight: ["own", "rent"],
        sentence: "Every app comes included. You pay only for infrastructure. Leave anytime.",
    },
};

/** The noun the rate card defines, in Destack's sense. */
const rate: Form = {
    syllables: ["rate"],
    pronunciation: "/reɪt/",
    partOfSpeech: "noun",
    sense: {
        definition: "a boring price for everything",
        highlight: ["boring price"],
        sentence: "Compute, storage and requests are billed as your apps run. Egress is free.",
    },
};

/** The noun the comparison defines, in Destack's sense. */
const bill: Form = {
    syllables: ["bill"],
    pronunciation: "/bɪl/",
    partOfSpeech: "noun",
    sense: {
        definition: "the price of integration",
        highlight: ["integration"],
        sentence: "Pick your tools. Compare a month of seats with a month of usage.",
    },
};

/** Render the pricing page: the plans, the unit prices, then the bill for your tools. */
export function Pricing() {
    // show the personal plans until the reader switches to the organisation ones
    const [audience, setAudience] = createSignal<Audience>("personal");

    // show prices in the currency of the reader's region, which they can switch
    const [currency, setCurrency] = createSignal<Currency>("USD");
    onSettled(() => {
        setCurrency(
            currencyOf(Intl.DateTimeFormat().resolvedOptions().timeZone, navigator.language),
        );
    });

    // keep whether the tools run destacked, flipped by the figure's switch
    const [isOpen, setIsOpen] = createSignal(false);
    createEventListener(
        () => document,
        commandEvents.switchStack,
        () => setIsOpen((current) => !current),
    );

    // keep the tools the reader uses, each pick adding or removing one
    const [picks, setPicks] = createSignal<readonly Tool[]>(firstPicks);
    const pick = (tool: Tool) =>
        setPicks((current) =>
            current.includes(tool)
                ? current.filter((picked) => picked !== tool)
                : [...current, tool],
        );

    return (
        <>
            <Metadata
                title="Pricing"
                description="Destack's plans, its unit prices, and what your tools cost on Destack."
                alternates={{ canonical: "/pricing/" }}
            />

            {/* the plans */}
            <section
                {...style.attrs(
                    lattice.frame,
                    lattice.ruled,
                    lattice.section,
                    lattice.ruleBottom,
                    styles.plans,
                )}
            >
                <Entry form={pricing} isTitle />
                <PlansLabel
                    audience={audience()}
                    currency={currency()}
                    onAudience={setAudience}
                    onCurrency={setCurrency}
                />
                <Plans audience={audience()} currency={currency()} />
            </section>

            {/* the rate card */}
            <section {...style.attrs(lattice.frame, lattice.ruled, lattice.ruleBottom)}>
                <Entry form={rate} />
                <Rates currency={currency()} />
            </section>

            {/* the bill: your tools at seat prices, or your apps at usage prices */}
            <section
                {...style.attrs(
                    lattice.frame,
                    lattice.ruled,
                    lattice.section,
                    lattice.ruleBottom,
                    styles.figure,
                )}
            >
                <Entry form={bill} />
                <Tally
                    number={3}
                    stacked="your tools at seat prices"
                    destacked="your apps at usage prices"
                    isOpen={isOpen()}
                    picks={picks()}
                    onPick={pick}
                />
            </section>
            <div aria-hidden="true" {...style.attrs(lattice.frame, lattice.interval)} />
        </>
    );
}

/** The pricing page styles. */
const styles = style.create({
    // the entry, the figure's label, then the figure taking the rest
    figure: {
        gridTemplateRows: { default: "auto auto minmax(0, 1fr)", [screen.belowDesktop]: "auto" },
    },
    // the entry, the label, the plan rows and the stack, one screen tall or more when the cards need it
    plans: {
        gridTemplateRows: {
            default: "auto auto repeat(5, auto) minmax(auto, 1fr) auto",
            [screen.belowDesktop]: "auto",
        },
        height: "auto",
        minHeight: { default: frame.section, [screen.belowDesktop]: "auto" },
    },
});
