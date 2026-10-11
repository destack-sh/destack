import * as style from "@destack/style";

import { lattice } from "../../layout/lattice.stylex";
import { Entry, FigureLabel, type Form } from "../entry/entry";
import { Label } from "../entry/label";
import { StackSwitch } from "../entry/switch";
import { labelOf, lineCount, tallyOf, ToolWall } from "./wall";
import { Bill } from "../figure/bill";
import { screen } from "../../layout/screen.stylex";
import { createSteps } from "../figure/stagger";
import { createSignal } from "@destack/view";
import { createReducedMotion } from "@destack/view/motion";

/** The milliseconds between the steps of the bill's switch, one per line. */
const stepTime = 60;

/** The participle, the last of the word's forms before the page closes. */
const participle: Form = {
    syllables: ["de", "stack", "ing"],
    pronunciation: "/diːˈstakɪŋ/",
    partOfSpeech: "present participle",
    sense: {
        definition: "reconquering your stack, one app at a time",
        highlight: ["one app at a time"],
        sentence: "Start with one app, bring your own agent, and keep going as far as you want.",
    },
};

/** Show the participle at work: the tools you pick, tallied, and the space they become. */
export function Participle(properties: {
    isOpen: boolean;
    picks: readonly string[];
    onPick: (name: string) => void;
}) {
    // follow the motion the section shows, and step the bill's lines in order as the stack switches
    const [section, setSection] = createSignal<HTMLElement>();
    const isReduced = createReducedMotion(section);
    const { isOpenAt } = createSteps(() => properties.isOpen, lineCount, stepTime, isReduced);

    return (
        <section
            ref={setSection}
            {...style.attrs(
                lattice.frame,
                lattice.ruled,
                lattice.section,
                lattice.ruleBottom,
                styles.section,
            )}
        >
            <Entry form={participle} />
            <FigureLabel
                number={5}
                isOpen={properties.isOpen}
                stacked="the fragmented stack you rent today"
                destacked="the integrated stack you own instead"
            >
                <StackSwitch isOpen={properties.isOpen} />
            </FigureLabel>
            <div {...style.attrs(lattice.cell, lattice.figureCell, styles.stage)}>
                <Label>
                    {properties.isOpen ? "Your own corner of the cloud" : "Pick your current stack"}
                </Label>
                <ToolWall
                    picks={properties.picks}
                    isOpen={properties.isOpen}
                    onPick={properties.onPick}
                />
            </div>
            <div {...style.attrs(lattice.cell, lattice.figureCell, styles.key)}>
                <Label>{properties.isOpen ? "Your space" : "Your current stack"}</Label>
                <Bill
                    lines={tallyOf(properties.picks)}
                    label={labelOf(properties.picks, properties.isOpen)}
                    isOpenAt={isOpenAt}
                />
            </div>
        </section>
    );
}

/** The closing styles. */
const styles = style.create({
    section: {
        gridTemplateRows: {
            default: "auto auto minmax(0, 1fr)",
            [screen.belowDesktop]: "auto",
        },
    },
    stage: {
        gridColumn: { default: "1 / 8", [screen.belowDesktop]: "1 / -1" },
        gridTemplateRows: "auto minmax(0, 1fr)",
    },
    key: {
        gridColumn: { default: "8 / -1", [screen.belowDesktop]: "1 / -1" },
        gridTemplateRows: "auto minmax(0, 1fr)",
    },
});
