import { present } from "@destack/schema";
import * as style from "@destack/style";
import { color } from "@destack/theme/tokens.stylex";
import { createSignal, useEnvironment } from "@destack/view";
import { createTimer, Timeout } from "@destack/view/primitives/timer";
import { createReducedMotion } from "@destack/view/motion";
import { createVisibilityObserver } from "@destack/view/primitives/intersection-observer";

import { lattice } from "../../layout/lattice.stylex";
import { Entry, FigureLabel, type Form } from "../entry/entry";
import { Label, LabelRow, LabelTabs } from "../entry/label";
import { AgentChat, assistantOptions, createAssistantTurn, createBeat } from "../command/chat";
import { Picker } from "../figure/picker";
import { destackedScene, stackedConnectors, stackedScene } from "./scene";
import { Dial, teams } from "../figure/dial";
import { Bill } from "../figure/bill";
import { accentOf, type Item, marksOf } from "../figure/item";
import {
    everyRow,
    categoryAt,
    DATA_ROW,
    holdingsWith,
    rowOf,
    pages,
    SpaceBrowser,
    type Viewer,
} from "./space";
import type { Stagger } from "../figure/stagger";
import { BASE_USERS, jobWorkloads, type Workload, workloadOf } from "../figure/price";
import { StackSwitch } from "../entry/switch";
import { sources } from "./source";
import { type Lineup, lineupAt, type Tool, toolsOf } from "../tool/tool";
import type { Option } from "../figure/picker";
import { screen } from "../../layout/screen.stylex";

/** The milliseconds the pointer rests on a holding before the browser turns to its page. */
const settleIn = 80;

/** The milliseconds after the pointer leaves before the browser turns back. */
const settleOut = 300;

/** The milliseconds each page of your corner holds before the browser turns to the next. */
const pageTime = 5200;

/** The milliseconds your agent takes to make a change you asked for. */
const remixTime = 1400;

/** The milliseconds an owned Tasks page shows before your agent remixes it once on its own. */
const firstRemixAt = 1800;

/** The row of Tasks, which your agent remixes once on its own. */
const TASKS_ROW = rowOf("tasks");

/** The verb, which titles the page. */
const verb: Form = {
    syllables: ["de", "stack"],
    pronunciation: "/diːˈstak/",
    partOfSpeech: "verb",
    sense: {
        definition: "to reconquer your software",
        highlight: ["reconquer"],
        sentence:
            "Own the apps you use, the apps you make, and the agents you run, in one open stack.",
        gloss: "to standardise how software is made, shipped, and changed",
    },
};

/** Return the apps a row offers: its category's tools, or the personal apps on the data row. */
function optionsOf(row: number): readonly Option[] {
    const category = categoryAt(row);
    const offered = category === undefined ? sources : toolsOf(category);

    return offered.map((app) => ({ name: app.name, mark: { icon: app.icon, tint: app.tint } }));
}

/** Return the work a row's tool takes: its job's, or the personal app's on the data row. */
function workloadAt(row: number, name: string): Workload {
    const category = categoryAt(row);

    return category === undefined ? workloadOf(name) : jobWorkloads[category];
}

/** Return the tint of an item's first mark, or the muted tone where it has none. */
function tintOf(item: Item): string {
    const [mark] = marksOf(item.mark);

    return mark !== undefined && "tint" in mark ? mark.tint : color.mutedForeground;
}

