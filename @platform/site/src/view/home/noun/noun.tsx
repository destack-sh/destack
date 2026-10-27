import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color } from "@destack/theme/tokens.stylex";
import { present } from "@destack/schema";
import { createMemo, createSignal } from "@destack/view";
import { createVisibilityObserver } from "@destack/view/primitives/intersection-observer";
import { createTimer } from "@destack/view/primitives/timer";

import { lattice } from "../../layout/lattice.stylex";
import { Entry, FigureLabel, type Form } from "../entry/entry";
import { Label, LabelRow, LabelTabs } from "../entry/label";
import { audienceOf, audiences, Dial } from "../figure/dial";
import { Bill } from "../figure/bill";
import { accentOf } from "../figure/item";
import { lookOf, type Maker, makerOf, makers, makerSections } from "./maker";
import { XRay } from "./xray";
import { AgentChat, createBeat } from "../command/chat";
import { destackedScene, stackedSceneOf } from "./scene";
import {
    categoryOf,
    entriesWith,
    rentedWith,
    serviceCount,
    type ServiceCategory,
    serviceIndexOf,
    type ServiceLineup,
    vendorOf,
    vendorsOf,
} from "./service";
import { type Option, Picker } from "../figure/picker";
import { createSteps } from "../figure/stagger";
import { BASE_USERS, workloadOf } from "../figure/price";
import { StackSwitch } from "../entry/switch";
import { screen } from "../../layout/screen.stylex";
import { createReducedMotion } from "@destack/view/motion";

/** The milliseconds between the steps of the figure's switch, one per service. */
const stepTime = 40;

/** The milliseconds a row holds its service before the next row turns to its next one. */
const turnTime = 2500;

/** The noun. */
const noun: Form = {
    syllables: ["De", "stack"],
    pronunciation: "/ˈdiːstak/",
    partOfSpeech: "noun",
    sense: {
        definition: "an absurdly integrated universal software engine",
        highlight: ["universal software engine"],
        sentence: (
            <>
                Make high-quality custom software easily, with one standardised pipeline that{" "}
                <em>your</em> agents love.
            </>
        ),
        gloss: "Unreal Engine, for everyday software",
    },
};

