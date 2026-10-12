import * as style from "@destack/style";
import { color } from "@destack/theme/tokens.stylex";
import { createSignal } from "@destack/view";

import { lattice } from "../../layout/lattice.stylex";
import { Entry, FigureLabel, type Form } from "../entry/entry";
import { Fade } from "../figure/fade";
import { Label } from "../entry/label";
import { Bill } from "../figure/bill";
import { accentOf } from "../figure/item";
import { Favicon } from "../figure/glyph";
import { Tile } from "../figure/tile";
import { PagesApp, services } from "./pages";
import { createSteps, type Stagger } from "../figure/stagger";
import { workloadOf } from "../figure/price";
import { StackSwitch } from "../entry/switch";
import { Window } from "../figure/window";
import { screen } from "../../layout/screen.stylex";
import { createReducedMotion } from "@destack/view/motion";

/** The milliseconds between the steps of the figure's switch, one per service. */
const stepTime = 40;

/** The noun. */
const noun: Form = {
    syllables: ["De", "stack"],
    pronunciation: "/ˈdiːstak/",
    partOfSpeech: "noun",
    sense: {
        definition: "a complete, absurdly integrated software engine",
        highlight: ["complete", "absurdly integrated"],
        sentence:
            "Auth, secrets, workflows, sync and more – everything serious software needs, solved once.",
    },
};

/** Show the noun at work: a docs app taken apart into the ten services it runs on, rented one by one or built into one engine. */
export function Noun(properties: { isOpen: boolean }) {
    // follow the motion the section shows
    const [section, setSection] = createSignal<HTMLElement>();
    const isReduced = createReducedMotion(section);

    // switch the app's parts and the ledger's rows in service order before the rest
    const steps = createSteps(() => properties.isOpen, services.length + 1, stepTime, isReduced);
    const isOpenAt = steps.isOpenAt;

    // count the services still rented, which a row's switch changes one at a time
    const rented = () => services.filter((_, index) => !isOpenAt(index + 1)).length;

    // destack or rent one service alone, flipping its parts in as the stack switch does
    const toggle = (row: number) => {
        document.documentElement.dataset["switched"] = "";
        steps.toggle(row);
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
            <Entry form={noun} />
            <FigureLabel
                number={3}
                isOpen={properties.isOpen}
                stacked="one app, glued to ten vendors"
                destacked="one app, built on one engine"
            >
                <StackSwitch isOpen={properties.isOpen} />
            </FigureLabel>
            <div {...style.attrs(lattice.cell, lattice.figureCell, styles.stage)}>
                <Label>{isOpenAt(0) ? "Pages, a docs app" : "Notion, a docs app"}</Label>
                <Window
                    title={
                        <span {...style.attrs(styles.title)}>
                            {isOpenAt(0) ? (
                                <>
                                    <Tile name="pages" />
                                    Pages
                                </>
                            ) : (
                                <>
                                    <Favicon icon="notion" tint="#191919" />
                                    Notion
                                </>
                            )}
                        </span>
                    }
                    xstyle={styles.window}
                >
                    <PagesApp isOpenAt={isOpenAt} />
                </Window>
            </div>
            <div {...style.attrs(lattice.cell, lattice.figureCell, styles.key)}>
                <div {...style.attrs(styles.keyHead)}>
                    <Fade isOpen={rented() === 0}>
                        <Label>{keyOf(rented())}</Label>
                    </Fade>
                </div>
                <Bill
                    lines={services.map((entry) => ({
                        entry,
                        workload: workloadOf(entry.stacked.name),
                        tint: accentOf(entry.stacked.name, color.mutedForeground),
                    }))}
                    label={labelOf(isOpenAt)}
                    isOpenAt={isOpenAt}
                    onToggle={toggle}
                />
            </div>
        </section>
    );
}

/** Label the ledger by how much of the stack is rented: all of it, none of it, or some. */
function keyOf(rented: number) {
    if (rented === services.length) {
        return "Ten vendors, glued together";
    } else if (rented === 0) {
        return "One unified software engine";
    }

    return "Some vendors, some engine";
}

/** Label the bill: the vendors still rented, and the engine behind the rest. */
function labelOf(isOpenAt: Stagger): string {
    // count the vendors still rented, and the engine once any service runs on it
    const rented = services.filter((_, index) => !isOpenAt(index + 1)).length;
    const vendors = rented === 0 ? "" : `${rented} ${rented === 1 ? "vendor" : "vendors"}`;
    const engine = rented < services.length ? "1 engine" : "";

    return [vendors, engine].filter((part) => part !== "").join(" + ");
}

/** The noun section styles. */
const styles = style.create({
    keyHead: {
        alignItems: "baseline",
        display: "flex",
        gap: "1rem",
        justifyContent: "space-between",
    },
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
        height: { default: null, [screen.belowDesktop]: "30rem" },
    },
    title: {
        alignItems: "center",
        display: "inline-flex",
        fontWeight: 600,
        gap: "0.375rem",
    },
});
