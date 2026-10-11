import * as style from "@destack/style";

import { lattice } from "../../layout/lattice.stylex";
import { Entry, type Form } from "../entry/entry";
import { InstallWays } from "./install";

/** The imperative, which closes the page on the verb it opens with. */
const imperative: Form = {
    syllables: ["de", "stack"],
    pronunciation: "/diːˈstak/",
    partOfSpeech: "imperative",
    sense: {
        definition: "destack your first app",
        highlight: ["your first app"],
        sentence: "Pick one way in. Everything else can follow.",
    },
};

/** Close the page on the verb as an instruction, over the three ways to install, the agent's prompt naming the tools you picked. */
export function Imperative(properties: { prompt: string }) {
    return (
        <section {...style.attrs(lattice.frame, lattice.ruled, lattice.ruleBottom)}>
            <Entry form={imperative} />
            <InstallWays prompt={properties.prompt} />
        </section>
    );
}
