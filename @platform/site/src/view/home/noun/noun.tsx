import { color, font } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import { createSignal } from "@destack/view";

import { lattice } from "../../layout/lattice.stylex";
import { Entry, FigureLabel, type Form } from "../entry/entry";
import { Fade } from "../figure/fade";
import { Inspector } from "../figure/inspector";
import { Label } from "../entry/label";
import { Ledger } from "../figure/ledger";
import { Tile } from "../figure/tile";
import { PagesApp, services } from "./pages";
import { createSteps } from "../figure/stagger";
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
                title={
                    properties.isOpen
                        ? "one app, built on one engine"
                        : "one app, wired to twelve vendors"
                }
            >
                <StackSwitch isOpen={properties.isOpen} />
            </FigureLabel>
            <div {...style.attrs(lattice.cell, lattice.figureCell, styles.stage)}>
                <Label>Pages, the launch plan</Label>
                <Window
                    title={
                        <span {...style.attrs(styles.title)}>
                            <Tile name="pages" />
                            Pages
                        </span>
                    }
                    xstyle={styles.window}
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
            <div {...style.attrs(lattice.cell, lattice.figureCell, styles.key)}>
                <div {...style.attrs(styles.keyHead)}>
                    <Fade isOpen={rented() === 0}>
                        <Label>{keyOf(rented())}</Label>
                    </Fade>
                    <span {...style.attrs(styles.hint)}>Click a row to destack it</span>
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
const styles = style.create({
    keyHead: {
        alignItems: "baseline",
        display: "flex",
        gap: "1rem",
        justifyContent: "space-between",
    },
    hint: {
        color: color.mutedForeground,
        fontFamily: font.code,
        fontSize: "0.6875rem",
        letterSpacing: "0.04em",
        opacity: 0.7,
        whiteSpace: "nowrap",
        display: { default: null, [screen.belowDesktop]: "none" },
    },
    section: {
        gridTemplateRows: { default: "auto auto minmax(0, 1fr)", [screen.belowDesktop]: "auto" },
    },
    stage: {
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
