import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { present } from "@destack/schema";
import { createMemo, createSignal } from "@destack/view";
import { createVisibilityObserver } from "@destack/view/primitives/intersection-observer";
import { createTimer } from "@destack/view/primitives/timer";

import { lattice } from "../../layout/lattice.stylex";
import { Gearset } from "../drawing/gearset";
import { Entry, FigureLabel, type Form } from "../entry/entry";
import { LabelRow, pane, PaneLabel } from "../entry/label";
import { audienceOf, audiences, Dial, teams } from "../figure/dial";
import { BillTotal, totalOf } from "../figure/bill";
import { type Maker, makerOf, makers, makerSections } from "./maker";
import { XRay } from "./xray";
import { AgentChat, createBeat } from "../command/chat";
import { destackedScene, stackedConnectors, stackedSceneOf } from "./scene";
import {
    entriesWith,
    rentedWith,
    serviceCount,
    serviceIndexOf,
    type ServiceLineup,
    vendorsOf,
} from "./service";
import { Picker } from "../figure/picker";
import { createSteps } from "../figure/stagger";
import { BASE_USERS, workloadOf } from "../figure/price";
import { StackSwitch } from "../entry/switch";
import { createReducedMotion } from "@destack/view/motion";

/** The panes the figure switches between below the desktop width, its app and your agent. */
const PANES = ["Your app", "Your agent"];

/** The milliseconds a row holds its service before the next row turns to its next one. */
const turnTime = 2500;

/** The noun. */
export const noun: Form = {
    syllables: ["De", "stack"],
    pronunciation: "dee-stack",
    partOfSpeech: "noun",
    sense: {
        definition: "a universal software engine",
        highlight: ["software engine"],
        sentence: "Make serious custom apps on one standardised pipeline.",
    },
};

/** Show the noun at work: your launch tracker on nine rented services, or on one engine. */
export function Noun(properties: { isOpen: boolean }) {
    // follow the motion the section shows
    const [section, setSection] = createSignal<HTMLElement>();
    const isReduced = createReducedMotion(section);

    // switch each service and its ledger row in order, after the space itself
    const steps = createSteps(() => properties.isOpen, serviceCount + 1, isReduced);
    const isOpenAt = steps.isOpenAt;

    // read the bill for a team and an audience of the sizes the reader dials
    const [people, setPeople] = createSignal<number>(1);
    const [users, setUsers] = createSignal<number>(BASE_USERS);

    // turn one row at a time to its next service while the section shows, until the reader touches it
    const isShown = createVisibilityObserver(section, { initialValue: true });
    const [isTouched, setIsTouched] = createSignal(false);
    const [turns, setTurns] = createSignal(0);
    createTimer(
        () => setTurns((current) => current + 1),
        () => (!isReduced() && isShown() && !isTouched() ? turnTime : false),
        "interval",
    );

    // hold the agent the app was made with, Amp until the reader picks another
    const [maker, setMaker] = createSignal<Maker>(makerOf("Amp"));
    const chooseMaker = (name: string) => {
        setIsTouched(true);
        setMaker(makerOf(name));
    };

    // run your agent's chat beside the app while it shows
    const beat = createBeat(
        () => !isReduced() && isShown(),
        () => [properties.isOpen, maker()],
    );
    const scene = createMemo(() => (properties.isOpen ? destackedScene : stackedSceneOf(maker())));
    const moment = () => scene().at(isReduced() ? undefined : beat() % scene().length);

    // rent the service each category's row has turned to
    const lineup = (): ServiceLineup => (category) => {
        // count how often the category's row has turned
        const index = serviceIndexOf(category);
        const advances = turns() > index ? Math.floor((turns() - 1 - index) / serviceCount) + 1 : 0;
        const options = vendorsOf(category);

        return present(options[advances % options.length], "turned service");
    };
    const rented = () => rentedWith(lineup());

    // light the ledger row pointed at, counting the space as row 0, or else the row that just turned
    const [pointed, setLit] = createSignal<number | undefined>(undefined);
    const lit = () =>
        pointed() ??
        (!isReduced() && !isTouched() && turns() > 0
            ? ((turns() - 1) % serviceCount) + 1
            : undefined);

    // show the app or your agent below the desktop width, where one pane shows at a time
    const [shownPane, setShownPane] = createSignal(0);

    return (
        <section
            ref={setSection}
            {...style.attrs(lattice.frame, lattice.ruled, lattice.section, styles.section)}
        >
            <Entry form={noun} isTitle />
            <FigureLabel
                number={2}
                isOpen={properties.isOpen}
                stacked="one app, nine services, software pricing"
                destacked="one app, one engine, hardware pricing"
            >
                <StackSwitch isOpen={properties.isOpen} place="label" />
            </FigureLabel>
            <Gearset isOpen={properties.isOpen} />
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
                                    entriesWith(lineup()).map((entry) => ({
                                        entry,
                                        workload: workloadOf(entry.stacked.name),
                                    })),
                                    { people: people(), users: users() },
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
                            <Dial
                                label="Users"
                                options={audiences}
                                value={users()}
                                format={audienceOf}
                                onChange={(next) => {
                                    setIsTouched(true);
                                    setUsers(next);
                                }}
                            />
                        </span>
                    }
                >
                    <PaneLabel panes={PANES} shown={shownPane()} onShow={setShownPane}>
                        {isOpenAt(0) ? "Your app, in production" : "Your app, in limbo"}
                    </PaneLabel>
                </LabelRow>
                <div {...style.attrs(styles.window)}>
                    <XRay
                        rented={rented()}
                        maker={maker()}
                        isOpenAt={isOpenAt}
                        lit={lit()}
                        onLight={setLit}
                    />
                </div>
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
                            label="Made with"
                            options={makers}
                            sections={makerSections}
                            shown={maker().name}
                            progress={undefined}
                            onChoose={chooseMaker}
                        />
                    }
                >
                    <PaneLabel panes={PANES} shown={shownPane()} onShow={setShownPane}>
                        {isOpenAt(0) ? "Your agent, in your space" : "Your agent, beside the app"}
                    </PaneLabel>
                </LabelRow>
                <AgentChat
                    title="Launch tracker"
                    isOpen={properties.isOpen}
                    moment={moment()}
                    assistant={maker().agent}
                    connectors={stackedConnectors}
                />
            </div>
        </section>
    );
}

/** The noun section styles. */
const styles = style.create({
    section: {
        gridTemplateRows: { default: "auto auto minmax(0, 1fr)", [media.maxLg]: "auto" },
    },
    cost: {
        alignItems: "baseline",
        display: "inline-flex",
        flexWrap: "wrap",
        gap: "0.75rem",
    },
    stage: {
        containerType: "inline-size",
        gridColumn: { default: "1 / 8", [media.maxLg]: "1 / -1" },
        gridTemplateRows: "auto minmax(0, 1fr)",
    },
    key: {
        gridColumn: { default: "8 / -1", [media.maxLg]: "1 / -1" },
        gridTemplateRows: {
            default: "auto minmax(0, 1fr)",
            [media.maxLg]: "auto 34rem",
            [media.maxMd]: "auto 36rem",
        },
    },
    window: {
        minHeight: 0,
        height: { default: "100%", [media.maxLg]: "34rem", [media.maxMd]: "36rem" },
    },
});
