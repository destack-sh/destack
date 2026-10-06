import * as stylex from "@destack/style";

import { lattice } from "../style/lattice.stylex";
import { tokens } from "../style/tokens.stylex";
import { Entry, FigureLabel, type Form } from "./entry";
import { Label } from "./label";
import { StackFigure } from "./figure";
import { StackSwitch } from "./switch";

/** The past participle, the state of having destacked. */
const destacked: Form = {
    syllables: ["de", "stacked"],
    pronunciation: "/diːˈstakt/",
    partOfSpeech: "past participle",
    sense: {
        definition: "unified into one stack you control",
        highlight: ["unified", "control"],
        sentence: "Your people, apps, data and context as one, without the hassle.",
    },
};

/** Show the past participle at work: your apps replaced in place, their separate stacks becoming one space, with the switch between both states. */
export function PastParticiple(properties: {
    isOpen: boolean;
    onChange: (isOpen: boolean) => void;
}) {
    return (
        <>
            <section {...stylex.attrs(lattice.frame, lattice.ruled)}>
                <Entry form={destacked} />
                <FigureLabel
                    number={2}
                    title={
                        properties.isOpen
                            ? "every app on one stack, all the way down"
                            : "each app on its own separate stack"
                    }
                >
                    <StackSwitch isOpen={properties.isOpen} />
                </FigureLabel>
                <div {...stylex.attrs(lattice.cell, styles.labelCell, styles.drawingLabel)}>
                    <Label>Your apps, and everything under them</Label>
                </div>
                <div {...stylex.attrs(lattice.cell, styles.labelCell, styles.layersLabel)}>
                    <Label>Six layers of a stack</Label>
                </div>
            </section>
            <StackFigure onChange={properties.onChange} />
        </>
    );
}

/** The past participle styles. */
const styles = stylex.create({
    labelCell: {
        paddingBlock: "1.125rem 0.375rem",
        paddingInline: tokens.inset,
    },
    drawingLabel: {
        gridColumn: { default: "1 / 9", "@media (max-width: 1099px)": "1 / -1" },
    },
    layersLabel: {
        gridColumn: "9 / -1",
        "@media (max-width: 1099px)": { display: "none" },
    },
});
