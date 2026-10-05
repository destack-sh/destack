import * as stylex from "@destack/style";

import { lattice } from "../style/lattice.stylex";
import { Entry, FigureLabel, type Form } from "./entry";
import { services, WeekApp } from "./week";
import { Inspector } from "./inspector";
import { Label } from "./label";
import { createStagger } from "./stagger";
import { StackSwitch } from "./switch";
import { Window } from "./window";

/** The milliseconds between the steps of the figure's switch, one per service. */
const stepTime = 150;

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
        sentence: "Made your way, yours to change, yours to keep, and built to trust.",
    },
};

/** Show the adjective at work: the same request answered as a chat artifact kept up by hand, or as an app in your space that works with the rest of it. */
export function Adjective(properties: { isOpen: boolean }) {
    // switch the chat first and then the app's parts in service order
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
            <Entry form={destackable} />
            <FigureLabel number={3} title="An app made by asking">
                <StackSwitch isOpen={properties.isOpen} />
            </FigureLabel>
            <div {...stylex.attrs(lattice.cell, lattice.figureCell, styles.stage)}>
                <Label>My week, built and changed in one chat</Label>
                <Window
                    title={
                        <span {...stylex.attrs(styles.title)}>
                            {isOpenAt(0) ? "Destack" : "Chat"}
                        </span>
                    }
                    style={styles.window}
                >
                    <Inspector isOpen={isOpenAt(services.length + 1)} services={services}>
                        <WeekApp isOpenAt={isOpenAt} />
                    </Inspector>
                </Window>
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
        gridColumn: "1 / -1",
        gridTemplateRows: "auto minmax(0, 1fr)",
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
});
