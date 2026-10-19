import * as style from "@destack/style";
import { createSignal } from "@destack/view";
import { createReducedMotion } from "@destack/view/motion";

import { lattice } from "../../layout/lattice.stylex";
import { screen } from "../../layout/screen.stylex";
import { Entry, FigureLabel, type Form } from "../entry/entry";
import { Label, LabelRow } from "../entry/label";
import { Dial, teams } from "../figure/dial";
import { StackSwitch } from "../entry/switch";
import { Bill } from "../figure/bill";
import { BASE_USERS } from "../figure/price";
import { createSteps } from "../figure/stagger";
import { InstallWays } from "./install";
import type { Tool } from "../tool/tool";
import { lineCount, tallyOf, ToolWall } from "./wall";

/** The milliseconds between the steps of the bill's switch, one per line. */
const stepTime = 60;

/** The present participle, the act of destacking, which closes the page. */
const participle: Form = {
    syllables: ["de", "stack", "ing"],
    pronunciation: "/diːˈstakɪŋ/",
    partOfSpeech: "present participle",
    sense: {
        definition: "making all your software work as one",
        highlight: ["work as one"],
        sentence: "Begin with the tools you know, then remix them your way.",
    },
};

/** Close the page on destacking: the tools you pick, the space they become, and how to install. */
export function Participle(properties: {
    isOpen: boolean;
    picks: readonly Tool[];
    onPick: (tool: Tool) => void;
    prompt: string;
}) {
    // follow the motion the section shows, and step the bill's lines in order as the stack switches
    const [section, setSection] = createSignal<HTMLElement>();
    const isReduced = createReducedMotion(section);
    const { isOpenAt } = createSteps(() => properties.isOpen, lineCount, stepTime, isReduced);

    // read the bill for a team of the size the reader dials
    const [people, setPeople] = createSignal<number>(5);

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
                    number={6}
                    isOpen={properties.isOpen}
                    stacked="your tools, each on its own"
                    destacked="your tools, in one space"
                >
                    <StackSwitch isOpen={properties.isOpen} place="label" />
                </FigureLabel>
                <div {...style.attrs(lattice.cell, lattice.figureCell, styles.stage)}>
                    <Label>{properties.isOpen ? "Your tools, owned" : "Your tools, rented"}</Label>
                    <ToolWall
                        picks={properties.picks}
                        isOpen={properties.isOpen}
                        onPick={properties.onPick}
                    />
                </div>
                <div {...style.attrs(lattice.cell, lattice.figureCell, styles.key)}>
                    <LabelRow
                        end={
                            <Dial
                                label="People"
                                options={teams}
                                value={people()}
                                format={String}
                                onChange={setPeople}
                            />
                        }
                    >
                        <Label>{properties.isOpen ? "Paid by usage" : "Paid per seat"}</Label>
                    </LabelRow>
                    <Bill
                        isWindow
                        lines={tallyOf(properties.picks)}
                        scale={{ people: people(), users: BASE_USERS }}
                        scales={teams.map((team) => ({ people: team, users: BASE_USERS }))}
                        isOpenAt={isOpenAt}
                    />
                </div>
            </section>

            {/* install in one of three ways, the agent's prompt naming the picked tools */}
            <section
                id="install"
                {...style.attrs(lattice.frame, lattice.ruled, lattice.ruleBottom)}
            >
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
