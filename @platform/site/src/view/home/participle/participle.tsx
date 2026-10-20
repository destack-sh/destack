import * as style from "@destack/style";

import { lattice } from "../../layout/lattice.stylex";
import { screen } from "../../layout/screen.stylex";
import { Entry, type Form } from "../entry/entry";
import { InstallWays } from "./install";
import type { Tool } from "../tool/tool";
import { Tally } from "./tally";

/** The present participle, the act of destacking, which closes the page. */
const participle: Form = {
    syllables: ["de", "stack", "ing"],
    pronunciation: "/diːˈstakɪŋ/",
    partOfSpeech: "present participle",
    sense: {
        definition: "making all your software work as one",
        highlight: ["work as one"],
        sentence: "Moving in takes just one prompt, one app at a time.",
    },
};

/** Close the page on destacking: the tools you pick, the space they become, and how to install. */
export function Participle(properties: {
    isOpen: boolean;
    picks: readonly Tool[];
    onPick: (tool: Tool) => void;
    prompt: string;
}) {
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
            <Entry form={participle} />
            <Tally
                number={6}
                stacked="your tools, each on its own"
                destacked="your tools, in one space"
                isOpen={properties.isOpen}
                picks={properties.picks}
                onPick={properties.onPick}
            />

            {/* end on the install, in one of three ways, the agent's prompt naming the picked tools */}
            <div {...style.attrs(styles.install)}>
                <InstallWays prompt={properties.prompt} />
            </div>
        </section>
    );
}

/** The present participle section styles. */
const styles = style.create({
    section: {
        gridTemplateRows: {
            default: "auto auto minmax(0, 1fr) auto",
            [screen.belowDesktop]: "auto",
        },
    },
    install: {
        gridColumn: "1 / -1",
    },
});
