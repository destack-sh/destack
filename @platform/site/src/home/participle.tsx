import * as stylex from "@destack/style";
import { createSignal } from "@destack/view";

import { lattice } from "../style/lattice.stylex";
import { Entry, FigureLabel, type Form } from "./entry";
import { InstallWays } from "./install";
import { Label } from "./label";
import { StackSwitch } from "./switch";
import { countOf, firstPicks, promptOf, ToolWall } from "./wall";

/** The participle, which closes the page as the verb opens it. */
const participle: Form = {
    syllables: ["de", "stack", "ing"],
    pronunciation: "/diːˈstakɪŋ/",
    partOfSpeech: "present participle",
    sense: {
        definition: "taking back control over your software stack",
        highlight: ["control"],
        sentence: "Start with one app, then a second, and keep going as long as you like.",
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
            {...stylex.attrs(
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
                        ? "the same stack, owned and integrated"
                        : "the fragmented stack you use today"
                }
            >
                <StackSwitch isOpen={properties.isOpen} />
            </FigureLabel>
            <div {...stylex.attrs(lattice.cell, lattice.figureCell, styles.wall)}>
                <Label>
                    {properties.isOpen ? "Your space" : "Pick your current stack"} ·{" "}
                    {countOf(picks(), properties.isOpen)}
                </Label>
                <ToolWall picks={picks()} isOpen={properties.isOpen} onPick={pick} />
            </div>
            <InstallWays prompt={promptOf(picks())} />
        </section>
    );
}

/** The closing styles. */
const styles = stylex.create({
    section: {
        gridTemplateRows: "auto auto minmax(0, 1fr) auto",
        "@media (max-width: 1099px)": { gridTemplateRows: "auto" },
    },
    wall: {
        gridColumn: "1 / -1",
        gridTemplateRows: "auto minmax(0, 1fr)",
    },
});
