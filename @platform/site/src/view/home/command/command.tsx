import { present } from "@destack/schema";
import * as style from "@destack/style";
import { createEffect, createSignal } from "@destack/view";
import { createReducedMotion } from "@destack/view/motion";
import { createVisibilityObserver } from "@destack/view/primitives/intersection-observer";
import { createTimer } from "@destack/view/primitives/timer";

import { lattice } from "../../layout/lattice.stylex";
import { screen } from "../../layout/screen.stylex";
import { Entry, FigureLabel, type Form } from "../entry/entry";
import { Label } from "../entry/label";
import { StackSwitch } from "../entry/switch";
import { pages, SpaceBrowser, type Vendor, type Viewer } from "../verb/space";
import { opensRequests, ResendLogs } from "../adjective/console";
import { AgentChat, assistants } from "./chat";
import { destacked, stacked } from "./scene";
import { sources } from "../verb/source";

/** The milliseconds a beat takes. */
const BEAT = 500;

/** The beats each step of the task stays before the next one lands. */
const STEP_BEATS = 3;

/** The beats the finished task holds before it runs again. */
const HOLD_BEATS = 12;

/** The rows of fig. 1 the task touches: Chat, Mail and the waitlist. */
const ROWS: readonly number[] = [3, 7, 8];

/** The mail service the agent reaches through its connector, in place of fig. 1's mail. */
const resend: Vendor = {
    row: 7,
    icon: "resend",
    tint: "#000000",
    title: "Logs · Resend",
    site: "resend.com/logs",
    page: () => <ResendLogs requests={opensRequests} />,
};

/** The command every app answers to, as an imperative. */
const command: Form = {
    syllables: ["de", "stack!"],
    pronunciation: "/diːˈstak/",
    partOfSpeech: "imperative",
    sense: {
        definition: "every app, programmable and composable",
        highlight: ["programmable", "composable"],
        sentence: "One API across your whole space, for you and your agents, within your rules.",
    },
};

/** Show the imperative at work: an app beside a chat with your agent, a turn per app. */
export function Command(properties: { isOpen: boolean }) {
    // hold the section, the motion it may show, and whether it is on screen
    const [section, setSection] = createSignal<HTMLElement>();
    const isReduced = createReducedMotion(section);
    const isShown = createVisibilityObserver(section, { initialValue: true });

    // count beats, held while still or off screen, and start the task again on every switch
    const [beat, setBeat] = createSignal(0);
    createTimer(
        () => setBeat((current) => current + 1),
        () => (!isReduced() && isShown() ? BEAT : false),
        setInterval,
    );
    createEffect(
        () => properties.isOpen,
        () => {
            setBeat(0);
        },
    );

    // run the task in the state the stack is in, a step every few beats, then hold and run again
    const steps = () => (properties.isOpen ? destacked : stacked);
    const run = () => steps().length * STEP_BEATS + HOLD_BEATS;
    const shown = () =>
        isReduced()
            ? steps().length
            : Math.min(steps().length, Math.floor((beat() % run()) / STEP_BEATS) + 1);
    const step = () => present(steps()[shown() - 1], "shown step");

    // show the chat up to the step reached, an approval replaced once its outcome lands
    const items = () => {
        const reached = steps()
            .slice(0, shown())
            .map((candidate) => candidate.item);
        const isApproved = reached.some((item) => item.kind === "approval" && item.isApproved);

        return reached.filter(
            (item) => !(item.kind === "approval" && !item.isApproved && isApproved),
        );
    };

    // take the next assistant's look each time the task runs again
    const look = () => assistants[Math.floor(beat() / run()) % assistants.length] ?? "Claude";

    // show the page the reader opens from a tab, or else the page of fig. 1 the step touches
    const [viewer, setViewer] = createSignal<Viewer>("Me");
    const [opened, setOpened] = createSignal<number | undefined>(undefined);
    const row = () => opened() ?? step().row;
    const page = () =>
        present(
            pages.find((candidate) => candidate.row === row()),
            "step page",
        );

    // let the task take the page back when it runs again
    createEffect(
        () => Math.floor(beat() / run()),
        () => {
            setOpened(undefined);
        },
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
            <Entry form={command} />
            <FigureLabel
                number={5}
                isOpen={properties.isOpen}
                stacked="three apps, three connectors"
                destacked="three apps, one space"
            >
                <StackSwitch isOpen={properties.isOpen} place="label" />
            </FigureLabel>
            <div {...style.attrs(lattice.cell, lattice.figureCell, styles.app)}>
                <Label>
                    {properties.isOpen ? "Every app on one API" : "Every app behind its own API"}
                </Label>
                <SpaceBrowser
                    name="interface"
                    rows={ROWS}
                    isChanged={step().isChanged && row() === step().row}
                    source={present(sources[0], "first source")}
                    vendor={resend}
                    remix={undefined}
                    isOpenAt={() => properties.isOpen}
                    page={page()}
                    viewer={viewer()}
                    onViewer={setViewer}
                    onChoose={setOpened}
                />
            </div>
            <div {...style.attrs(lattice.cell, lattice.figureCell, styles.console)}>
                <Label>
                    {properties.isOpen
                        ? "Your agent, one integration for everything"
                        : "Your agent, one connector per app"}
                </Label>
                <AgentChat isOpen={properties.isOpen} items={items()} assistant={look()} />
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
