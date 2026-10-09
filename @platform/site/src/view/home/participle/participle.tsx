import * as style from "@destack/style";
import { createSignal } from "@destack/view";

import { lattice } from "../../layout/lattice.stylex";
import { Entry, FigureLabel, type Form } from "../entry/entry";
import { InstallWays } from "./install";
import { Label } from "../entry/label";
import { StackSwitch } from "../entry/switch";
import { countOf, firstPicks, promptOf, ToolWall } from "./wall";
import { screen } from "../../layout/screen.stylex";

/** The participle, which closes the page as the verb opens it. */
const participle: Form = {
    syllables: ["de", "stack", "ing"],
    pronunciation: "/diːˈstakɪŋ/",
    partOfSpeech: "present participle",
    sense: {
        definition: "taking back your stack, one app at a time",
        highlight: ["one app at a time"],
        sentence: "Start with one app, bring your own agent, and keep going as far as you want.",
    },
};

/** Close the page as the hero opens it: the participle, the tools you pick and the space they become, and the ways to install under them. */
export function Participle(properties: { isOpen: boolean }) {
    // hold the tools the reader uses, starting from the first figure's six
    const [picks, setPicks] = createSignal<readonly string[]>(firstPicks);
    const pick = (name: string) =>
        setPicks((current) =>
            current.includes(name)
                ? current.filter((picked) => picked !== name)
                : [...current, name],
        );

    return (
        <section
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
                title={
                    properties.isOpen
                        ? "the integrated stack you own instead"
                        : "the fragmented stack you rent today"
                }
            >
                <StackSwitch isOpen={properties.isOpen} />
            </FigureLabel>
            <div {...style.attrs(lattice.cell, lattice.figureCell, styles.wall)}>
                <Label>
                    {properties.isOpen ? "Your space" : "Pick your stack"} ·{" "}
                    {countOf(picks(), properties.isOpen)}
                </Label>
                <ToolWall picks={picks()} isOpen={properties.isOpen} onPick={pick} />
            </div>
            <InstallWays prompt={promptOf(picks())} />
        </section>
    );
}

/** The closing styles. */
const styles = style.create({
    section: {
        gridTemplateRows: {
            default: "auto auto minmax(0, 1fr) auto",
            [screen.belowDesktop]: "auto",
        },
    },
    wall: {
        gridColumn: "1 / -1",
        gridTemplateRows: "auto minmax(0, 1fr)",
    },
});
