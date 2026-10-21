import { frame } from "../../layout/frame.stylex";
import { color, font } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import { Crossfade } from "../figure/crossfade";
import { type JSX, onSettled } from "@destack/view";
import { makeIntersectionObserver } from "@destack/view/primitives/intersection-observer";

import { lattice } from "../../layout/lattice.stylex";
import { media } from "@destack/style/media.stylex";
import { screen } from "../../layout/screen.stylex";
import { typeScale, tracking } from "../../layout/type.stylex";

/** One sense of a form: its definition, the passages underlined in it, and the phrase under it. */
type Sense = {
    /** The definition. */
    definition: string;
    /** The passages of the definition drawn underlined, in order. */
    highlight: readonly string[];
    /** The plain phrase under the definition that says what it means. */
    sentence: JSX.Element;
    /** A short note in the form's own part of speech, set in brackets after it. */
    gloss?: string;
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

/** Head a section with a dictionary entry. */
export function Entry(properties: { form: Form; isTitle?: boolean }) {
    return (
        <header id={anchorOf(properties.form)} {...style.attrs(lattice.cell, styles.entry)}>
            <p {...style.attrs(styles.kicker)}>
                <b {...style.attrs(styles.headword)}>
                    <Syllables syllables={properties.form.syllables} />
                </b>
                <span>{properties.form.pronunciation}</span>
                <i>{properties.form.partOfSpeech}</i>
                {properties.form.sense.gloss === undefined ? undefined : (
                    <span>[{properties.form.sense.gloss}]</span>
                )}
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
            <p {...style.attrs(styles.sentence)}>{properties.form.sense.sentence}</p>
        </header>
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
    figure: {
        color: color.primary,
        marginRight: "1rem",
        whiteSpace: "nowrap",
        fontFamily: font.code,
        fontSize: typeScale.label,
        fontWeight: 600,
        letterSpacing: tracking.label,
        textTransform: "uppercase",
    },
    kicker: {
        alignItems: "baseline",
        color: color.mutedForeground,
        display: "flex",
        flexWrap: "wrap",
        fontFamily: font.text,
        fontSize: typeScale.body,
        rowGap: "0.25rem",
        columnGap: "0.625rem",
        margin: 0,
    },
    headword: {
        color: color.primary,
        fontSize: typeScale.body,
        fontWeight: 600,
    },
    interpunct: {
        marginInline: "0.05em",
    },
    definition: {
        fontSize: typeScale.display,
        fontWeight: 500,
        letterSpacing: tracking.display,
        lineHeight: 1.15,
        margin: 0,
        textWrap: "balance",
    },
    sentence: {
        color: color.mutedForeground,
        fontSize: typeScale.lead,
        lineHeight: 1.45,
        margin: 0,
        maxWidth: "64rem",
        textWrap: "pretty",
    },
    highlight: {
        textDecorationColor: color.primary,
        textDecorationLine: "underline",
        textDecorationSkipInk: "none",
        textDecorationThickness: "0.12em",
        textUnderlineOffset: "0.14em",
    },
});
