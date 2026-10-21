import * as style from "@destack/style";
import { createEffect, createSignal } from "@destack/view";
import { createReducedMotion } from "@destack/view/motion";
import { createVisibilityObserver } from "@destack/view/primitives/intersection-observer";
import { createTimer } from "@destack/view/primitives/timer";

import { lattice } from "../../layout/lattice.stylex";
import { screen } from "../../layout/screen.stylex";
import { Entry, FigureLabel, type Form } from "../entry/entry";
import { Label, LabelRow } from "../entry/label";
import { Picker } from "../figure/picker";
import { AgentChat, assistantOptions, createAssistantTurn } from "../command/chat";
import { BEAT } from "../command/scene";
import { StackSwitch } from "../entry/switch";
import { type Console, Inspector } from "./inspect";
import {
    destackedChatAt,
    destackedScene,
    RUN_LENGTH,
    runMomentAt,
    stackedConnectors,
    stackedScene,
} from "./timeline";

/** The adjective every app answers to. */
const destackable: Form = {
    syllables: ["de", "stack", "a", "ble"],
    pronunciation: "/diːˈstakəbl/",
    partOfSpeech: "adjective",
    sense: {
        definition: "open, transparent, and hackable",
        highlight: ["transparent", "hackable"],
        sentence: (
            <>
                One obvious way to build, change, and monitor software, full stack: Git, SQLite,
                TypeScript, and OTel.
            </>
        ),
        gloss: "as open as VS Code, as transparent as SQLite, as hackable as Minecraft",
    },
};

/** The apps the stacked agent opens in the browser, by the server its connector calls. */
const consoleNames: ReadonlySet<string> = new Set<Console>([
    "zapier",
    "airtable",
    "trello",
    "slack",
    "googlecalendar",
]);

/** Show the adjective at work: one run in rented consoles or your space, your agent beside. */
export function Adjective(properties: { isOpen: boolean }) {
    // hold the section, the motion it may show, and whether it is on screen
    const [section, setSection] = createSignal<HTMLElement>();
    const isReduced = createReducedMotion(section);
    const isShown = createVisibilityObserver(section, { initialValue: true });

    // count beats until the reader takes the playhead, and start again on every switch
    const [beat, setBeat] = createSignal(0);
    const [held, setHeld] = createSignal<number | undefined>(undefined);
    createTimer(
        () => setBeat((current) => current + 1),
        () => (!isReduced() && isShown() && held() === undefined ? BEAT : false),
        setInterval,
    );
    createEffect(
        () => properties.isOpen,
        () => {
            setBeat(0);
            setHeld(undefined);
        },
    );

    // run the conversation in the state the stack is in, finished when still
    const scene = () => (properties.isOpen ? destackedScene : stackedScene);
    const at = () => (isReduced() ? undefined : beat() % scene().length);
    const moment = () => (properties.isOpen ? destackedChatAt(at()) : stackedScene.at(at()));

    // take the assistant the reader chose, or else the next one's look each time it runs again
    const { look, progress, choose } = createAssistantTurn(beat, () => scene().length, isReduced);

    // show the run where the reader holds it, or where the conversation has it
    const run = () => runMomentAt(at() ?? destackedScene.length, held());

    // open the app the stacked agent last called, the launch channel before it calls any
    const opened = (): Console => {
        const calls = moment().items.filter((item) => item.kind === "call");
        const last = calls.at(-1);

        return last?.kind === "call" && isConsole(last.server) ? last.server : "slack";
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
            <Entry form={destackable} />
            <FigureLabel
                number={5}
                isOpen={properties.isOpen}
                stacked="five tools, five black boxes"
                destacked="one app, one glass box"
            >
                <StackSwitch isOpen={properties.isOpen} place="label" />
            </FigureLabel>
            <div {...style.attrs(lattice.cell, lattice.figureCell, styles.app)}>
                <Label>
                    {properties.isOpen ? "Your app, in the editor" : "Your app, in five tools"}
                </Label>
                <Inspector
                    isOpen={properties.isOpen}
                    console={opened()}
                    moment={run()}
                    onScrub={(share) => setHeld(share * RUN_LENGTH)}
                    onTouch={() => setHeld((current) => current ?? run().time)}
                />
            </div>
            <div {...style.attrs(lattice.cell, lattice.figureCell, styles.console)}>
                <LabelRow
                    end={
                        <Picker
                            label="Agent"
                            options={assistantOptions}
                            shown={look()}
                            progress={progress()}
                            onChoose={choose}
                        />
                    }
                >
                    <Label>
                        {properties.isOpen
                            ? "Your agent, in your space"
                            : "Your agent, one connector per tool"}
                    </Label>
                </LabelRow>
                <AgentChat
                    isOpen={properties.isOpen}
                    moment={moment()}
                    assistant={look()}
                    connectors={stackedConnectors}
                />
            </div>
        </section>
    );
}

/** Report whether a connector's server opens one of the stacked apps. */
function isConsole(server: string): server is Console {
    return consoleNames.has(server);
}

/** The adjective section styles. */
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
        minHeight: { default: 0, [screen.belowDesktop]: "44rem" },
    },
    console: {
        gridColumn: { default: "8 / -1", [screen.belowDesktop]: "1 / -1" },
        gridTemplateRows: "auto minmax(0, 1fr)",
        minHeight: { default: 0, [screen.belowDesktop]: "22rem" },
    },
});
