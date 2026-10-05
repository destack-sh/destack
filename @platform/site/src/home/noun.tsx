import * as stylex from "@destack/style";
import { createSignal } from "@destack/view";

import { lattice } from "../style/lattice.stylex";
import { Entry, FigureLabel, type Form } from "./entry";
import { Fade } from "./fade";
import { Inspector } from "./inspector";
import { Label } from "./label";
import { Ledger } from "./ledger";
import { Tile } from "./tile";
import { PagesApp, services } from "./page";
import { createStagger } from "./stagger";
import { StackSwitch } from "./switch";
import { Window } from "./window";

/** The media query for screens narrower than the desktop frame, where the cells stack. */
const narrow = "@media (max-width: 1099px)";

/** The milliseconds between the steps of the figure's switch, one per service. */
const stepTime = 70;

/** The noun. */
const noun: Form = {
    syllables: ["De", "stack"],
    pronunciation: "/ˈdiːstak/",
    partOfSpeech: "noun",
    sense: {
        definition: "a complete software engine, absurdly integrated",
        highlight: ["complete", "integrated"],
        sentence:
            "One open software engine with every library and service your apps need, fully standardised.",
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

    // switch the app's parts and the ledger's rows in service order before the rest
    const isOpenAt = createStagger(() => properties.isOpen, services.length + 1, stepTime);

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
            <FigureLabel number={2} title="A modern docs app and what it runs on">
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
                    <Inspector
                        isOpen={isOpenAt(services.length + 1)}
                        lighting={lighting}
                        services={services.map((service) => service.destacked.name)}
                    >
                        <PagesApp isOpenAt={isOpenAt} />
                    </Inspector>
                </Window>
            </div>
            <div {...stylex.attrs(lattice.cell, lattice.figureCell, styles.key)}>
                <Fade isOpen={isOpenAt(0)}>
                    <Label>
                        {isOpenAt(0) ? "One software engine" : "Rented and glued together"}
                    </Label>
                </Fade>
                <Ledger
                    entries={services}
                    total={{
                        stacked: ["12 vendors", "12 accounts · 12 bills"],
                        destacked: ["1 engine", "1 account · 1 bill"],
                    }}
                    lighting={lighting}
                    isOpenAt={isOpenAt}
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
