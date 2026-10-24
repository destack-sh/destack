import * as style from "@destack/style";
import { createEffect, createSignal, onCleanup, onSettled, untrack } from "@destack/view";
import { Metadata } from "@destack/view/document";

import { Noun } from "../home/noun/noun";
import { BottomBar } from "../home/entry/switch";
import { Adjective } from "../home/adjective/adjective";
import { Command } from "../home/command/command";
import { Participle } from "../home/participle/participle";
import { firstPicks, promptOf } from "../home/participle/wall";
import type { Tool } from "../home/tool/tool";
import { PastParticiple } from "../home/past/past";
import { Verb } from "../home/verb/verb";
import { lattice } from "../layout/lattice.stylex";
import "../home/figure/figure.css";

/** The figures' entries in page order, each the anchor of the section it heads. */
const FIGURES = [
    "verb",
    "noun",
    "past-participle",
    "imperative",
    "adjective",
    "present-participle",
] as const;

/** The milliseconds a switch's view transition takes, before off-screen figures switch. */
const SETTLE_TIME = 320;

/** The longest wait for an idle moment before switching the next off-screen figure anyway. */
const IDLE_TIMEOUT = 400;

/** The milliseconds between off-screen figures in a browser without idle callbacks. */
const IDLE_FALLBACK = 50;

/** Render the public Destack homepage. */
export function Home() {
    // hold whether the stack is open, which every figure follows
    const [isOpen, setIsOpen] = createSignal(false);

    // follow which page blocks show, so a switch finds the figures on screen without reading layout
    const shown = new Set<Element>();
    let spans: readonly (readonly Element[])[] = [];
    let observer: IntersectionObserver | undefined;
    onSettled(() => {
        spans = spansOf();
        observer = new IntersectionObserver((entries) => {
            for (const entry of entries) {
                // note a block coming into view
                if (entry.isIntersecting) {
                    shown.add(entry.target);
                }
                // forget a block leaving it
                else {
                    shown.delete(entry.target);
                }
            }
        });
        for (const element of spans.flat()) {
            observer.observe(element);
        }
    });
    onCleanup(() => observer?.disconnect());

    // switch the figures on screen in the switch's own frame, and the rest one at a time after it
    const [opens, setOpens] = createSignal<readonly boolean[]>(FIGURES.map(() => false));
    createEffect(isOpen, (open) => {
        // leave the figures alone while each already shows the state, as on the first render
        if (untrack(opens).every((state) => state === open)) {
            return undefined;
        }

        // switch every figure on screen now, inside the switch's view transition
        const isShown = spans.map((span) => span.some((element) => shown.has(element)));
        setOpens((current) =>
            current.map((state, index) => (isShown[index] === true ? open : state)),
        );

        // switch each figure off screen once the switch's transition has settled, one per idle moment
        const later = FIGURES.flatMap((_, index) => (isShown[index] === true ? [] : [index]));
        let isCancelled = false;
        const next = () => {
            // take the next figure, stopping once none is left or a newer switch took over
            const figure = later.shift();
            if (figure === undefined || isCancelled) {
                return;
            }

            // switch it, then wait for the next idle moment
            setOpens((current) => current.map((state, index) => (index === figure ? open : state)));
            whenIdle(next);
        };
        const start = setTimeout(() => whenIdle(next), SETTLE_TIME);

        return () => {
            isCancelled = true;
            clearTimeout(start);
        };
    });
    const isOpenAt = (figure: number) => opens()[figure] ?? false;

    // hold the tools the reader uses, starting from the first figure's, which the install prompt names
    const [picks, setPicks] = createSignal<readonly Tool[]>(firstPicks);
    const pick = (tool: Tool) =>
        setPicks((current) =>
            current.includes(tool)
                ? current.filter((picked) => picked !== tool)
                : [...current, tool],
        );

    // choose a tool from the first figure: pick it, last, so that figure shows it
    const choose = (tool: Tool) =>
        setPicks((current) => [...current.filter((picked) => picked !== tool), tool]);

    return (
        <>
            <Metadata alternates={{ canonical: "/" }} />
            <Verb isOpen={isOpenAt(0)} picks={picks()} onChoose={choose} />
            <Interval />
            <Noun isOpen={isOpenAt(1)} />
            <Interval />
            <PastParticiple isOpen={isOpenAt(2)} onChange={setIsOpen} />
            <Interval />
            <Command isOpen={isOpenAt(3)} />
            <Interval />
            <Adjective isOpen={isOpenAt(4)} />
            <Interval />
            <Participle
                isOpen={isOpenAt(5)}
                picks={picks()}
                onPick={pick}
                prompt={promptOf(picks())}
            />
            <BottomBar isOpen={isOpen()} />
            <Interval />
        </>
    );
}

/** Run a callback in the browser's next idle moment, or soon where it has no idle callbacks. */
function whenIdle(callback: () => void): void {
    // use idle callbacks where they exist
    if (typeof requestIdleCallback === "function") {
        requestIdleCallback(callback, { timeout: IDLE_TIMEOUT });
    }
    // fall back to a short timeout in Safari, which has none
    else {
        setTimeout(callback, IDLE_FALLBACK);
    }
}

/** Return the page blocks each figure spans: its entry's section up to the next figure's. */
function spansOf(): readonly (readonly Element[])[] {
    // require every figure's section
    const starts = FIGURES.map((id) => {
        const section = document.getElementById(id)?.closest("section");
        if (section === null || section === undefined) {
            throw new TypeError(`no section heads the figure ${id}`);
        }

        return section;
    });

    // walk from each start to the next one, or to the page's end
    return starts.map((start, index) => {
        const span: Element[] = [];
        for (
            let element: Element | null = start;
            element !== null && element !== starts[index + 1];
            element = element.nextElementSibling
        ) {
            span.push(element);
        }

        return span;
    });
}

/** Space two sections, or the last one and the footer, apart inside the frame. */
function Interval() {
    return <div aria-hidden="true" {...style.attrs(lattice.frame, lattice.interval)} />;
}
