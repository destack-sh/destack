import { PlainDate, TimeZone } from "@destack/schema";
import * as style from "@destack/style";
import {
    createEffect,
    createSignal,
    onCleanup,
    onSettled,
    untrack,
    useEnvironment,
} from "@destack/view";
import { Metadata } from "@destack/view/document";

import { Noun, noun } from "../home/noun/noun";
import { BottomBar } from "../home/entry/switch";
import { Adjective, destackable } from "../home/adjective/adjective";
import { Participle, participle } from "../home/participle/participle";
import { anchorOf, type Form } from "../home/entry/entry";
import { ThumbIndex } from "../home/entry/thumb";
import { firstPicks, promptOf } from "../home/participle/wall";
import { setInstallPrompt } from "../install/prompt";
import type { Tool } from "../home/tool/tool";
import { destacked, PastParticiple } from "../home/past/past";
import { Verb, verb } from "../home/verb/verb";
import { Story, StoryContext } from "../home/verb/story";
import { Interval } from "../layout/sheet";
import { lattice } from "../layout/lattice.stylex";
import "../home/figure/figure.css";
import { requestIdle } from "@destack/view/primitives/scheduled";

/** The sections' dictionary forms in page order. */
const FORMS: readonly Form[] = [verb, noun, destacked, destackable, participle];

/** The figures' entries in page order, each the anchor of the section it heads. */
const FIGURES = FORMS.map(anchorOf);

/** The milliseconds on-screen figures take to fade into a switch, before off-screen ones switch. */
const SETTLE_TIME = 320;

/** The longest wait for an idle moment before switching the next off-screen figure anyway. */
const IDLE_TIMEOUT = 400;

/** Render the public Destack homepage, its story told from a day in a time zone. */
export function Home(properties: { readonly today: PlainDate; readonly zone: string }) {
    // hold the page's clock and display, and whether the stack is open, which every figure follows
    const environment = useEnvironment();
    const { clock } = environment;
    const [isOpen, setIsOpen] = createSignal(false);

    // tell the story from the day the page was rendered, then from the reader's week
    const [story, setStory] = createSignal(
        untrack(() => Story.of(properties.today, properties.zone)),
    );
    onSettled(() => {
        const today = TimeZone.date(environment.timeZone, clock.now());
        const reader = Story.of(today, environment.timeZone);
        if (!PlainDate.equals(reader.monday, story().monday) || reader.zone !== story().zone) {
            setStory(reader);
        }
    });

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

        // switch every figure on screen now
        const isShown = spans.map((span) => span.some((element) => shown.has(element)));
        setOpens((current) =>
            current.map((state, index) => (isShown[index] === true ? open : state)),
        );

        // switch each figure off screen once the switch has settled, one per idle moment
        const later = FIGURES.flatMap((_, index) => (isShown[index] === true ? [] : [index]));
        let cancelIdle: (() => void) | undefined;
        const next = () => {
            // take the next figure, stopping once none is left
            const figure = later.shift();
            if (figure === undefined) {
                return;
            }

            // switch it, then wait for the next idle moment
            setOpens((current) => current.map((state, index) => (index === figure ? open : state)));
            cancelIdle = requestIdle(next, IDLE_TIMEOUT, environment.display);
        };
        const start = clock.setTimeout(() => {
            cancelIdle = requestIdle(next, IDLE_TIMEOUT, environment.display);
        }, SETTLE_TIME);

        // stop once a newer switch takes over
        return () => {
            clock.clearTimeout(start);
            cancelIdle?.();
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

    // offer the picked tools in the footer's install prompt
    createEffect(picks, (current) => {
        setInstallPrompt(promptOf(current));
    });

    return (
        <StoryContext value={story}>
            <Metadata alternates={{ canonical: "/" }} />
            <Verb isOpen={isOpenAt(0)} picks={picks()} />
            <Interval section={2}>
                <RunningHead number={2} form={noun} />
            </Interval>
            <Noun isOpen={isOpenAt(1)} />
            <Interval section={3}>
                <RunningHead number={3} form={destacked} />
            </Interval>
            <PastParticiple isOpen={isOpenAt(2)} onChange={setIsOpen} />
            <Interval section={4}>
                <RunningHead number={4} form={destackable} />
            </Interval>
            <Adjective isOpen={isOpenAt(3)} />
            <Interval section={5}>
                <RunningHead number={5} form={participle} />
            </Interval>
            <Participle isOpen={isOpenAt(4)} picks={picks()} onPick={pick} />
            <BottomBar isOpen={isOpen()} />
            <Interval />
            <ThumbIndex forms={FORMS} />
        </StoryContext>
    );
}

/** Return the page blocks each figure spans: its entry's section up to the next figure's. */
function spansOf(): readonly (readonly Element[])[] {
    // require every figure's section
    const starts = FIGURES.map((id) => {
        const section = document.getElementById(id)?.closest("section");
        if (section === null || section === undefined) {
            throw new TypeError(`no section heads the figure '${id}'`);
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

/** Head the next section as a dictionary page does: number, part of speech, headword. */
function RunningHead(properties: { number: number; form: Form }) {
    const number = String(properties.number).padStart(2, "0");

    return (
        <div {...style.attrs(lattice.head)}>
            {/* set the guide words */}
            <span {...style.attrs(lattice.guide)}>
                <b {...style.attrs(lattice.number)}>§ {number}</b> /{" "}
                {String(FIGURES.length).padStart(2, "0")} — {properties.form.partOfSpeech}
            </span>
            <span {...style.attrs(lattice.guide, lattice.guideWord)}>
                {properties.form.syllables.join("·")}
            </span>
        </div>
    );
}
