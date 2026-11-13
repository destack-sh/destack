import { media } from "@destack/style/media.stylex";
import * as style from "@destack/style";
import { createSignal, onSettled, useEnvironment } from "@destack/view";
import { createEventListener } from "@destack/view/primitives/event-listener";
import { Metadata } from "@destack/view/document";

import { Entry, type Form } from "../home/entry/entry";
import { Tally } from "../home/participle/tally";
import { firstPicks } from "../home/participle/wall";
import type { Tool } from "../home/tool/tool";
import { commandEvents } from "../layout/command";
import { Interval } from "../layout/sheet";
import { lattice } from "../layout/lattice.stylex";
import { type Currency, currencyOf } from "../pricing/money";
import { Comparison, ComparisonLabel } from "../pricing/comparison";
import { type Audience, Plans, PlansLabel } from "../pricing/plan";
import { Gauge } from "../pricing/gauge";
import { Hosts } from "../pricing/hosts";
import { Silos } from "../pricing/silos";
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
        sentence: "The software is included and yours to keep. Run it on our hardware or your own.",
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
        sentence:
            "Compute, storage and requests are billed as your apps run. Data in and out is free.",
    },
};

/** The noun the comparison defines, in Destack's sense. */
const bill: Form = {
    syllables: ["bill"],
    pronunciation: "/bɪl/",
    partOfSpeech: "noun",
    sense: {
        definition: "the price of silos",
        highlight: ["silos"],
        sentence: "Each silo sends its own bill. None of them connect.",
    },
};

/** Render the pricing page: the plans, their comparison, the unit prices, then the bill. */
export function Pricing() {
    // show the personal plans until the reader switches to the organisation ones
    const [audience, setAudience] = createSignal<Audience>("personal");

    // show prices in the currency of the reader's region, which they can switch
    const [currency, setCurrency] = createSignal<Currency>("USD");
    const { locale, timeZone } = useEnvironment();
    onSettled(() => {
        setCurrency(currencyOf(timeZone, locale));
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
            <section {...style.attrs(lattice.frame, lattice.ruled, styles.plans, styles.drawn)}>
                <Entry form={pricing} isTitle />
                <Hosts />
                <PlansLabel
                    audience={audience()}
                    currency={currency()}
                    onAudience={setAudience}
                    onCurrency={setCurrency}
                />
                <Plans audience={audience()} currency={currency()} />
            </section>
            <Interval section={2} />

            {/* the plans compared */}
            <section {...style.attrs(lattice.frame, lattice.ruled)}>
                <ComparisonLabel />
                <Comparison audience={audience()} currency={currency()} />
            </section>
            <Interval section={3} />

            {/* the rate card */}
            <section {...style.attrs(lattice.frame, lattice.ruled, styles.drawn)}>
                <Entry form={rate} />
                <Gauge />
                <Rates currency={currency()} />
            </section>
            <Interval section={4} />

            {/* the bill: your tools at seat prices, or your apps at usage prices */}
            <section
                {...style.attrs(
                    lattice.frame,
                    lattice.ruled,
                    lattice.section,
                    styles.figure,
                    styles.drawn,
                )}
            >
                <Entry form={bill} />
                <Silos isOpen={isOpen()} />
                <Tally
                    number={4}
                    stacked="your tools at seat prices"
                    destacked="your apps at usage prices"
                    isOpen={isOpen()}
                    picks={picks()}
                    onPick={pick}
                />
            </section>
            <Interval />
        </>
    );
}

/** The pricing page styles. */
const styles = style.create({
    // hold each section's drawing in its entry's word column
    drawn: {
        position: "relative",
    },
    // the entry, the figure's label, then the figure taking the rest
    figure: {
        gridTemplateRows: { default: "auto auto minmax(0, 1fr)", [media.maxLg]: "auto" },
    },
    // the entry, the label, then the plan rows
    plans: {
        gridTemplateRows: {
            default: "auto auto repeat(4, auto) minmax(auto, 1fr)",
            [media.maxLg]: "auto",
        },
    },
});
