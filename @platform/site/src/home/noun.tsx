import * as stylex from "@destack/style";
import { createSignal } from "@destack/view";

import { lattice } from "../style/lattice.stylex";
import { Entry, FigureLabel, type Form } from "./entry";
import { Label } from "./label";
import { Ledger } from "./ledger";
import { Tile } from "./tile";
import { PagesApp, services } from "./page";
import { StackSwitch } from "./switch";
import { Window } from "./window";

/** The media query for screens narrower than the desktop frame, where the cells stack. */
const narrow = "@media (max-width: 1099px)";

/** The noun. */
const noun: Form = {
    syllables: ["De", "stack"],
    pronunciation: "/ˈdiːstak/",
    partOfSpeech: "noun",
    sense: {
        definition: "a software engine, absurdly integrated",
        highlight: ["software engine", "absurdly integrated"],
        sentence:
            "Integrated building blocks replace the thousand little rented silos every app glues together.",
    },
};

/** Show the noun at work: a docs app taken apart into the twelve services it runs on, rented one by one or built into one engine. */
export function Noun(properties: { isOpen: boolean }) {
    // hold the service pointed at, in the app or in the ledger
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
            <Entry form={noun} />
            <FigureLabel number={2} title="The humble modern docs app, deconstructed">
                <StackSwitch isOpen={properties.isOpen} />
            </FigureLabel>
            <div {...stylex.attrs(lattice.cell, lattice.figureCell, styles.stage)}>
                <Label>Pages, with the launch plan open</Label>
                <Window
                    title={
                        <span {...stylex.attrs(styles.title)}>
                            <Tile name="pages" />
                            Pages
                        </span>
                    }
                    style={styles.window}
                >
                    <PagesApp isOpen={properties.isOpen} lighting={lighting} />
                </Window>
            </div>
            <div {...stylex.attrs(lattice.cell, lattice.figureCell, styles.key)}>
                <Label>
                    {properties.isOpen ? "One software engine" : "Rented and glued together"}
                </Label>
                <Ledger
                    items={services.map((service) =>
                        properties.isOpen ? service.destacked : service.stacked,
                    )}
                    total={
                        properties.isOpen
                            ? ["1 engine", "1 account · 1 bill"]
                            : ["12 vendors", "12 accounts · 12 bills"]
                    }
                    lighting={lighting}
                    isOpen={properties.isOpen}
                />
            </div>
        </section>
    );
}

/** The noun section styles. */
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
        [narrow]: { height: "30rem" },
    },
    title: {
        alignItems: "center",
        display: "inline-flex",
        fontWeight: 600,
        gap: "0.375rem",
    },
});
