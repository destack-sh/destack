import { frame } from "../../layout/frame.stylex";
import { color, font } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import { Crossfade } from "../figure/crossfade";
import { createSignal, For, type JSX, onSettled } from "@destack/view";
import { createReducedMotion } from "@destack/view/motion";
import {
    createVisibilityObserver,
    makeIntersectionObserver,
} from "@destack/view/primitives/intersection-observer";
import { createTimer } from "@destack/view/primitives/timer";

import { lattice } from "../../layout/lattice.stylex";
import { media } from "@destack/style/media.stylex";
import { screen } from "../../layout/screen.stylex";
import { Glyph } from "../figure/glyph";

/** The milliseconds each sense of a phrase stays lit before the light moves to the next. */
const LIGHT_TIME = 2600;

/** One sense of a form: its definition, the passages underlined in it, and the phrase under it. */
type Sense = {
    /** The definition. */
    definition: string;
    /** The passages of the definition drawn underlined, in order. */
    highlight: readonly string[];
    /** The plain phrase under the definition that says what it means. */
    sentence: JSX.Element;
    /** The senses the phrase lists, each linking to the figure that shows it, lit one at a time. */
    points?: readonly Link[];
    /** The sense the phrase ends on as the page's call to action, set apart from the lit ones. */
    action?: Link;
};

/** A labelled link to a section of the page. */
type Link = { label: string; href: string };

/** One dictionary form of the word: its syllables, pronunciation, part of speech and sense. */
export type Form = {
    /** The headword's syllables, set with the dictionary dot between them. */
    syllables: readonly string[];
    /** The pronunciation in IPA. */
    pronunciation: string;
    /** The part of speech, such as verb or noun. */
    partOfSpeech: string;
    /** The sense this section shows. */
    sense: Sense;
};

/** Set a headword's syllables with the dictionary dot between them. */
export function Syllables(properties: { syllables: readonly string[] }) {
    return (
        <>
            {properties.syllables.map((syllable, index) => (
                <>
                    {index > 0 && (
                        <span aria-hidden="true" {...style.attrs(styles.interpunct)}>
                            ·
                        </span>
                    )}
                    {syllable}
                </>
            ))}
        </>
    );
}

/** Head a section with a dictionary entry. */
export function Entry(properties: { form: Form; isTitle?: boolean }) {
    // follow the phrase, the motion it may show, and whether it is on screen
    const [phrase, setPhrase] = createSignal<HTMLElement>();
    const isReduced = createReducedMotion(phrase);
    const isShown = createVisibilityObserver(phrase, { initialValue: true });

    // light the pointed sense, or else turn the light through the senses while the phrase shows
    const [pointed, setPointed] = createSignal<number | undefined>(undefined);
    const [turned, setTurned] = createSignal(0);
    const count = properties.form.sense.points?.length ?? 0;
    createTimer(
        () => setTurned((current) => (current + 1) % count),
        () =>
            count > 1 && !isReduced() && isShown() && pointed() === undefined ? LIGHT_TIME : false,
        setInterval,
    );
    const lit = () => pointed() ?? turned();

    return (
        <header id={anchorOf(properties.form)} {...style.attrs(lattice.cell, styles.entry)}>
            <p {...style.attrs(styles.kicker)}>
                <b {...style.attrs(styles.headword)}>
                    <Syllables syllables={properties.form.syllables} />
                </b>
                <span>{properties.form.pronunciation}</span>
                <i>{properties.form.partOfSpeech}</i>
            </p>
            {properties.isTitle === true ? (
                <h1 {...style.attrs(styles.definition)}>
                    <Definition sense={properties.form.sense} />
                </h1>
            ) : (
                <h2 {...style.attrs(styles.definition)}>
                    <Definition sense={properties.form.sense} />
                </h2>
            )}
            <p ref={setPhrase} {...style.attrs(styles.sentence)}>
                {properties.form.sense.points === undefined ? (
                    properties.form.sense.sentence
                ) : (
                    <>
                        <For each={properties.form.sense.points}>
                            {(point, index) => (
                                <>
                                    {index() > 0 && (
                                        <>
                                            {" "}
                                            <Dot />{" "}
                                        </>
                                    )}
                                    <Point
                                        link={point}
                                        isLit={lit() === index()}
                                        onPoint={(isPointed) =>
                                            setPointed(isPointed ? index() : undefined)
                                        }
                                    />
                                </>
                            )}
                        </For>
                        {properties.form.sense.action === undefined ? undefined : (
                            <>
                                {" "}
                                <Dot /> <Action link={properties.form.sense.action} />
                            </>
                        )}
                    </>
                )}
            </p>
        </header>
    );
}

