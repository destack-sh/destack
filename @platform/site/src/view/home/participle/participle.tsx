import * as style from "@destack/style";
import { createSignal } from "@destack/view";
import { createReducedMotion } from "@destack/view/motion";

import { lattice } from "../../layout/lattice.stylex";
import { screen } from "../../layout/screen.stylex";
import { Entry, FigureLabel, type Form } from "../entry/entry";
import { Label } from "../entry/label";
import { StackSwitch } from "../entry/switch";
import { Bill } from "../figure/bill";
import { createSteps } from "../figure/stagger";
import { InstallWays } from "./install";
import { lineCount, tallyOf, ToolWall } from "./wall";

/** The milliseconds between the steps of the bill's switch, one per line. */
const stepTime = 60;

/** The present participle, the act of destacking, which closes the page. */
const participle: Form = {
    syllables: ["de", "stack", "ing"],
    pronunciation: "/diːˈstakɪŋ/",
    partOfSpeech: "present participle",
    sense: {
        definition: "taking your software back, one app at a time",
        highlight: ["taking your software back"],
        sentence: "Start wherever you like.",
    },
};

/** Close the page on destacking: the tools you pick, the space they become, and how to install. */
export function Participle(properties: {
    isOpen: boolean;
    picks: readonly string[];
    onPick: (name: string) => void;
    prompt: string;
}) {
    // follow the motion the section shows, and step the bill's lines in order as the stack switches
    const [section, setSection] = createSignal<HTMLElement>();
    const isReduced = createReducedMotion(section);
    const { isOpenAt } = createSteps(() => properties.isOpen, lineCount, stepTime, isReduced);

    return (
        <>
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
                    stacked="the stack you rent today"
                    destacked="the stack you own instead"
                >
                    <StackSwitch isOpen={properties.isOpen} place="label" />
                </FigureLabel>
                <div {...style.attrs(lattice.cell, lattice.figureCell, styles.stage)}>
                    <Label>
                        {properties.isOpen
                            ? "Your own space in the cloud"
                            : "Pick your current stack"}
                    </Label>
                    <ToolWall
                        picks={properties.picks}
                        isOpen={properties.isOpen}
                        onPick={properties.onPick}
                    />
                </div>
                <div {...style.attrs(lattice.cell, lattice.figureCell, styles.key)}>
                    <Label>{properties.isOpen ? "Your space" : "Your current stack"}</Label>
                    <Bill lines={tallyOf(properties.picks)} isOpenAt={isOpenAt} />
                </div>
            </section>

            {/* install in one of three ways, the agent's prompt naming the picked tools */}
            <section {...style.attrs(lattice.frame, lattice.ruled, lattice.ruleBottom)}>
                <InstallWays prompt={properties.prompt} />
            </section>
        </>
    );
}

/** The present participle section styles. */
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
