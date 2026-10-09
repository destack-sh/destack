import * as style from "@destack/style";

import { lattice } from "../../layout/lattice.stylex";
import { Entry, FigureLabel, type Form } from "../entry/entry";
import { Label } from "../entry/label";
import { StackSwitch } from "../entry/switch";
import { tallyOf, ToolWall, totalOf } from "./wall";
import { Ledger } from "../figure/ledger";
import { screen } from "../../layout/screen.stylex";

/** The participle, the last of the word's forms before the page closes. */
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

/** Show the participle at work: the tools you pick, tallied, and the space they become. */
export function Participle(properties: {
    isOpen: boolean;
    picks: readonly string[];
    onPick: (name: string) => void;
}) {
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
                <Ledger
                    entries={tallyOf(properties.picks)}
                    total={totalOf(properties.picks, properties.isOpen)}
                    isOpenAt={() => properties.isOpen}
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
