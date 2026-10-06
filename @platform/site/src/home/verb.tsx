import { present } from "@destack/schema";
import * as stylex from "@destack/style";
import { createSignal, onSettled } from "@destack/view";

import { lattice } from "../style/lattice.stylex";
import { Entry, FigureLabel, type Form } from "./entry";
import { Fade } from "./fade";
import { Inspector } from "./inspector";
import { Label } from "./label";
import { Ledger } from "./ledger";
import { holdings, pages, SpaceBrowser, type Viewer } from "./space";
import { createSteps } from "./stagger";
import { StackSwitch } from "./switch";

/** The media query for screens narrower than the desktop frame, where the cells stack. */
const narrow = "@media (max-width: 1099px)";

/** The milliseconds between the steps of the figure's switch, one per holding. */
const stepTime = 110;

/** The milliseconds the pointer rests on a holding before the browser turns to its page. */
const settleIn = 140;

/** The milliseconds after the pointer leaves before the browser turns back. */
const settleOut = 400;

/** The milliseconds each page of your corner holds before the browser turns to the next. */
const pageTime = 5200;

/** The verb, which titles the page. */
const verb: Form = {
    syllables: ["de", "stack"],
    pronunciation: "/diːˈstak/",
    partOfSpeech: "verb",
    sense: {
        definition: "to take back your software",
        highlight: ["back"],
        sentence: "Own your apps and agents in one unified personal software platform.",
    },
};

/** Open the page with the verb, illustrated by your corner of the cloud: spread over rented sites, or held in one space at your own address. */
export function Verb(properties: { isOpen: boolean }) {
    // hold the holding pointed at, and the one the browser settles on a moment later
    const [lit, setLit] = createSignal<number | undefined>(undefined);
    const [settled, setSettled] = createSignal<number | undefined>(undefined);
    let settling: ReturnType<typeof setTimeout> | undefined;
    const lighting = {
        get lit() {
            return lit();
        },
        onLight: (row: number | undefined) => {
            // light the row at once, and turn the browser only once the pointer rests
            setLit(row);
            clearTimeout(settling);
            settling = setTimeout(() => setSettled(row), row === undefined ? settleOut : settleIn);
        },
    };

    // switch the browser first and then each holding in order
    const { isOpenAt } = createSteps(() => properties.isOpen, holdings.length + 1, stepTime);

    // hold who is looking, whether the reader chose them, and the page the window turns to on its own
    const [viewer, setViewer] = createSignal<Viewer>("Me");
    const [isViewerChosen, setIsViewerChosen] = createSignal(false);
    const [turned, setTurned] = createSignal(0);
    const chooseViewer = (chosen: Viewer) => {
        setIsViewerChosen(true);
        setViewer(chosen);
    };

    // show the page of the lit holding, or else the page the window has turned to
    const page = () =>
        present(
            pages.find((candidate) => candidate.row === settled()) ??
                pages[turned() % pages.length],
            "shown page",
        );

    // turn to the next page the viewer may see, unless motion is reduced
    onSettled(() => {
        if (matchMedia("(prefers-reduced-motion: reduce)").matches) {
            return undefined;
        }
        const timer = setInterval(() => {
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
        }, pageTime);

        return () => clearInterval(timer);
    });

    return (
        <section
            {...stylex.attrs(
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
                title={
                    properties.isOpen
                        ? "the same seven, in one space you own"
                        : "seven siloed apps, seven separate stacks"
                }
            >
                <StackSwitch isOpen={properties.isOpen} isHinted />
            </FigureLabel>
            <div {...stylex.attrs(lattice.cell, lattice.figureCell, styles.stage)}>
                <Label>Your corner of the cloud</Label>
                <Inspector
                    isOpenAt={isOpenAt}
                    lighting={lighting}
                    services={holdings.map((holding) => holding.destacked.name)}
                >
                    <SpaceBrowser
                        isOpenAt={isOpenAt}
                        page={page()}
                        viewer={viewer()}
                        onViewer={chooseViewer}
                    />
                </Inspector>
            </div>
            <div {...stylex.attrs(lattice.cell, lattice.figureCell, styles.key)}>
                <Fade isOpen={isOpenAt(0)}>
                    <Label>
                        {isOpenAt(0) ? "One space, owned by you" : "Seven sites, seven logins"}
                    </Label>
                </Fade>
                <Ledger
                    entries={holdings}
                    total={
                        isOpenAt(holdings.length + 1)
                            ? ["1 space, 1 stack", "1 account · you.dev"]
                            : ["7 sites, 7 stacks", "7 accounts · 7 invoices"]
                    }
                    lighting={lighting}
                    isOpenAt={isOpenAt}
                    columns={1}
                />
            </div>
        </section>
    );
}

/** The verb section styles. */
const styles = stylex.create({
    section: {
        gridTemplateRows: "auto auto minmax(0, 1fr)",
        [narrow]: { gridTemplateRows: "auto" },
    },
    stage: {
        gridColumn: "1 / 8",
        gridTemplateRows: { default: "auto minmax(0, 1fr)", [narrow]: "auto 30rem" },
        [narrow]: { gridColumn: "1 / -1" },
    },
    key: {
        gridColumn: "8 / -1",
        gridTemplateRows: "auto minmax(0, 1fr)",
        [narrow]: { gridColumn: "1 / -1" },
    },
});
