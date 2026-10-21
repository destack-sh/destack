import { present } from "@destack/schema";
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
import { StackSwitch } from "../entry/switch";
import { pages, rowOf, SpaceBrowser, type Viewer } from "../verb/space";
import { AgentChat, assistantOptions, createAssistantTurn } from "./chat";
import { BEAT, destacked, stacked, stackedConnectors } from "./scene";
import { sources } from "../verb/source";
import { AirtableSignups } from "../tool/airtable";
import { type Lineup, lineupAt, toolOf } from "../tool/tool";

/** The rows of fig. 1 the task touches: the waitlist, its table, the chat and the workflows. */
const ROWS: readonly number[] = (["tables", "builder", "chat", "workflows"] as const).map(rowOf);

/** The tools the stacked task connects to, each row's first, Airtable open on the waitlist. */
const lineup: Lineup = (category) =>
    category === "tables"
        ? {
              ...toolOf("Airtable"),
              tab: "Launch: Signups - Airtable",
              site: "airtable.com/appLaunch/tblSignups",
              page: AirtableSignups,
          }
        : lineupAt(0)(category);

/** The command every app answers to, as an imperative. */
const command: Form = {
    syllables: ["de", "stack!"],
    pronunciation: "/diːˈstak/",
    partOfSpeech: "imperative",
    sense: {
        definition: "every app, programmable and composable",
        highlight: ["programmable", "composable"],
        sentence: (
            <>
                One API for every app in <em>your</em> space — for you, for your agents, on your
                terms.
            </>
        ),
        gloss: "program every app like a spreadsheet",
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

    // run the task in the state the stack is in, then hold and run again, finished when still
    const scene = () => (properties.isOpen ? destacked : stacked);
    const moment = () => scene().at(isReduced() ? undefined : beat() % scene().length);
    const step = () => moment().step;

    // take the assistant the reader chose, or else the next one's look each time it runs again
    const { look, progress, choose } = createAssistantTurn(beat, () => scene().length, isReduced);

    // show the page the reader opens from a tab, or else the page the step or the walk touches
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
        () => Math.floor(beat() / scene().length),
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
                number={4}
                isOpen={properties.isOpen}
                stacked="two apps, two connectors"
                destacked="two apps, one space"
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
                    change={step().change}
                    source={present(sources[0], "first source")}
                    lineup={lineup}
                    vendor={undefined}
                    remix={undefined}
                    isOpenAt={() => properties.isOpen}
                    page={page()}
                    viewer={viewer()}
                    onViewer={setViewer}
                    onChoose={setOpened}
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
                            ? "Your agent, one connection"
                            : "Your agent, one connector per app"}
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