/** Set one sense of a phrase as a link, set bold while lit, in the room its bold face takes. */
function Point(properties: { link: Link; isLit: boolean; onPoint: (isPointed: boolean) => void }) {
    return (
        <a
            href={properties.link.href}
            onPointerEnter={() => properties.onPoint(true)}
            onPointerLeave={() => properties.onPoint(false)}
            onFocus={() => properties.onPoint(true)}
            onBlur={() => properties.onPoint(false)}
            {...style.attrs(styles.point)}
        >
            <span {...style.attrs(styles.pointText, properties.isLit && styles.pointLit)}>
                {properties.link.label}
            </span>
            <span aria-hidden="true" {...style.attrs(styles.pointRoom)}>
                {properties.link.label}
            </span>
        </a>
    );
}

/** Set the sense a phrase ends on as its call to action, with an arrow down to where it leads. */
function Action(properties: { link: Link }) {
    return (
        <a href={properties.link.href} {...style.attrs(styles.point, styles.action)}>
            {properties.link.label}
            <Glyph name="download" size={18} weight={2} />
        </a>
    );
}

/** Set the dot between the senses of a phrase, as between a headword's syllables. */
function Dot() {
    return (
        <span aria-hidden="true" {...style.attrs(styles.dot)}>
            ·
        </span>
    );
}

/** Return the anchor of a form's section: its part of speech, such as verb or past-participle. */
function anchorOf(form: Form): string {
    return form.partOfSpeech.replaceAll(" ", "-");
}

/** Label the figure under an entry with its number and a usage example, as a dictionary does. */
export function FigureLabel(properties: {
    number: number;
    isOpen: boolean;
    stacked: string;
    destacked: string;
    children?: JSX.Element;
}) {
    // put the figure's section to sleep while it is off screen, so a switch applies there without motion
    let label: HTMLDivElement | undefined;
    const sight = makeIntersectionObserver([], (entries) => {
        for (const entry of entries) {
            entry.target.toggleAttribute("data-asleep", !entry.isIntersecting);
        }
    });
    onSettled(() => {
        // require the rendered label inside its section
        const section = label?.parentElement;
        if (!section) {
            throw new TypeError("the figure label rendered outside a section");
        }
        sight.add(section);
    });

    return (
        <div ref={label} data-figure-label {...style.attrs(lattice.cell, styles.label)}>
            <span {...style.attrs(styles.caption)}>
                <span {...style.attrs(styles.figure)}>Fig. {properties.number}</span>
                <span {...style.attrs(styles.title)}>
                    <Crossfade
                        isOpen={properties.isOpen}
                        stacked={properties.stacked}
                        destacked={properties.destacked}
                    />
                </span>
            </span>
            <span {...style.attrs(styles.labelControl)}>{properties.children}</span>
        </div>
    );
}

/** Set a definition with its highlighted passages underlined. */
function Definition(properties: { sense: Sense }) {
    // cut the definition into plain and underlined runs
    const runs = () => {
        // walk the passages in order
        const result: { text: string; isHighlighted: boolean }[] = [];
        let rest = properties.sense.definition;
        for (const passage of properties.sense.highlight) {
            // find the passage, or refuse a definition that lacks it
            const at = rest.indexOf(passage);
            if (at === -1) {
                throw new Error(`"${passage}" is not in "${properties.sense.definition}"`);
            }
            result.push({ text: rest.slice(0, at), isHighlighted: false });
            result.push({ text: passage, isHighlighted: true });
            rest = rest.slice(at + passage.length);
        }
        result.push({ text: rest, isHighlighted: false });

        return result;
    };

    return (
        <span>
            {runs().map((run) =>
                run.isHighlighted ? (
                    <span {...style.attrs(styles.highlight)}>{run.text}</span>
                ) : (
                    run.text
                ),
            )}
        </span>
    );
}

