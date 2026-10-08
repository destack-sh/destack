import { frame } from "../../layout/frame.stylex";
import * as style from "@destack/style";

import { lattice } from "../../layout/lattice.stylex";
import { Entry, FigureLabel, type Form } from "../entry/entry";
import { Label } from "../entry/label";
import { StackFigure } from "./stack";
import { StackSwitch } from "../entry/switch";

/** The past participle, the state of having destacked. */
const destacked: Form = {
    syllables: ["de", "stacked"],
    pronunciation: "/diːˈstakt/",
    partOfSpeech: "past participle",
    sense: {
        definition: "unified into one stack you control",
        highlight: ["unified", "control"],
        sentence: "You, your people, apps, agents, data, and context in one unified place.",
    },
};

/** Show the past participle at work: your apps replaced in place, their separate stacks becoming one space, with the switch between both states. */
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
                    title={
                        properties.isOpen
                            ? "every app on one shared stack"
                            : "each app on its own separate stack"
                    }
                >
                    <StackSwitch isOpen={properties.isOpen} />
                </FigureLabel>
                <div {...style.attrs(lattice.cell, styles.labelCell, styles.drawingLabel)}>
                    <Label>Your apps, and everything under them</Label>
                </div>
                <div {...style.attrs(lattice.cell, styles.labelCell, styles.layersLabel)}>
                    <Label>Six layers of a stack</Label>
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
        gridColumn: { default: "1 / 9", "@media (max-width: 1099px)": "1 / -1" },
    },
    layersLabel: {
        gridColumn: "9 / -1",
        "@media (max-width: 1099px)": { display: "none" },
    },
});
