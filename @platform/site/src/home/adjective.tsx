import * as stylex from "@destack/style";
import { createSignal } from "@destack/view";

import { lattice } from "../style/lattice.stylex";
import { tokens } from "../style/tokens.stylex";
import { Entry, FigureLabel, type Form } from "./entry";
import { BudgetApp, moments } from "./budget";
import { Label } from "./label";
import { Ledger } from "./ledger";
import { StackSwitch } from "./switch";
import { Window } from "./window";

/** The media query for screens narrower than the desktop frame, where the cells stack. */
const narrow = "@media (max-width: 1099px)";

/** The adjective every app answers to. */
const destackable: Form = {
    syllables: ["de", "stack", "a", "ble"],
    pronunciation: "/diːˈstakəbl/",
    partOfSpeech: "adjective",
    sense: {
        definition: "malleable, just-in-time software you can trust",
        highlight: ["malleable", "just-in-time"],
        sentence: "Make any app your way, and trust the ones you’re sent.",
    },
};

/** Show the adjective at work: the same request answered as a chat artifact, or as an app in your space remixed from standard components. */
export function Adjective(properties: { isOpen: boolean }) {
    // hold the part pointed at, in the app or in the ledger
    const [lit, setLit] = createSignal<number | undefined>(undefined);
    const lighting = {
        get lit() {
            return lit();
        },
        onLight: setLit,
    };

    return (
        <section
            {...stylex.attrs(
                lattice.frame,
                lattice.ruled,
                lattice.section,
                lattice.ruleBottom,
                styles.section,
            )}
        >
            <Entry form={destackable} />
            <FigureLabel
                number={3}
                title="The same request, as a chat artifact and as an app in your space"
            >
                <StackSwitch isOpen={properties.isOpen} />
            </FigureLabel>
            <div {...stylex.attrs(lattice.cell, lattice.figureCell, styles.stage)}>
                <Label>A household budget, built on request</Label>
                <Window
                    title={
                        <span {...stylex.attrs(styles.title)}>
                            {properties.isOpen ? "Budget" : "Chat"}
                            <span
                                {...stylex.attrs(
                                    styles.branch,
                                    properties.isOpen && styles.branchOn,
                                )}
                            >
                                {properties.isOpen ? "branch budget" : "artifact"}
                            </span>
                        </span>
                    }
                    style={styles.window}
                >
                    <BudgetApp isOpen={properties.isOpen} lighting={lighting} />
                </Window>
            </div>
            <div {...stylex.attrs(lattice.cell, lattice.figureCell, styles.key)}>
                <Label>
                    {properties.isOpen
                        ? "Its life as an app in your space"
                        : "Its life as a chat artifact"}
                </Label>
                <Ledger
                    items={moments.map((moment) =>
                        properties.isOpen ? moment.destacked : moment.stacked,
                    )}
                    total={
                        properties.isOpen
                            ? ["1 app", "every change reviewed"]
                            : ["1 chat", "every change starts over"]
                    }
                    lighting={lighting}
                    isOpen={properties.isOpen}
                    isSingle
                    isNoted
                />
            </div>
        </section>
    );
}

/** The adjective section styles. */
const styles = stylex.create({
    section: {
        gridTemplateRows: "auto auto minmax(0, 1fr)",
        [narrow]: { gridTemplateRows: "auto" },
    },
    stage: {
        gridColumn: "1 / 8",
        gridTemplateRows: "auto minmax(0, 1fr)",
        [narrow]: { gridColumn: "1 / -1" },
    },
    key: {
        gridColumn: "8 / -1",
        gridTemplateRows: "auto minmax(0, 1fr)",
        [narrow]: { gridColumn: "1 / -1" },
    },
    window: {
        minHeight: 0,
        [narrow]: { height: "32rem" },
    },
    title: {
        alignItems: "center",
        display: "inline-flex",
        flexGrow: 1,
        fontWeight: 600,
        gap: "0.375rem",
    },
    branch: {
        borderColor: tokens.rule,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        fontFamily: tokens.monoFont,
        fontSize: "0.72rem",
        fontWeight: 400,
        marginLeft: "auto",
        paddingBlock: "1px",
        paddingInline: "0.625rem",
    },
    branchOn: {
        backgroundColor: "rgb(255 121 46 / 16%)",
        borderColor: tokens.signal,
    },
});
