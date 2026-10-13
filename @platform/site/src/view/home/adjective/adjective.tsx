import { media } from "@destack/style/media.stylex";
import * as style from "@destack/style";

import { lattice } from "../../layout/lattice.stylex";
import { screen } from "../../layout/screen.stylex";
import { Entry, FigureLabel, type Form } from "../entry/entry";
import { Label } from "../entry/label";
import { StackSwitch } from "../entry/switch";
import { Inspector } from "./inspect";

/** The adjective every app answers to. */
const destackable: Form = {
    syllables: ["de", "stack", "a", "ble"],
    pronunciation: "/diːˈstakəbl/",
    partOfSpeech: "adjective",
    sense: {
        definition: "open, transparent, and hackable",
        highlight: ["transparent", "hackable"],
        sentence: "See everything every app and agent does, and change it.",
    },
};

/** Show the adjective at work: your launch as rented dashboards see it, or as your space does. */
export function Adjective(properties: { isOpen: boolean }) {
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
            <Entry form={destackable} />
            <FigureLabel
                number={4}
                isOpen={properties.isOpen}
                stacked="one invite, six consoles"
                destacked="one invite, one trace"
            >
                <StackSwitch isOpen={properties.isOpen} place="label" />
            </FigureLabel>
            <div {...style.attrs(lattice.cell, lattice.figureCell, styles.stage)}>
                <Label>
                    {properties.isOpen
                        ? "Inviting 400 from the waitlist, as your space shows it"
                        : "Inviting 400 from the waitlist, as six vendors show it"}
                </Label>
                <Inspector isOpen={properties.isOpen} />
            </div>
        </section>
    );
}

/** The adjective section styles. */
const styles = style.create({
    section: {
        gridTemplateRows: { default: "auto auto minmax(0, 1fr)", [screen.belowDesktop]: "auto" },
    },
    stage: {
        containerType: "inline-size",
        gridColumn: "1 / -1",
        gridTemplateRows: { default: "auto minmax(0, 1fr)", [media.maxMd]: "auto 39rem" },
    },
});
