import { present } from "@destack/schema";
import * as style from "@destack/style";
import { createSignal } from "@destack/view";
import { createReducedMotion } from "@destack/view/motion";
import { createVisibilityObserver } from "@destack/view/primitives/intersection-observer";
import { createTimer } from "@destack/view/primitives/timer";

import { lattice } from "../../layout/lattice.stylex";
import { screen } from "../../layout/screen.stylex";
import { Entry, FigureLabel, type Form } from "../entry/entry";
import { Label } from "../entry/label";
import { StackSwitch } from "../entry/switch";
import { pages, SpaceBrowser, type Viewer } from "../verb/space";
import { AgentChat, assistants } from "./chat";
import { turns } from "./scene";

/** The milliseconds a beat takes. */
const BEAT = 500;

/** The beats each scene stays open: the call, its result landing, then time to read both. */
const SCENE_BEATS = 12;

/** The beat within a scene at which your space's call lands. */
const LANDING = 5;

/** The beat within a scene at which the chat takes the next assistant's look. */
const RESKIN = 9;

/** The command every app answers to, as an imperative. */
const command: Form = {
    syllables: ["de", "stack!"],
    pronunciation: "/diːˈstak/",
    partOfSpeech: "imperative",
    sense: {
        definition: "every app, fully programmable",
        highlight: ["fully programmable"],
        sentence: "One API across your whole space, for you and your agents, within your rules.",
    },
};

/** Show the imperative at work: an app beside a chat with your agent, a turn per app. */
export function Command(properties: { isOpen: boolean }) {
    // hold the section, the motion it may show, and whether it is on screen
    const [section, setSection] = createSignal<HTMLElement>();
    const isReduced = createReducedMotion(section);
    const isShown = createVisibilityObserver(section, { initialValue: true });

    // count beats, held while still or off screen, landing each scene's call partway through
    const [beat, setBeat] = createSignal(0);
    createTimer(
        () => setBeat((current) => current + 1),
        () => (!isReduced() && isShown() ? BEAT : false),
        setInterval,
    );
    const isLanded = () => isReduced() || beat() % SCENE_BEATS >= LANDING;

    // open each scene in turn, until the reader opens one
    const [chosen, setChosen] = createSignal<number>();
    const scene = () => chosen() ?? Math.floor(beat() / SCENE_BEATS) % turns.length;

    // take each assistant's look in turn, a few beats before each new turn
    const look = () =>
        assistants[Math.floor((beat() + SCENE_BEATS - RESKIN) / SCENE_BEATS) % assistants.length] ??
        "Claude";

    // show the open scene's app as fig. 1 shows it, and choose a scene from its tabs
    const [viewer, setViewer] = createSignal<Viewer>("Me");
    const page = () =>
        present(
            pages.find((candidate) => candidate.row === turns[scene()]?.row),
            "scene page",
        );
    const chooseRow = (row: number | undefined) => {
        const index = turns.findIndex((candidate) => candidate.row === row);
        if (index >= 0) {
            setChosen(index);
        }
    };

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
            <Entry form={command} />
            <FigureLabel
                number={5}
                isOpen={properties.isOpen}
                stacked="four apps, four APIs"
                destacked="every app, one API"
            >
                <StackSwitch isOpen={properties.isOpen} place="label" />
            </FigureLabel>
            <div {...style.attrs(lattice.cell, lattice.figureCell, styles.app)}>
                <Label>
                    {properties.isOpen ? "Every app on one API" : "Every app behind its own API"}
                </Label>
                <SpaceBrowser
                    name="interface"
                    rows={turns.map((candidate) => candidate.row)}
                    isChanged={properties.isOpen && isLanded()}
                    isOpenAt={() => properties.isOpen}
                    page={page()}
                    viewer={viewer()}
                    onViewer={setViewer}
                    onChoose={chooseRow}
                />
            </div>
            <div {...style.attrs(lattice.cell, lattice.figureCell, styles.console)}>
                <Label>
                    {properties.isOpen
                        ? "Your agent, one universal integration"
                        : "Your agent, one connector per app"}
                </Label>
                <AgentChat
                    isOpen={properties.isOpen}
                    turn={scene()}
                    isLanded={isLanded()}
                    assistant={look()}
                />
            </div>
        </section>
    );
}

/** The interface section styles. */
const styles = style.create({
    section: {
        gridTemplateRows: {
            default: "auto auto minmax(0, 1fr)",
            [screen.belowDesktop]: "auto",
        },
    },
    app: {
        gridColumn: { default: "1 / 8", [screen.belowDesktop]: "1 / -1" },
        gridTemplateRows: "auto minmax(0, 1fr)",
        minHeight: { default: 0, [screen.belowDesktop]: "24rem" },
    },
    console: {
        gridColumn: { default: "8 / -1", [screen.belowDesktop]: "1 / -1" },
        gridTemplateRows: "auto minmax(0, 1fr)",
        minHeight: { default: 0, [screen.belowDesktop]: "22rem" },
    },
});