/** Show the noun at work: your launch tracker on nine rented services, or on one engine. */
export function Noun(properties: { isOpen: boolean }) {
    // follow the motion the section shows
    const [section, setSection] = createSignal<HTMLElement>();
    const isReduced = createReducedMotion(section);

    // switch each service and its ledger row in order, after the space itself
    const steps = createSteps(() => properties.isOpen, serviceCount + 1, stepTime, isReduced);
    const isOpenAt = steps.isOpenAt;

    // read the bill for an app with as many users as the reader dials
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

    // hold the agent the app was made with, Claude until the reader picks another
    const [maker, setMaker] = createSignal<Maker>(makerOf("Claude"));
    const chooseMaker = (name: string) => {
        setIsTouched(true);
        setMaker(makerOf(name));
    };

    // show the vendors or your agent's chat beside the app, the chat running while it shows
    const [tab, setTab] = createSignal(0);
    const beat = createBeat(
        () => tab() === 1 && !isReduced() && isShown(),
        () => [properties.isOpen, maker()],
    );
    const scene = createMemo(() => (properties.isOpen ? destackedScene : stackedSceneOf(maker())));
    const moment = () => scene().at(isReduced() ? undefined : beat() % scene().length);

    // rent each category's chosen service, or the one its row has turned to
    const [picked, setPicked] = createSignal<Readonly<Partial<Record<ServiceCategory, string>>>>(
        {},
    );
    const lineup = (): ServiceLineup => (category) => {
        // count how often the category's row has turned
        const name = picked()[category];
        const index = serviceIndexOf(category);
        const advances = turns() > index ? Math.floor((turns() - 1 - index) / serviceCount) + 1 : 0;
        const options = vendorsOf(category);

        // rent the chosen service, or the one the row has turned to
        return name === undefined
            ? present(options[advances % options.length], "turned service")
            : vendorOf(name);
    };
    const rented = () => rentedWith(lineup());

    // hold the service the reader chooses for a row
    const choose = (row: number, name: string) => {
        setIsTouched(true);
        setPicked((current) => ({ ...current, [categoryOf(row - 1)]: name }));
    };

    // light the ledger row pointed at, counting the space as row 0, or else the row that just turned
    const [pointed, setLit] = createSignal<number | undefined>(undefined);
    const lit = () =>
        pointed() ??
        (!isReduced() && !isTouched() && turns() > 0
            ? ((turns() - 1) % serviceCount) + 1
            : undefined);

    // count the services still rented, which a row's switch changes one at a time
    const rentedCount = () =>
        Array.from({ length: serviceCount }, (_, index) => index).filter(
            (index) => !isOpenAt(index + 1),
        ).length;

    // destack or rent one service alone, flipping it in as the stack switch does
    const toggle = (row: number) => {
        setIsTouched(true);
        document.documentElement.dataset["switched"] = "";
        steps.toggle(row);
    };

    return (
        <section
            ref={setSection}
            {...style.attrs(lattice.frame, lattice.ruled, lattice.section, styles.section)}
        >
            <Entry form={noun} isTitle />
            <FigureLabel
                number={1}
                isOpen={properties.isOpen}
                stacked="one prototype, nine services short of an app"
                destacked="one app, one engine"
            >
                <StackSwitch isOpen={properties.isOpen} place="label" />
            </FigureLabel>
            <div {...style.attrs(lattice.cell, lattice.figureCell, styles.stage)}>
                <LabelRow
                    end={
                        isOpenAt(0) ? undefined : (
                            <Picker
                                label="Made with"
                                options={makers}
                                sections={makerSections}
                                shown={maker().name}
                                progress={undefined}
                                onChoose={chooseMaker}
                            />
                        )
                    }
                >
                    <Label>
                        {isOpenAt(0)
                            ? "Your app, in production"
                            : "Your app, stuck where it was made"}
                    </Label>
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
            <div {...style.attrs(lattice.cell, lattice.figureCell, styles.key)}>
                <LabelRow
                    end={
                        tab() === 0 ? (
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
                        ) : undefined
                    }
                >
                    <LabelTabs
                        tabs={[keyOf(rentedCount()), "Your agent"]}
                        shown={tab()}
                        onShow={setTab}
                    />
                </LabelRow>
                {tab() === 1 ? (
                    <AgentChat
                        isOpen={properties.isOpen}
                        moment={moment()}
                        look={lookOf(maker())}
                        connectors={[]}
                    />
                ) : (
                    <Bill
                        isWindow
                        plan="plus"
                        scale={{ people: 1, users: users() }}
                        scales={audiences.map((audience) => ({ people: 1, users: audience }))}
                        lines={entriesWith(lineup()).map((entry, index) => {
                            const mark = present(rented()[index], "rented service").mark;

                            return {
                                entry,
                                workload: workloadOf(entry.stacked.name),
                                tint: accentOf(
                                    entry.stacked.name,
                                    "tint" in mark ? mark.tint : color.mutedForeground,
                                ),
                            };
                        })}
                        isOpenAt={isOpenAt}
                        lighting={{
                            lit: lit(),
                            onLight: setLit,
                            onChoose: setLit,
                            chosen: undefined,
                        }}
                        onToggle={toggle}
                        optionsOf={optionsOf}
                        onChoose={choose}
                    />
                )}
            </div>
        </section>
    );
}

/** Return the services a row offers: its category's, the most common one first. */
function optionsOf(row: number): readonly Option[] {
    return vendorsOf(categoryOf(row - 1)).map((vendor) => ({
        name: vendor.name,
        mark: vendor.mark,
    }));
}

/** Label the ledger by how many vendors the app still rents from: nine, fewer, or one engine. */
function keyOf(rented: number) {
    if (rented === serviceCount) {
        return "Nine vendors";
    } else if (rented === 0) {
        return "One engine";
    }

    return "Fewer vendors";
}

/** The noun section styles. */
const styles = style.create({
    section: {
        gridTemplateRows: { default: "auto auto minmax(0, 1fr)", [screen.belowDesktop]: "auto" },
    },
    stage: {
        containerType: "inline-size",
        gridColumn: { default: "1 / 8", [screen.belowDesktop]: "1 / -1" },
        gridTemplateRows: "auto minmax(0, 1fr)",
    },
    key: {
        gridColumn: { default: "8 / -1", [screen.belowDesktop]: "1 / -1" },
        gridTemplateRows: "auto minmax(0, 1fr)",
    },
    window: {
        minHeight: 0,
        height: { default: "100%", [screen.belowDesktop]: "34rem", [media.maxMd]: "36rem" },
    },
});
