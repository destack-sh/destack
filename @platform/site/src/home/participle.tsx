import * as stylex from "@destack/style";

import { lattice } from "../style/lattice.stylex";
import { Entry, FigureLabel, type Form } from "./entry";
import { InstallWays } from "./install";

/** The participle, which closes the page as the verb opens it. */
const participle: Form = {
    syllables: ["de", "stack", "ing"],
    pronunciation: "/diːˈstakɪŋ/",
    partOfSpeech: "present participle",
    sense: {
        definition: "taking back control over your software stack",
        highlight: ["control"],
        sentence: "Owning your stack, without the hard parts of traditional software ownership.",
    },
};

/** Close the page as the hero opens it: the participle, over the three ways to install. */
export function Participle() {
    return (
        <section
            {...stylex.attrs(lattice.frame, lattice.ruled, lattice.ruleBottom, styles.section)}
        >
            <Entry form={participle} />
            <FigureLabel number={4} title="Install Destack" />
            <InstallWays />
        </section>
    );
}

/** The closing styles. */
const styles = stylex.create({
    section: {
        gridTemplateRows: "auto auto auto",
        "@media (max-width: 1099px)": { gridTemplateRows: "auto" },
    },
});