/** Open the page with the verb, illustrated by your space in the cloud. */
export function Verb(properties: {
    isOpen: boolean;
    picks: readonly Tool[];
    onChoose: (tool: Tool) => void;
}) {
    // hold the page's clock, the holding pointed at, the one the browser settles on a moment later, and the one chosen
    const { clock } = useEnvironment();
    const [lit, setLit] = createSignal<number | undefined>(undefined);
    const [settled, setSettled] = createSignal<number | undefined>(undefined);
    const [chosen, setChosen] = createSignal<number | undefined>(undefined);

    // hold the figure still once the reader has touched any of it
    const [isTouched, setIsTouched] = createSignal(false);
    const choose = (row: number | undefined) => {
        setIsTouched(true);
        setChosen(row);
    };
    const settling = new Timeout();
    const lighting = {
        get lit() {
            return lit() ?? chosen();
        },
        onChoose: choose,
        get chosen() {
            return chosen();
        },
        onLight: (row: number | undefined) => {
            // light the row at once, and turn the browser only once the pointer rests
            setLit(row);
            settling.set(() => setSettled(row), row === undefined ? settleOut : settleIn);
        },
    };

    // read the bill for a team of the size the reader dials
    const [people, setPeople] = createSignal<number>(5);

    // follow the motion the section shows, and whether it is on screen
    const [section, setSection] = createSignal<HTMLElement>();
    const isReduced = createReducedMotion(section);
    const isShown = createVisibilityObserver(section, { initialValue: true });

    // switch the browser and every holding at once, so the switch morphs as one
    const isOpenAt: Stagger = () => properties.isOpen;

    // show the bill or your agent's chat beside the space, the chat running while it shows
    const [tab, setTab] = createSignal(0);
    const beat = createBeat(
        () => tab() === 1 && !isReduced() && isShown(),
        () => properties.isOpen,
    );
    const scene = () => (properties.isOpen ? destackedScene : stackedScene);
    const moment = () => scene().at(isReduced() ? undefined : beat() % scene().length);
    const {
        look,
        progress,
        choose: chooseAssistant,
    } = createAssistantTurn(beat, () => scene().length, isReduced);

    // hold who is looking, whether the reader chose them, and the page the window turns to on its own
    const [viewer, setViewer] = createSignal<Viewer>("Me");
    const [isViewerChosen, setIsViewerChosen] = createSignal(false);
    const [turned, setTurned] = createSignal(0);
    const chooseViewer = (next: Viewer) => {
        setIsTouched(true);
        setIsViewerChosen(true);
        setViewer(next);
    };

    // ask your agent to change an app, which it does a moment later
    const [remixed, setRemixed] = createSignal<readonly number[]>([]);
    const [asking, setAsking] = createSignal<number | undefined>(undefined);
    const remix = (row: number) => {
        setAsking(row);
        clock.setTimeout(() => {
            setRemixed((rows) => (rows.includes(row) ? rows : [...rows, row]));
            setAsking(undefined);
        }, remixTime);
    };

    // turn each row to its next app each time the browser leaves its page, one row at a time
    const turnsOf = (row: number) =>
        turned() >= row ? Math.floor((turned() - row) / pages.length) + 1 : 0;

    // show the personal app the reader chose, or else the one the data row has turned to
    const [chosenSource, setChosenSource] = createSignal<string | undefined>(undefined);
    const source = () =>
        present(
            sources.find((candidate) => candidate.name === chosenSource()) ??
                sources[turnsOf(DATA_ROW) % sources.length],
            "shown source",
        );

    // show the tool the reader picked last for a row, or else the one the row has turned to
    const lineup = (): Lineup => (category) => {
        const picked = toolsOf(category).filter((tool) => properties.picks.includes(tool));
        const latest = picked.toSorted(
            (left, right) => properties.picks.indexOf(left) - properties.picks.indexOf(right),
        )[picked.length - 1];

        return latest ?? lineupAt(turnsOf(rowOf(category)))(category);
    };

    // show a chosen app's page: a tool every figure shares, or a personal app of this one
    const pick = (row: number, name: string) => {
        const category = categoryAt(row);
        if (category === undefined) {
            setChosenSource(name);
        } else {
            properties.onChoose(
                present(
                    toolsOf(category).find((tool) => tool.name === name),
                    `${name} tool`,
                ),
            );
        }
        choose(row);
    };

    // show the page of the pointed or chosen holding, or else the page the window has turned to
    const page = () =>
        present(
            pages.find((candidate) => candidate.row === (settled() ?? chosen())) ??
                pages[turned() % pages.length],
            "shown page",
        );

    // remix Tasks once on its own the first time its owned page shows
    createTimer(
        () => remix(TASKS_ROW),
        () =>
            isOpenAt(1) &&
            !isTouched() &&
            page().row === TASKS_ROW &&
            remixed().length === 0 &&
            asking() === undefined &&
            !isReduced()
                ? firstRemixAt
                : false,
        "timeout",
    );

    // turn to the next page the viewer may see every page time, held while still or off screen
    createTimer(
        () => {
            // look as your agent every other round once owned, until the reader picks a viewer
            const nextRound = Math.floor((turned() + 1) / pages.length);
            if (isOpenAt(1) && !isViewerChosen()) {
                setViewer(nextRound % 2 === 1 ? "Agent" : "Me");
            }

            // skip the pages the viewer may not see once owned
            let next = turned() + 1;
            while (
                isOpenAt(1) &&
                pages[next % pages.length]?.viewers.includes(viewer()) !== true &&
                next < turned() + pages.length
            ) {
                next += 1;
            }
            setTurned(next);
        },
        () => (isReduced() || isTouched() || !isShown() ? false : pageTime),
        "interval",
    );

    return (
        <section
            ref={setSection}
            {...style.attrs(lattice.frame, lattice.ruled, lattice.section, styles.section)}
        >
            <Entry form={verb} />
            <FigureLabel
                number={2}
                isOpen={properties.isOpen}
                stacked="twelve apps, twelve silos"
                destacked="twelve apps, one space"
            >
                <StackSwitch isOpen={properties.isOpen} place="label" />
            </FigureLabel>
            <div {...style.attrs(lattice.cell, lattice.figureCell, styles.stage)}>
                <Label>
                    {isOpenAt(0) ? "Your work, in your space" : "Your work, on their servers"}
                </Label>
                <SpaceBrowser
                    name="space"
                    rows={everyRow}
                    change="start"
                    isOpenAt={isOpenAt}
                    page={page()}
                    viewer={viewer()}
                    onViewer={chooseViewer}
                    onChoose={choose}
                    source={source()}
                    lineup={lineup()}
                    vendor={undefined}
                    remix={{
                        remixed: remixed(),
                        asking: asking(),
                        onRemix: (row) => {
                            setIsTouched(true);
                            remix(row);
                        },
                    }}
                />
            </div>
            <div {...style.attrs(lattice.cell, lattice.figureCell, styles.key)}>
                <LabelRow
                    end={
                        tab() === 0 ? (
                            <Dial
                                label="People"
                                options={teams}
                                value={people()}
                                format={String}
                                onChange={(next) => {
                                    setIsTouched(true);
                                    setPeople(next);
                                }}
                            />
                        ) : (
                            <Picker
                                label="Agent"
                                options={assistantOptions}
                                sections={undefined}
                                shown={look()}
                                progress={progress()}
                                onChoose={chooseAssistant}
                            />
                        )
                    }
                >
                    <LabelTabs
                        tabs={[isOpenAt(0) ? "Paid by usage" : "Paid per seat", "Your agent"]}
                        shown={tab()}
                        onShow={setTab}
                    />
                </LabelRow>
                {tab() === 1 ? (
                    <AgentChat
                        isOpen={properties.isOpen}
                        moment={moment()}
                        look={{ kind: "assistant", assistant: look() }}
                        connectors={stackedConnectors}
                    />
                ) : (
                    <Bill
                        isWindow
                        scale={{ people: people(), users: BASE_USERS }}
                        scales={teams.map((team) => ({ people: team, users: BASE_USERS }))}
                        lines={holdingsWith(source(), lineup()).map((entry, index) => ({
                            entry,
                            workload: workloadAt(index + 1, entry.stacked.name),
                            tint: accentOf(entry.stacked.name, tintOf(entry.stacked)),
                        }))}
                        lighting={lighting}
                        isOpenAt={isOpenAt}
                        optionsOf={optionsOf}
                        onChoose={pick}
                    />
                )}
            </div>
        </section>
    );
}

/** The verb section styles. */
const styles = style.create({
    section: {
        gridTemplateRows: { default: "auto auto minmax(0, 1fr)", [screen.belowDesktop]: "auto" },
    },
    stage: {
        containerType: "inline-size",
        gridColumn: { default: "1 / 8", [screen.belowDesktop]: "1 / -1" },
        gridTemplateRows: { default: "auto minmax(0, 1fr)", [screen.belowDesktop]: "auto 30rem" },
    },
    key: {
        gridColumn: { default: "8 / -1", [screen.belowDesktop]: "1 / -1" },
        position: "relative",
        gridTemplateRows: "auto minmax(0, 1fr)",
    },
});
