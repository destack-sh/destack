import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color } from "@destack/theme/tokens.stylex";
import { createSignal } from "@destack/view";

import { lattice } from "../../layout/lattice.stylex";
import { Entry, FigureLabel, type Form } from "../entry/entry";
import { Fade } from "../figure/fade";
import { Label, LabelRow } from "../entry/label";
import { audienceOf, audiences, Dial } from "../figure/dial";
import { Bill } from "../figure/bill";
import { accentOf } from "../figure/item";
import { XRay } from "./xray";
import { services } from "./service";
import { createSteps } from "../figure/stagger";
import { BASE_USERS, workloadOf } from "../figure/price";
import { StackSwitch } from "../entry/switch";
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
            "Auth, data, workflows and views, shared by every app, so any app can build on the others.",
    },
};

/** Show the noun at work: a docs app on ten rented services, or built on one engine. */
export function Noun(properties: { isOpen: boolean }) {
    // follow the motion the section shows
    const [section, setSection] = createSignal<HTMLElement>();
    const isReduced = createReducedMotion(section);

    // switch each service and its ledger row in order, after the space itself
    const steps = createSteps(() => properties.isOpen, services.length + 1, stepTime, isReduced);
    const isOpenAt = steps.isOpenAt;

    // read the bill for an app with as many users as the reader dials
    const [users, setUsers] = createSignal<number>(BASE_USERS);

    // hold the service pointed at, in the diagram or the bill
    const [lit, setLit] = createSignal<number | undefined>(undefined);

    // count the services still rented, which a row's switch changes one at a time
    const rented = () => services.filter((_, index) => !isOpenAt(index + 1)).length;

    // destack or rent one service alone, flipping it in as the stack switch does
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
                stacked="your apps, glued to ten vendors"
                destacked="your apps, built on one engine"
            >
                <StackSwitch isOpen={properties.isOpen} place="label" />
            </FigureLabel>
            <div {...style.attrs(lattice.cell, lattice.figureCell, styles.stage)}>
                <Label>
                    {isOpenAt(0) ? "One engine under every app" : "Ten vendors under every app"}
                </Label>
                <div {...style.attrs(styles.window)}>
                    <XRay isOpenAt={isOpenAt} lit={lit()} onLight={setLit} />
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
                            onChange={setUsers}
                        />
                    }
                >
                    <Fade isOpen={rented() === 0}>
                        <Label>{keyOf(rented())}</Label>
                    </Fade>
                </LabelRow>
                <Bill
                    scale={{ people: 1, users: users() }}
                    scales={audiences.map((audience) => ({ people: 1, users: audience }))}
                    lines={services.map((entry) => ({
                        entry,
                        workload: workloadOf(entry.stacked.name),
                        tint: accentOf(entry.stacked.name, color.mutedForeground),
                    }))}
                    isOpenAt={isOpenAt}
                    lighting={{ lit: lit(), onLight: setLit, onChoose: setLit }}
                    onToggle={toggle}
                />
            </div>
        </section>
    );
}

/** Label the ledger by how much of the stack is rented: all of it, none of it, or some. */
function keyOf(rented: number) {
    if (rented === services.length) {
        return "Ten services, ten vendors";
    } else if (rented === 0) {
        return "Ten services, one engine";
    }

    return "Ten services, fewer vendors";
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