/** The entry styles. */
const styles = style.create({
    entry: {
        display: "grid",
        alignContent: "start",
        gridColumn: "1 / -1",
        minHeight: { default: frame.entry, [screen.belowDesktop]: "auto" },
        paddingBlockStart: { default: "3rem", [media.maxMd]: "2.5rem" },
        paddingBlockEnd: { default: "2rem", [media.maxMd]: "1.75rem" },
        paddingInline: frame.inset,
        rowGap: "0.75rem",
        scrollMarginTop: frame.bar,
    },
    title: {
        display: "inline-block",
    },
    caption: {
        alignItems: "baseline",
        display: "flex",
    },
    label: {
        alignItems: "baseline",
        display: "flex",
        flexWrap: "wrap",
        fontSize: "0.9375rem",
        fontWeight: 600,
        gap: "1rem",
        gridColumn: "1 / -1",
        margin: 0,
        paddingBlock: "0.875rem",
        paddingInline: frame.inset,
    },
    labelControl: {
        marginLeft: "auto",
    },
    figure: {
        color: color.primary,
        marginRight: "1rem",
        whiteSpace: "nowrap",
        fontFamily: font.code,
        fontSize: "0.6875rem",
        fontWeight: 600,
        letterSpacing: "0.1em",
        textTransform: "uppercase",
    },
    kicker: {
        alignItems: "baseline",
        color: color.mutedForeground,
        display: "flex",
        flexWrap: "wrap",
        fontFamily: font.text,
        fontSize: "0.9375rem",
        rowGap: "0.25rem",
        columnGap: "0.625rem",
        margin: 0,
    },
    headword: {
        color: color.primary,
        fontSize: "1.0625rem",
        fontWeight: 700,
    },
    interpunct: {
        marginInline: "0.05em",
    },
    definition: {
        fontSize: "clamp(1.75rem, 3vw, 2.375rem)",
        fontWeight: 500,
        letterSpacing: "-0.02em",
        lineHeight: 1.15,
        margin: 0,
        textWrap: "balance",
    },
    sentence: {
        color: color.mutedForeground,
        fontSize: "clamp(1rem, 1.3vw, 1.1875rem)",
        lineHeight: 1.45,
        margin: 0,
        maxWidth: "64rem",
        textWrap: "pretty",
    },
    point: {
        color: "inherit",
        display: "inline-grid",
        textDecorationLine: "none",
        whiteSpace: "nowrap",
    },
    pointText: {
        gridArea: "1 / 1",
        textDecorationColor: {
            default: color.border,
            ":hover": { default: null, [media.hover]: color.primary },
        },
        textDecorationLine: "underline",
        textDecorationSkipInk: "none",
        textDecorationThickness: "0.08em",
        textUnderlineOffset: "0.2em",
        transitionDuration: { default: "420ms", [media.motionReduce]: "0ms" },
        transitionProperty: "color, font-weight, text-decoration-color, text-decoration-thickness",
        transitionTimingFunction: "ease",
    },
    pointLit: {
        color: color.foreground,
        fontWeight: 600,
        textDecorationColor: color.primary,
        textDecorationThickness: "0.12em",
    },
    action: {
        alignItems: "center",
        color: {
            default: color.foreground,
            ":hover": { default: null, [media.hover]: color.primary },
        },
        columnGap: "0.3em",
        display: "inline-flex",
        textDecorationColor: {
            default: color.border,
            ":hover": { default: null, [media.hover]: color.primary },
        },
        textDecorationLine: "underline",
        textDecorationSkipInk: "none",
        textDecorationThickness: "0.08em",
        textUnderlineOffset: "0.2em",
    },
    pointRoom: {
        fontWeight: 600,
        gridArea: "1 / 1",
        visibility: "hidden",
    },
    dot: {
        color: color.primary,
        fontWeight: 700,
        marginInline: "0.3em",
    },
    highlight: {
        textDecorationColor: color.primary,
        textDecorationLine: "underline",
        textDecorationSkipInk: "none",
        textDecorationThickness: "0.12em",
        textUnderlineOffset: "0.14em",
    },
});
