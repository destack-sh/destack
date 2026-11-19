import { media } from "@destack/style/media.stylex";
import { present } from "@destack/schema";
import * as style from "@destack/style";
import { createSignal, useEnvironment } from "@destack/view";
import { createTimer } from "@destack/view/primitives/timer";
import { createReducedMotion } from "@destack/view/motion";
import { createVisibilityObserver } from "@destack/view/primitives/intersection-observer";

import { lattice } from "../../layout/lattice.stylex";
import { Entry, FigureLabel, type Form } from "../entry/entry";
import { LabelRow, pane, PaneLabel } from "../entry/label";
import { AgentChat, assistantOptions, createAssistantTurn, createBeat } from "../command/chat";
import { Picker } from "../figure/picker";
import { destackedScene, stackedConnectors, stackedScene } from "./scene";
import { Dial, teams } from "../figure/dial";
import { BillTotal, totalOf } from "../figure/bill";
import { PlanetDrawing } from "../drawing/planet";
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
import { tagline } from "../../content/site";

/** The panes the figure switches between below the desktop width, its app and your agent. */
const PANES = ["Your apps", "Your agent"];

/** The milliseconds each page of your corner holds before the browser turns to the next. */
const pageTime = 5200;

/** The milliseconds your agent takes to make a change you asked for. */
const remixTime = 1400;

/** The milliseconds an owned Tasks page shows before your agent remixes it once on its own. */
const firstRemixAt = 1800;

/** The row of Tasks, which your agent remixes once on its own. */
const TASKS_ROW = rowOf("tasks");

/** The row of the press list your agent makes, which the browser turns to once it is done. */
const TABLES_ROW = rowOf("tables");

/** The verb, which titles the page. */
export const verb: Form = {
    syllables: ["de", "stack"],
    pronunciation: "dee-stack",
    partOfSpeech: "verb",
    sense: {
        definition: "to own your stack",
        highlight: ["own"],
        sentence: tagline,
    },
};

/** Return the work a row's tool takes: its job's, or the personal app's on the data row. */
function workloadAt(row: number, name: string): Workload {
    const category = categoryAt(row);

    return category === undefined ? workloadOf(name) : jobWorkloads[category];
}

