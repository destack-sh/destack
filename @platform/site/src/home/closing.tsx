import * as stylex from "@destack/style";

import { lattice } from "../style/lattice.stylex";
import { tokens } from "../style/tokens.stylex";
import { Entry, type Form } from "./entry";
import { InstallFrames } from "./install";

/** The participle, which closes the page as the verb opens it. */
const participle: Form = {
    syllables: ["de", "stack", "ing"],
    pronunciation: "/diːˈstakɪŋ/",
    partOfSpeech: "present participle",
    senses: [
        {
            definition: "taking back your software",
            highlight: ["back"],
            sentence: "Running your apps on your own engine, with your agents on all of it.",
        },
    ],
};

/** Close the page as the hero opens it: the participle, with the three ways to install framed under it. */
export function Closing() {
    return (
        <section data-universe {...stylex.attrs(lattice.frame, lattice.ruleBottom, styles.section)}>
            <Entry form={participle} />
            <InstallFrames />
        </section>
    );
}

/** The closing styles. */
const styles = stylex.create({
    section: {
        gridTemplateRows: `auto calc(${tokens.stage} * 2.5)`,
        "@media (max-width: 1099px)": { gridTemplateRows: "auto" },
    },
});
