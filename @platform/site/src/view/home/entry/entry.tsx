import { frame } from "../../layout/frame.stylex";
import { color, font } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import type { JSX } from "@destack/view";

import { lattice } from "../../layout/lattice.stylex";
import { Fade } from "../figure/fade";

/** One sense of a form: its definition, the passages underlined in it, and the phrase under it. */
type Sense = {
    /** The definition. */
    definition: string;
    /** The passages of the definition drawn underlined, in order: whole words, phrases or parts of words. */
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

/** Head a section with a dictionary entry: the headword line, the definition, and the phrase under it. */
export function Entry(properties: { form: Form; isTitle?: boolean }) {
    return (
        <header {...style.attrs(lattice.cell, styles.entry)}>
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
            <p {...style.attrs(styles.sentence)}>{properties.form.sense.sentence}</p>
        </header>
    );
}

/** Label the figure under an entry with its number and a usage example of the word, as a dictionary does. */
export function FigureLabel(properties: { number: number; title: string; children?: JSX.Element }) {
    return (
        <div {...style.attrs(lattice.cell, styles.label)}>
            <span {...style.attrs(styles.caption)}>
                <span {...style.attrs(styles.figure)}>Fig. {properties.number}</span>
                <Fade isOpen={false} turn={properties.title} xstyle={styles.title}>
                    {properties.title}
                </Fade>
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
        minHeight: { default: frame.entry, "@media (max-width: 1099px)": "auto" },
        paddingBlockStart: "3rem",
        paddingBlockEnd: "2rem",
        paddingInline: frame.inset,
        rowGap: "0.75rem",
        "@media (max-width: 767px)": { paddingBlockStart: "2.5rem", paddingBlockEnd: "1.75rem" },
    },
    title: {
        display: "inline-block",
    },
    caption: {
        alignItems: "baseline",
        display: "flex",
    },
    label: {
        alignItems: "center",
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
    highlight: {
        textDecorationColor: color.primary,
        textDecorationLine: "underline",
        textDecorationSkipInk: "none",
        textDecorationThickness: "0.12em",
        textUnderlineOffset: "0.14em",
    },
});
