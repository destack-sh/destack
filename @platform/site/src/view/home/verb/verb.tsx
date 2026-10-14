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
import { Label } from "../entry/label";
import { Bill } from "../figure/bill";
import { accentOf, type Item, marksOf } from "../figure/item";
import {
    everyRow,
    holdings,
    pages,
    Presence,
    PresenceAnchor,
    SpaceBrowser,
    type Viewer,
} from "./space";
import type { Stagger } from "../figure/stagger";
import { workloadOf } from "../figure/price";
import { StackSwitch } from "../entry/switch";
import { screen } from "../../layout/screen.stylex";

/** The milliseconds the pointer rests on a holding before the browser turns to its page. */
const settleIn = 80;

/** The milliseconds after the pointer leaves before the browser turns back. */
const settleOut = 300;

/** The milliseconds each page of your corner holds before the browser turns to the next. */
const pageTime = 5200;

/** The verb, which titles the page. */
const verb: Form = {
    syllables: ["de", "stack"],
    pronunciation: "/diːˈstak/",
    partOfSpeech: "verb",
    sense: {
        definition: "to reconquer your software",
        highlight: ["reconquer"],
        sentence: "Own your apps and agents in one personal superapp.",
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
        onLight: (row: number | undefined) => {
            // light the row at once, and turn the browser only once the pointer rests
            setLit(row);
            clearTimeout(settling);
            settling = setTimeout(() => setSettled(row), row === undefined ? settleOut : settleIn);
        },
    };

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

    // show the page of the pointed or chosen holding, or else the page the window has turned to
    const page = () =>
        present(
            pages.find((candidate) => candidate.row === (settled() ?? chosen())) ??
                pages[turned() % pages.length],
            "shown page",
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
                stacked="eight separate apps, eight separate stacks"
                destacked="eight unified apps, one integrated space"
            >
                <StackSwitch isOpen={properties.isOpen} place="label" />
            </FigureLabel>
            <div {...style.attrs(lattice.cell, lattice.figureCell, styles.stage)}>
                <Label>
                    {isOpenAt(0)
                        ? "Your space, and your agent sees all of it"
                        : "Eight rented apps, none truly yours"}
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
                />
            </div>
            <div {...style.attrs(lattice.cell, lattice.figureCell, styles.key)}>
                <Fade isOpen={isOpenAt(0)}>
                    <Label>{isOpenAt(0) ? "One bill" : "Eight bills"}</Label>
                </Fade>
                <Bill
                    lines={holdings.map((entry) => ({
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
