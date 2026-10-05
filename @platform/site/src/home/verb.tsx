import * as stylex from "@destack/style";

import { lattice } from "../style/lattice.stylex";
import { Entry, FigureLabel, type Form } from "./entry";
import { StackSwitch } from "./switch";
import { StackFigure } from "./figure";

/** The verb, which titles the page. */
const verb: Form = {
    syllables: ["de", "stack"],
    pronunciation: "/diːˈstak/",
    partOfSpeech: "verb",
    sense: {
        definition: "to take back your software",
        highlight: ["back"],
        sentence:
            "Build and run every app on one open, standardised stack, on your machine, keeping your data yours.",
    },
};

/** Open the page with the verb, illustrated by the stack figure and the switch between its two states. */
export function Verb(properties: { isOpen: boolean; onChange: (isOpen: boolean) => void }) {
    return (
        <>
            <section {...stylex.attrs(lattice.frame, lattice.ruled)}>
                <Entry form={verb} isTitle />
                <FigureLabel number={1} title="The same apps, on rented silos and on Destack">
                    <StackSwitch isOpen={properties.isOpen} />
                </FigureLabel>
            </section>
            <StackFigure onChange={properties.onChange} />
        </>
    );
}
