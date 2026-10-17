import { present } from "@destack/schema";
import * as style from "@destack/style";
import { color } from "@destack/theme/tokens.stylex";
import { createSignal } from "@destack/view";
import { createTimer } from "@destack/view/primitives/timer";
import { createReducedMotion } from "@destack/view/motion";
import { createVisibilityObserver } from "@destack/view/primitives/intersection-observer";

import { lattice } from "../../layout/lattice.stylex";
import { Entry, FigureLabel, type Form } from "../entry/entry";
import { Fade } from "../figure/fade";
import { Label, LabelRow } from "../entry/label";
import { Dial, teams } from "../figure/dial";
import { Bill } from "../figure/bill";
import { accentOf, type Item, marksOf } from "../figure/item";
import {
    everyRow,
    holdingsWith,
    pages,
    Presence,
    PresenceAnchor,
    SpaceBrowser,
    type Viewer,
} from "./space";
import type { Stagger } from "../figure/stagger";
import { BASE_USERS, workloadOf } from "../figure/price";
import { StackSwitch } from "../entry/switch";
import { sources } from "./source";
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
const TASKS_ROW = 2;

/** The verb, which titles the page. */
const verb: Form = {
    syllables: ["de", "stack"],
    pronunciation: "/diːˈstak/",
    partOfSpeech: "verb",
    sense: {
        definition: "to reconquer your software",
        highlight: ["reconquer"],
        sentence: undefined,
        points: [
            { label: "Own your apps", href: "#verb" },
            { label: "Keep your context", href: "#past-participle" },
            { label: "Integrate your agents", href: "#imperative" },
        ],
        link: { label: "Build your personal superapp", href: "#present-participle" },
    },
};

/** Return the tint of an item's first mark, or the muted tone where it has none. */
function tintOf(item: Item): string {
    const [mark] = marksOf(item.mark);

    return mark !== undefined && "tint" in mark ? mark.tint : color.mutedForeground;
}

/** Open the page with the verb, illustrated by your space in the cloud. */
export function Verb(properties: { isOpen: boolean }) {
    // hold the holding pointed at, the one the browser settles on a moment later, and the one chosen
    const [lit, setLit] = createSignal<number | undefined>(undefined);
    const [settled, setSettled] = createSignal<number | undefined>(undefined);
    const [chosen, setChosen] = createSignal<number | undefined>(undefined);
    let settling: ReturnType<typeof setTimeout> | undefined;
    const lighting = {
        get lit() {
            return lit() ?? chosen();
        },
        onChoose: setChosen,
        get chosen() {
            return chosen();
        },
        onLight: (row: number | undefined) => {
            // light the row at once, and turn the browser only once the pointer rests
            setLit(row);
            clearTimeout(settling);
            settling = setTimeout(() => setSettled(row), row === undefined ? settleOut : settleIn);
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

    // hold who is looking, whether the reader chose them, and the page the window turns to on its own
    const [viewer, setViewer] = createSignal<Viewer>("Me");
    const [isViewerChosen, setIsViewerChosen] = createSignal(false);
    const [turned, setTurned] = createSignal(0);
    const chooseViewer = (next: Viewer) => {
        setIsViewerChosen(true);
        setViewer(next);
    };

    // ask your agent to change an app, which it does a moment later
    const [remixed, setRemixed] = createSignal<readonly number[]>([]);
    const [asking, setAsking] = createSignal<number | undefined>(undefined);
    const remix = (row: number) => {
        setAsking(row);
        setTimeout(() => {
            setRemixed((rows) => (rows.includes(row) ? rows : [...rows, row]));
            setAsking(undefined);
        }, remixTime);
    };

    // turn to the next personal app each time the browser comes round
    const source = () =>
        present(sources[Math.floor(turned() / pages.length) % sources.length], "turned source");

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
            page().row === TASKS_ROW &&
            remixed().length === 0 &&
            asking() === undefined &&
            !isReduced()
                ? firstRemixAt
                : false,
        setTimeout,
    );

    // turn to the next page the viewer may see every page time, held while still or off screen
    createTimer(
        () => {
            // look as your agent every other round once owned, until the reader picks a viewer
            const round = Math.floor((turned() + 1) / pages.length);
            if (isOpenAt(1) && !isViewerChosen()) {
                setViewer(round % 2 === 1 ? "Agent" : "Me");
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
        () => (isReduced() || chosen() !== undefined || !isShown() ? false : pageTime),
        setInterval,
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
            <Entry form={verb} isTitle />
            <FigureLabel
                number={1}
                isOpen={properties.isOpen}
                stacked="eight apps, rented as silos"
                destacked="eight apps, owned as one space"
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
                    isChanged={false}
                    isOpenAt={isOpenAt}
                    page={page()}
                    viewer={viewer()}
                    onViewer={chooseViewer}
                    onChoose={setChosen}
                    source={source()}
                    vendor={undefined}
                    remix={{ remixed: remixed(), asking: asking(), onRemix: remix }}
                />
            </div>
            <div {...style.attrs(lattice.cell, lattice.figureCell, styles.key)}>
                <LabelRow
                    end={
                        <Dial
                            label="People"
                            options={teams}
                            value={people()}
                            format={String}
                            onChange={setPeople}
                        />
                    }
                >
                    <Fade isOpen={isOpenAt(0)}>
                        <Label>{isOpenAt(0) ? "Your space, by use" : "Rent, per seat"}</Label>
                    </Fade>
                </LabelRow>
                <Bill
                    isWindow
                    scale={{ people: people(), users: BASE_USERS }}
                    scales={teams.map((team) => ({ people: team, users: BASE_USERS }))}
                    lines={holdingsWith(source()).map((entry) => ({
                        entry,
                        workload: workloadOf(entry.stacked.name),
                        tint: accentOf(entry.stacked.name, tintOf(entry.stacked)),
                    }))}
                    lighting={lighting}
                    isOpenAt={isOpenAt}
                    badgeOf={(row) => <PresenceAnchor row={row} />}
                />
                <Presence row={page().row} isShown={isOpenAt(1)} />
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
