import * as style from "@destack/style";

import { lattice } from "../../layout/lattice.stylex";
import { Entry, FigureLabel, type Form } from "../entry/entry";
import { LaunchBoard, services } from "./board";
import { Inspector } from "../figure/inspector";
import { Label } from "../entry/label";
import { createSteps } from "../figure/stagger";
import { StackSwitch } from "../entry/switch";
import { Window } from "../figure/window";
import { screen } from "../../layout/screen.stylex";
import { createSignal } from "@destack/view";
import { createReducedMotion } from "@destack/view/motion";

/** The milliseconds between the steps of the figure's switch, one per service. */
const stepTime = 80;

/** The adjective every app answers to. */
const destackable: Form = {
    syllables: ["de", "stack", "a", "ble"],
    pronunciation: "/diːˈstakəbl/",
    partOfSpeech: "adjective",
    sense: {
        definition: "standardised, integrated, malleable and built to trust",
        highlight: ["standardised", "integrated", "malleable"],
        sentence: "Every app is made from the same modern, standardised, integrated stack.",
    },
};

/** Show the adjective at work: a launch board asked of your agent, glued together from rented apps, or changed into your own Tasks app and wired to the rest of your space. */
export function Adjective(properties: { isOpen: boolean }) {
    // follow the motion the section shows
    const [section, setSection] = createSignal<HTMLElement>();
    const isReduced = createReducedMotion(section);

    // switch the chat first and then the app's parts in service order
    const { isOpenAt } = createSteps(
        () => properties.isOpen,
        services.length + 1,
        stepTime,
        isReduced,
    );

    return (
        <section
            ref={setSection}
            {...style.attrs(
                lattice.frame,
                lattice.ruled,
                lattice.section,
                lattice.ruleBottom,
                styles.section,
            )}
        >
            <Entry form={destackable} />
            <FigureLabel
                number={4}
                title={
                    properties.isOpen
                        ? "an app you change yourself, wired into everything"
                        : "an app only its vendor can change"
                }
            >
                <StackSwitch isOpen={properties.isOpen} />
            </FigureLabel>
            <div {...style.attrs(lattice.cell, lattice.figureCell, styles.stage)}>
                <Label>
                    {properties.isOpen
                        ? "A launch board, in your own Tasks"
                        : "A launch board, on their roadmap"}
                </Label>
                <Window
                    title={<span {...style.attrs(styles.title)}>Chat</span>}
                    xstyle={styles.window}
                >
                    <Inspector isOpenAt={isOpenAt} services={services}>
                        <LaunchBoard isOpenAt={isOpenAt} />
                    </Inspector>
                </Window>
            </div>
        </section>
    );
}

/** The adjective section styles. */
const styles = style.create({
    section: {
        gridTemplateRows: { default: "auto auto minmax(0, 1fr)", [screen.belowDesktop]: "auto" },
    },
    stage: {
        gridColumn: "1 / -1",
        gridTemplateRows: "auto minmax(0, 1fr)",
    },
    window: {
        minHeight: 0,
        height: { default: null, [screen.belowDesktop]: "32rem" },
    },
    title: {
        alignItems: "center",
        display: "inline-flex",
        flexGrow: 1,
        fontWeight: 600,
        gap: "0.375rem",
    },
});
