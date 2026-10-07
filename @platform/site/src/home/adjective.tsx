import * as stylex from "@destack/style";

import { lattice } from "../style/lattice.stylex";
import { Entry, FigureLabel, type Form } from "./entry";
import { services, WeekApp } from "./week";
import { Inspector } from "./inspector";
import { Label } from "./label";
import { createSteps } from "./stagger";
import { StackSwitch } from "./switch";
import { Window } from "./window";

/** The milliseconds between the steps of the figure's switch, one per service. */
const stepTime = 80;

/** The media query for screens narrower than the desktop frame, where the cells stack. */
const narrow = "@media (max-width: 1099px)";

/** The adjective every app answers to. */
const destackable: Form = {
    syllables: ["de", "stack", "a", "ble"],
    pronunciation: "/diːˈstakəbl/",
    partOfSpeech: "adjective",
    sense: {
        definition: "malleable, standardised, and built to trust",
        highlight: ["malleable", "standardised"],
        sentence: "Every app is made from the same modern, integrated, standardised stack.",
    },
};

/** Show the adjective at work: the same request answered as a chat artifact kept up by hand, or as an app in your space that works with the rest of it. */
export function Adjective(properties: { isOpen: boolean }) {
    // switch the chat first and then the app's parts in service order
    const { isOpenAt } = createSteps(() => properties.isOpen, services.length + 1, stepTime);

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
                title={
                    properties.isOpen
                        ? "an app you asked for, and can keep remixing"
                        : "an app you can only regenerate, not (really) change"
                }
            >
                <StackSwitch isOpen={properties.isOpen} />
            </FigureLabel>
            <div {...stylex.attrs(lattice.cell, lattice.figureCell, styles.stage)}>
                <Label>My week, in a chat</Label>
                <Window
                    title={<span {...stylex.attrs(styles.title)}>Chat</span>}
                    style={styles.window}
                >
                    <Inspector isOpenAt={isOpenAt} services={services}>
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
