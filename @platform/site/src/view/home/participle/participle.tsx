import { media } from "@destack/style/media.stylex";
import * as style from "@destack/style";

import { lattice } from "../../layout/lattice.stylex";
import { Constellation } from "../drawing/constellation";
import { Entry, type Form } from "../entry/entry";
import type { Tool } from "../tool/tool";
import { Tally } from "./tally";

/** The present participle, the act of destacking, which closes the page. */
export const participle: Form = {
    syllables: ["de", "stack", "ing"],
    pronunciation: "dee-stack-ing",
    partOfSpeech: "present participle",
    sense: {
        definition: "making your software work as one",
        highlight: ["work as one"],
        sentence: "Move in one app at a time, or all at once: just ask your agent.",
    },
};

/** Close the page on destacking: the tools you pick and the space they become. */
export function Participle(properties: {
    isOpen: boolean;
    picks: readonly Tool[];
    onPick: (tool: Tool) => void;
}) {
    return (
        <section {...style.attrs(lattice.frame, lattice.ruled, lattice.section, styles.section)}>
            <Entry form={participle} />
            <Constellation isOpen={properties.isOpen} />
            <Tally
                number={5}
                stacked="your apps, fragmented, theirs to keep"
                destacked="your apps, unified, yours to keep"
                isOpen={properties.isOpen}
                picks={properties.picks}
                onPick={properties.onPick}
            />
        </section>
    );
}

/** The present participle section styles. */
const styles = style.create({
    section: {
        gridTemplateRows: {
            default: "auto auto minmax(0, 1fr)",
            [media.maxLg]: "auto",
        },
    },
});
