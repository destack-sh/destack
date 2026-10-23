import * as style from "@destack/style";

import { lattice } from "../../layout/lattice.stylex";
import { Entry, FigureLabel, type Form } from "../entry/entry";
import { StackFigure } from "./stack";
import { StackSwitch } from "../entry/switch";

/** The past participle, the state of having destacked. */
const destacked: Form = {
    syllables: ["de", "stacked"],
    pronunciation: "/diːˈstakt/",
    partOfSpeech: "past participle",
    sense: {
        definition: "unified into one standardised stack",
        highlight: ["unified", "standardised"],
        sentence:
            "Every app on one account, one database, and one interface. Think GitHub + npm + Convex + Vercel + Sentry.",
        gloss: "snapped together like LEGO, every app into every other",
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
                    number={3}
                    isOpen={properties.isOpen}
                    stacked="every app on its own stack"
                    destacked="every app in one stack"
                >
                    <StackSwitch isOpen={properties.isOpen} place="label" />
                </FigureLabel>
            </section>
            <StackFigure onChange={properties.onChange} />
        </>
    );
}
