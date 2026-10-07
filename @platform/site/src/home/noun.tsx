import { color } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";
import { createSignal } from "@destack/view";

import { lattice } from "../style/lattice.stylex";
import { tokens } from "../style/tokens.stylex";
import { Entry, FigureLabel, type Form } from "./entry";
import { Fade } from "./fade";
import { Inspector } from "./inspector";
import { Label } from "./label";
import { Ledger } from "./ledger";
import { Tile } from "./tile";
import { PagesApp, services } from "./page";
import { createSteps } from "./stagger";
import { StackSwitch } from "./switch";
import { Window } from "./window";

/** The media query for screens narrower than the desktop frame, where the cells stack. */
const narrow = "@media (max-width: 1099px)";

/** The milliseconds between the steps of the figure's switch, one per service. */
const stepTime = 40;

/** The noun. */
const noun: Form = {
    syllables: ["De", "stack"],
    pronunciation: "/ˈdiːstak/",
    partOfSpeech: "noun",
    sense: {
        definition: "a complete, absurdly integrated software engine",
        highlight: ["complete", "integrated"],
        sentence:
            "Auth, secrets, workflows, sync and more – everything serious software needs, already included.",
    },
};

/** Show the noun at work: a docs app taken apart into the twelve services it runs on, rented one by one or built into one engine. */
export function Noun(properties: { isOpen: boolean }) {
    // hold the service pointed at, in the app or in the ledger
    const [lit, setLit] = createSignal<number | undefined>(undefined);
    const lighting = {
        get lit() {
            return lit();
        },
        onLight: setLit,
    };

    // switch the app's parts and the ledger's rows in service order before the rest
    const steps = createSteps(() => properties.isOpen, services.length + 1, stepTime);
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
            {...stylex.attrs(
                lattice.frame,
                lattice.ruled,
                lattice.section,
                lattice.ruleBottom,
                styles.section,
            )}
        >
            <Entry form={noun} />
            <FigureLabel
                number={4}
                title={
                    properties.isOpen
                        ? "one app, built on one engine"
                        : "one app, wired to twelve vendors"
                }
            >
                <StackSwitch isOpen={properties.isOpen} />
            </FigureLabel>
            <div {...stylex.attrs(lattice.cell, lattice.figureCell, styles.stage)}>
                <Label>Pages, the launch plan</Label>
                <Window
                    title={
                        <span {...stylex.attrs(styles.title)}>
                            <Tile name="pages" />
                            Pages
                        </span>
                    }
                    style={styles.window}
                >
                    <Inspector
                        isOpenAt={isOpenAt}
                        lighting={lighting}
                        services={services.map((service) => service.destacked.name)}
                    >
                        <PagesApp isOpenAt={isOpenAt} />
                    </Inspector>
                </Window>
            </div>
            <div {...stylex.attrs(lattice.cell, lattice.figureCell, styles.key)}>
                <div {...stylex.attrs(styles.keyHead)}>
                    <Fade isOpen={rented() === 0}>
                        <Label>{keyOf(rented())}</Label>
                    </Fade>
                    <span {...stylex.attrs(styles.hint)}>Click a row to destack it</span>
                </div>
                <Ledger
                    entries={services}
                    total={totalOf(rented())}
                    lighting={lighting}
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
        return "Twelve vendors, wired together";
    } else if (rented === 0) {
        return "One engine, built in";
    }

    return "Some vendors, some engine";
}

/** Total the vendors, accounts and invoices behind the services still rented and the engine behind the rest. */
function totalOf(rented: number): readonly [label: string, value: string] {
    // count the engine once any service runs on it
    const hasEngine = rented < services.length;
    const accounts = rented + (hasEngine ? 1 : 0);
    const vendors = rented === 0 ? "" : `${rented} ${rented === 1 ? "vendor" : "vendors"}`;
    const label = [vendors, hasEngine ? "1 engine" : ""].filter((part) => part !== "").join(" + ");

    return [
        label,
        `${accounts} ${accounts === 1 ? "account" : "accounts"} · ${accounts} ${accounts === 1 ? "invoice" : "invoices"}`,
    ];
}

/** The noun section styles. */
const styles = stylex.create({
    keyHead: {
        alignItems: "baseline",
        display: "flex",
        gap: "1rem",
        justifyContent: "space-between",
    },
    hint: {
        color: color.mutedForeground,
        fontFamily: tokens.monoFont,
        fontSize: "0.6875rem",
        letterSpacing: "0.04em",
        opacity: 0.7,
        whiteSpace: "nowrap",
        [narrow]: { display: "none" },
    },
    section: {
        gridTemplateRows: "auto auto minmax(0, 1fr)",
        [narrow]: { gridTemplateRows: "auto" },
    },
    stage: {
        gridColumn: "1 / 8",
        gridTemplateRows: "auto minmax(0, 1fr)",
        [narrow]: { gridColumn: "1 / -1" },
    },
    key: {
        gridColumn: "8 / -1",
        gridTemplateRows: "auto minmax(0, 1fr)",
        [narrow]: { gridColumn: "1 / -1" },
    },
    window: {
        minHeight: 0,
        [narrow]: { height: "30rem" },
    },
    title: {
        alignItems: "center",
        display: "inline-flex",
        fontWeight: 600,
        gap: "0.375rem",
    },
});
