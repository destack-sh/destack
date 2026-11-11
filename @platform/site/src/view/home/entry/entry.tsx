import { frame } from "../../layout/frame.stylex";
import { color, font, motion } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import { Crossfade } from "../figure/crossfade";
import { createSignal, type JSX, onSettled } from "@destack/view";
import { makeIntersectionObserver } from "@destack/view/primitives/intersection-observer";

import { lattice } from "../../layout/lattice.stylex";
import { media } from "@destack/style/media.stylex";
import { screen } from "../../layout/screen.stylex";
import { typeScale, tracking } from "../../layout/type.stylex";

/** How far beyond the screen a figure starts rendering: one screen, a quick flick's travel. */
const NEAR_MARGIN = "100% 0px";

/** A page whose scripts run, which draws each underline in once its entry shows. */
const SCRIPTED = "@media (scripting: enabled)";

/** The bottom band of the screen an entry rises past before its underline draws. */
const SEEN_MARGIN = "0px 0px -20% 0px";

/** One sense of a form: its definition, the passages underlined in it, and the phrase under it. */
type Sense = {
    /** The definition. */
    definition: string;
    /** The passages of the definition drawn underlined, in order. */
    highlight: readonly string[];
    /** The plain phrase under the definition that says what it means. */
    sentence: JSX.Element;
};

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

/** Set a part of speech as a bracketed tag in small capitals, such as [ VERB ]. */
export function PartOfSpeech(properties: { children: string }) {
    return (
        <span data-part-of-speech={properties.children} {...style.attrs(styles.partOfSpeech)}>
            [ {properties.children} ]
        </span>
    );
}

/** Head a section with a dictionary entry. */
export function Entry(properties: { form: Form; isTitle?: boolean }) {
    // draw the definition's underline once, the first time the entry rises past the screen's bottom fifth
    let header: HTMLElement | undefined;
    const [isSeen, setIsSeen] = createSignal(false);
    const sight = makeIntersectionObserver(
        [],
        (entries) => {
            if (entries.some((entry) => entry.isIntersecting)) {
                setIsSeen(true);
                sight.stop();
            }
        },
        { rootMargin: SEEN_MARGIN },
    );
    onSettled(() => {
        // require the rendered header
        if (!header) {
            throw new TypeError("the entry rendered without its header");
        }
        sight.add(header);
    });

    return (
        <header
            ref={header}
            id={anchorOf(properties.form)}
            {...style.attrs(lattice.cell, styles.entry)}
        >
            {/* the rule between the word and its meaning, from the section's top to its bottom */}
            <span aria-hidden="true" {...style.attrs(styles.rule)} />

            {/* the word: its headword at the definition's size, its pronunciation under it */}
            <p {...style.attrs(styles.word)}>
                <b {...style.attrs(styles.headword)}>
                    <Syllables syllables={properties.form.syllables} />
                </b>
                <span {...style.attrs(styles.meta)}>{properties.form.pronunciation}</span>
            </p>

            {/* the meaning: its definition and the sentence under it */}
            {properties.isTitle === true ? (
                <h1 {...style.attrs(styles.definition)}>
                    <Definition sense={properties.form.sense} isDrawn={isSeen()} />
                </h1>
            ) : (
                <h2 {...style.attrs(styles.definition)}>
                    <Definition sense={properties.form.sense} isDrawn={isSeen()} />
                </h2>
            )}
            <p {...style.attrs(styles.sentence)}>{properties.form.sense.sentence}</p>
        </header>
    );
}

