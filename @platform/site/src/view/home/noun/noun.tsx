import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color } from "@destack/theme/tokens.stylex";
import { present } from "@destack/schema";
import { createSignal } from "@destack/view";
import { createVisibilityObserver } from "@destack/view/primitives/intersection-observer";
import { createTimer } from "@destack/view/primitives/timer";

import { lattice } from "../../layout/lattice.stylex";
import { Entry, FigureLabel, type Form } from "../entry/entry";
import { Fade } from "../figure/fade";
import { Label, LabelRow } from "../entry/label";
import { audienceOf, audiences, Dial } from "../figure/dial";
import { Bill } from "../figure/bill";
import { accentOf } from "../figure/item";
import { XRay } from "./xray";
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
import type { Option } from "../figure/picker";
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
        definition: "a complete, absurdly integrated software engine",
        highlight: ["complete", "absurdly integrated"],
        sentence: "Auth, sync, workflows, telemetry — great personal software, out of the box.",
    },
};

/** Show the noun at work: a docs app on eight rented services, or built on one engine. */
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
        setInterval,
    );

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
                stacked="a simple app gluing eight services"
                destacked="a simple app on Destack"
            >
                <StackSwitch isOpen={properties.isOpen} place="label" />
            </FigureLabel>
            <div {...style.attrs(lattice.cell, lattice.figureCell, styles.stage)}>
                <Label>
                    {isOpenAt(0)
                        ? "Your app, built on one engine"
                        : "Your app, stitched across eight services"}
                </Label>
                <div {...style.attrs(styles.window)}>
                    <XRay rented={rented()} isOpenAt={isOpenAt} lit={lit()} onLight={setLit} />
                </div>
            </div>
            <div {...style.attrs(lattice.cell, lattice.figureCell, styles.key)}>
                <LabelRow
                    end={
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
                    }
                >
                    <Fade isOpen={rentedCount() === 0}>
                        <Label>{keyOf(rentedCount())}</Label>
                    </Fade>
                </LabelRow>
                <Bill
                    isWindow
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
                    lighting={{ lit: lit(), onLight: setLit, onChoose: setLit, chosen: undefined }}
                    onToggle={toggle}
                    optionsOf={optionsOf}
                    onChoose={choose}
                />
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

/** Label the ledger by how many bills the stack sends: eight, one, or some between. */
function keyOf(rented: number) {
    if (rented === serviceCount) {
        return "Eight bills";
    } else if (rented === 0) {
        return "One bill";
    }

    return "Fewer bills";
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
