import { frame } from "../../layout/frame.stylex";
import * as style from "@destack/style";

import { lattice } from "../../layout/lattice.stylex";
import { Entry, FigureLabel, type Form } from "../entry/entry";
import { Label } from "../entry/label";
import { StackFigure } from "./stack";
import { StackSwitch } from "../entry/switch";
import { screen } from "../../layout/screen.stylex";

/** The past participle, the state of having destacked. */
const destacked: Form = {
    syllables: ["de", "stacked"],
    pronunciation: "/diːˈstakt/",
    partOfSpeech: "past participle",
    sense: {
        definition: "unified into one standardised stack",
        highlight: ["unified", "standardised"],
        sentence:
            "One account, one API, one data store, and open formats, from sign-in to hosting.",
    },
};

/** Show the past participle at work. */
export function PastParticiple(properties: {
    isOpen: boolean;
    onChange: (isOpen: boolean) => void;
}) {
    return (
        <>
            <section {...style.attrs(lattice.frame, lattice.ruled)}>
                <Entry form={destacked} />
                <FigureLabel
                    number={2}
                    isOpen={properties.isOpen}
                    stacked="each app with its own silo"
                    destacked="every app on the same standard stack"
                >
                    <StackSwitch isOpen={properties.isOpen} place="label" />
                </FigureLabel>
                <div {...style.attrs(lattice.cell, styles.labelCell, styles.drawingLabel)}>
                    <Label>Your apps, and what's under them</Label>
                </div>
                <div {...style.attrs(lattice.cell, styles.labelCell, styles.layersLabel)}>
                    <Label>Every layer of the stack</Label>
                </div>
            </section>
            <StackFigure onChange={properties.onChange} />
        </>
    );
}

/** The past participle styles. */
const styles = style.create({
    labelCell: {
        paddingBlockStart: "1.125rem",
        paddingBlockEnd: "0.375rem",
        paddingInline: frame.inset,
    },
    drawingLabel: {
        gridColumn: { default: "1 / 9", [screen.belowDesktop]: "1 / -1" },
    },
    layersLabel: {
        gridColumn: "9 / -1",
        display: { default: null, [screen.belowDesktop]: "none" },
    },
});