/** Open the page with the verb, illustrated by your space in the cloud. */
export function Verb(properties: { isOpen: boolean; picks: readonly Tool[] }) {
    // hold the page's clock and the page the reader chose
    const { clock } = useEnvironment();
    const [chosen, setChosen] = createSignal<number | undefined>(undefined);

    // hold the figure still once the reader has touched any of it
    const [isTouched, setIsTouched] = createSignal(false);
    const choose = (row: number | undefined) => {
        setIsTouched(true);
        setChosen(row);
    };

    // read the bill for a team of the size the reader dials
    const [people, setPeople] = createSignal<number>(5);

    // follow the motion the section shows, and whether it is on screen
    const [section, setSection] = createSignal<HTMLElement>();
    const isReduced = createReducedMotion(section);
    const isShown = createVisibilityObserver(section, { initialValue: true });

    // switch the browser and every holding at once
    const isOpenAt: Stagger = () => properties.isOpen;

    // run your agent's chat beside the space while it shows
    const beat = createBeat(
        () => !isReduced() && isShown(),
        () => properties.isOpen,
    );
    const scene = () => (properties.isOpen ? destackedScene : stackedScene);
    const moment = () => scene().at(isReduced() ? undefined : beat() % scene().length);
    const {
        look,
        progress,
        choose: chooseAssistant,
    } = createAssistantTurn("Claude", beat, () => scene().length, isReduced);

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

    // show the personal app the data row has turned to
    const source = () => present(sources[turnsOf(DATA_ROW) % sources.length], "shown source");

    // show the tool the reader picked last for a row, or else the one the row has turned to
    const lineup = (): Lineup => (category) => {
        const picked = toolsOf(category).filter((tool) => properties.picks.includes(tool));
        const latest = picked.toSorted(
            (left, right) => properties.picks.indexOf(left) - properties.picks.indexOf(right),
        )[picked.length - 1];

        return latest ?? lineupAt(turnsOf(rowOf(category)))(category);
    };

    // show the page the reader chose, the press list once your agent says it is done, or the turned page
    const isDone = () => moment().items.some((item) => item.kind === "reply");
    const shownRow = () => chosen() ?? (isDone() ? TABLES_ROW : undefined);
    const page = () =>
        present(
            pages.find((candidate) => candidate.row === shownRow()) ??
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

    // show the app or your agent below the desktop width, where one pane shows at a time
    const [shownPane, setShownPane] = createSignal(0);

    return (
        <section
            ref={setSection}
            {...style.attrs(lattice.frame, lattice.ruled, lattice.section, styles.section)}
        >
            <Entry form={verb} />
            <FigureLabel
                number={1}
                isOpen={properties.isOpen}
                stacked="twelve apps, twelve silos, rigid and closed"
                destacked="twelve apps, one space, malleable and open"
            >
                <StackSwitch isOpen={properties.isOpen} place="label" isHinted />
            </FigureLabel>
            <PlanetDrawing isOpen={properties.isOpen} isMoving={!isReduced() && isShown()} />
            <div
                {...style.attrs(
                    lattice.cell,
                    lattice.figureCell,
                    styles.stage,
                    shownPane() !== 0 && pane.hidden,
                )}
            >
                <LabelRow
                    end={
                        <span {...style.attrs(styles.cost)}>
                            <BillTotal
                                amount={totalOf(
                                    holdingsWith(source(), lineup()).map((entry, index) => ({
                                        entry,
                                        workload: workloadAt(index + 1, entry.stacked.name),
                                    })),
                                    { people: people(), users: BASE_USERS },
                                    isOpenAt,
                                )}
                            />
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
                        </span>
                    }
                >
                    <PaneLabel panes={PANES} shown={shownPane()} onShow={setShownPane}>
                        {isOpenAt(0) ? "Your work, in your space" : "Your work, on their servers"}
                    </PaneLabel>
                </LabelRow>
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
            <div
                {...style.attrs(
                    lattice.cell,
                    lattice.figureCell,
                    styles.key,
                    shownPane() !== 1 && pane.hidden,
                )}
            >
                <LabelRow
                    end={
                        <Picker
                            label="Agent"
                            options={assistantOptions}
                            sections={undefined}
                            shown={look()}
                            progress={progress()}
                            onChoose={chooseAssistant}
                        />
                    }
                >
                    <PaneLabel panes={PANES} shown={shownPane()} onShow={setShownPane}>
                        {isOpenAt(0)
                            ? "Your agent, in your space"
                            : "Your agent, with a connector per tool"}
                    </PaneLabel>
                </LabelRow>
                <AgentChat
                    title="Press outreach"
                    isOpen={properties.isOpen}
                    moment={moment()}
                    assistant={look()}
                    connectors={stackedConnectors}
                />
            </div>
        </section>
    );
}

/** The verb section styles. */
const styles = style.create({
    section: {
        gridTemplateRows: { default: "auto auto minmax(0, 1fr)", [media.maxLg]: "auto" },
    },
    stage: {
        containerType: "inline-size",
        gridColumn: { default: "1 / 8", [media.maxLg]: "1 / -1" },
        gridTemplateRows: { default: "auto minmax(0, 1fr)", [media.maxLg]: "auto 30rem" },
    },
    cost: {
        alignItems: "baseline",
        display: "inline-flex",
        gap: "0.75rem",
    },
    key: {
        gridColumn: { default: "8 / -1", [media.maxLg]: "1 / -1" },
        position: "relative",
        gridTemplateRows: { default: "auto minmax(0, 1fr)", [media.maxLg]: "auto 30rem" },
    },
});
