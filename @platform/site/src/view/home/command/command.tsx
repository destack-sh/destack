import { present } from "@destack/schema";
import * as style from "@destack/style";
import { createEffect, createSignal } from "@destack/view";
import { createReducedMotion } from "@destack/view/motion";
import { createVisibilityObserver } from "@destack/view/primitives/intersection-observer";

import { lattice } from "../../layout/lattice.stylex";
import { screen } from "../../layout/screen.stylex";
import { Entry, FigureLabel, type Form } from "../entry/entry";
import { Label, LabelRow } from "../entry/label";
import { Picker } from "../figure/picker";
import { StackSwitch } from "../entry/switch";
import { pages, rowOf, SpaceBrowser, type Viewer } from "../verb/space";
import { AgentChat, assistantOptions, createAssistantTurn, createBeat } from "./chat";
import { artifactOf } from "./artifact";
import { destacked, stacked, stackedConnectors } from "./scene";
import { sources } from "../verb/source";
import { AirtableSignups } from "../tool/airtable";
import { type Lineup, lineupAt, toolOf } from "../tool/tool";

/** The rows of fig. 1 the task touches: the waitlist, its table, the chat and the workflows. */
const ROWS: readonly number[] = (["tables", "builder", "chat", "workflows"] as const).map(rowOf);

/** The row of the app you made, where the assistant's artifact opens at its public link. */
const BUILDER_ROW = rowOf("builder");

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
        definition: "every app, programmable by your agents",
        highlight: ["programmable"],
        sentence: "Put your agents to work across everything in your space, with one API.",
        gloss: "as programmable as a humble spreadsheet",
    },
};

/** Show the imperative at work: an app beside a chat with your agent, a turn per app. */
export function Command(properties: { isOpen: boolean }) {
    // hold the section, the motion it may show, and whether it is on screen
    const [section, setSection] = createSignal<HTMLElement>();
    const isReduced = createReducedMotion(section);
    const isShown = createVisibilityObserver(section, { initialValue: true });

    // count beats, held while still or off screen, and start the task again on every switch
    const beat = createBeat(
        () => !isReduced() && isShown(),
        () => properties.isOpen,
    );

    // run the task in the state the stack is in, then hold and run again, finished when still
    const scene = () => (properties.isOpen ? destacked : stacked);
    const moment = () => scene().at(isReduced() ? undefined : beat() % scene().length);
    const step = () => moment().step;

    // take the assistant the reader chose, or else the next one's look each time it runs again
    const { look, progress, choose } = createAssistantTurn(beat, () => scene().length, isReduced);

    // open the app the assistant made at its public link once it exists, while stacked
    const artifact = () => {
        const app = moment().items.find((item) => item.kind === "app");

        return !properties.isOpen && app?.kind === "app"
            ? artifactOf(look(), BUILDER_ROW, app.title, app.rows)
            : undefined;
    };

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
            {...style.attrs(lattice.frame, lattice.ruled, lattice.section, styles.section)}
        >
            <Entry form={command} />
            <FigureLabel
                number={5}
                isOpen={properties.isOpen}
                stacked="two apps, two connectors"
                destacked="two apps, one API"
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
                    vendor={artifact()}
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
                            sections={undefined}
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
                    look={{ kind: "assistant", assistant: look() }}
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