/** Return the anchor of a form's section: its part of speech, such as verb or past-participle. */
export function anchorOf(form: Form): string {
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

    // render the figure's cells from a screen away, before they scroll in, since Safari renders them only once in view
    const nearness = makeIntersectionObserver(
        [],
        (entries) => {
            for (const entry of entries) {
                entry.target.toggleAttribute("data-near", entry.isIntersecting);
            }
        },
        { rootMargin: NEAR_MARGIN },
    );
    onSettled(() => {
        // require the rendered label inside its section
        const section = label?.parentElement;
        if (!section) {
            throw new TypeError("the figure label rendered outside a section");
        }
        sight.add(section);
        nearness.add(section);
    });

    return (
        <div ref={label} data-figure-label {...style.attrs(lattice.cell, styles.label)}>
            <span {...style.attrs(styles.caption)}>
                {/* number the figure in a balloon, its leader running to the title */}
                <span {...style.attrs(styles.balloon)}>
                    <span {...style.attrs(styles.hidden)}>Fig. </span>
                    {properties.number}
                </span>
                <span aria-hidden="true" {...style.attrs(styles.leader)} />
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

/** Set a definition with its highlighted passages underlined, drawn in from the left once shown. */
function Definition(properties: { sense: Sense; isDrawn: boolean }) {
    // cut the definition into plain and underlined runs
    const runs = () => {
        // walk the passages in order
        const result: { text: string; isHighlighted: boolean }[] = [];
        let rest = properties.sense.definition;
        for (const passage of properties.sense.highlight) {
            // find the passage, or refuse a definition that lacks it
            const at = rest.indexOf(passage);
            if (at === -1) {
                throw new Error(`'${passage}' is not in '${properties.sense.definition}'`);
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
                    <span
                        {...style.attrs(
                            styles.highlight,
                            properties.isDrawn && styles.highlightDrawn,
                        )}
                    >
                        {run.text}
                    </span>
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
        paddingBlockStart: { default: "2.75rem", [media.maxMd]: "2.25rem" },
        paddingBlockEnd: { default: "1.75rem", [media.maxMd]: "1.5rem" },
        paddingInline: frame.inset,
        position: "relative",
        alignItems: "baseline",
        columnGap: { default: "2rem", [screen.belowDesktop]: "0.75rem" },
        gridTemplateAreas: {
            default: `"word title" "word sentence"`,
            [screen.belowDesktop]: `"word" "title" "sentence"`,
        },
        gridTemplateColumns: {
            default: `${frame.word} minmax(0, 1fr)`,
            [screen.belowDesktop]: "minmax(0, 1fr)",
        },
        rowGap: { default: "0.875rem", [screen.belowDesktop]: "0.5rem" },
        scrollMarginTop: frame.bar,
    },
    title: {
        display: "inline-block",
    },
    caption: {
        alignItems: "center",
        display: "flex",
    },
    label: {
        alignItems: "baseline",
        display: "flex",
        flexWrap: "wrap",
        fontSize: typeScale.body,
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
    balloon: {
        alignItems: "center",
        borderColor: color.primary,
        borderRadius: "50%",
        borderStyle: "solid",
        borderWidth: "1.5px",
        color: color.primary,
        display: "inline-flex",
        flexShrink: 0,
        fontFamily: font.code,
        fontSize: typeScale.label,
        fontWeight: 600,
        height: "1.5rem",
        justifyContent: "center",
        width: "1.5rem",
    },
    leader: {
        backgroundColor: color.primary,
        flexShrink: 0,
        height: "1px",
        marginInlineEnd: "0.75rem",
        marginInlineStart: "0.375rem",
        width: "1.25rem",
    },
    hidden: {
        clipPath: "inset(50%)",
        height: "1px",
        overflow: "hidden",
        position: "absolute",
        whiteSpace: "nowrap",
        width: "1px",
    },
    rule: {
        backgroundImage: `repeating-linear-gradient(to bottom, ${color.border} 0 4px, transparent 0 8px)`,
        bottom: 0,
        display: { default: "block", [screen.belowDesktop]: "none" },
        insetInlineStart: `calc(${frame.inset} + ${frame.word} + 1rem)`,
        position: "absolute",
        top: 0,
        width: "1px",
    },
    word: {
        alignItems: "baseline",
        columnGap: "0.75rem",
        display: { default: "grid", [screen.belowDesktop]: "flex" },
        flexWrap: "wrap",
        gridArea: "word",
        justifyItems: "start",
        margin: 0,
        rowGap: "0.5rem",
    },
    meta: {
        color: color.mutedForeground,
        fontSize: { default: typeScale.lead, [screen.belowDesktop]: typeScale.body },
        lineHeight: 1.45,
    },
    partOfSpeech: {
        fontFamily: font.code,
        fontSize: typeScale.label,
        letterSpacing: tracking.label,
        textTransform: "uppercase",
        whiteSpace: "nowrap",
    },
    headword: {
        color: color.primary,
        fontSize: { default: typeScale.display, [screen.belowDesktop]: typeScale.headword },
        fontWeight: { default: 500, [screen.belowDesktop]: 600 },
        letterSpacing: tracking.display,
        lineHeight: 1.15,
        whiteSpace: "nowrap",
    },
    interpunct: {
        marginInline: "0.05em",
    },
    definition: {
        fontSize: typeScale.display,
        fontWeight: 500,
        letterSpacing: tracking.display,
        gridArea: "title",
        lineHeight: 1.15,
        margin: 0,
        textWrap: "balance",
    },
    sentence: {
        color: color.mutedForeground,
        fontSize: typeScale.lead,
        gridArea: "sentence",
        lineHeight: 1.45,
        margin: 0,
        maxWidth: "64rem",
        textWrap: "pretty",
    },
    highlight: {
        backgroundImage: `linear-gradient(${color.primary}, ${color.primary})`,
        backgroundPosition: "left 0 bottom 0.04em",
        backgroundRepeat: "no-repeat",
        backgroundSize: { default: "100% 0.12em", [SCRIPTED]: "0% 0.12em" },
        transitionDuration: motion.durationLong,
        transitionProperty: "background-size",
        transitionTimingFunction: motion.easingEmphasised,
    },
    highlightDrawn: {
        backgroundSize: "100% 0.12em",
    },
});
