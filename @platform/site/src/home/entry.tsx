import { color, text } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";
import type { JSX } from "@destack/view";

import { lattice } from "../style/lattice.stylex";
import { tokens } from "../style/tokens.stylex";

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
                        <span aria-hidden="true" {...stylex.attrs(styles.interpunct)}>
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
        <header {...stylex.attrs(lattice.cell, styles.entry)}>
            <p {...stylex.attrs(styles.kicker)}>
                <b {...stylex.attrs(styles.headword)}>
                    <Syllables syllables={properties.form.syllables} />
                </b>
                <span>{properties.form.pronunciation}</span>
                <i>{properties.form.partOfSpeech}</i>
            </p>
            {properties.isTitle === true ? (
                <h1 {...stylex.attrs(styles.definition)}>
                    <Definition sense={properties.form.sense} />
                </h1>
            ) : (
                <h2 {...stylex.attrs(styles.definition)}>
                    <Definition sense={properties.form.sense} />
                </h2>
            )}
            <p {...stylex.attrs(styles.sentence)}>{properties.form.sense.sentence}</p>
        </header>
    );
}

/** Label the figure under an entry with its number and title, as a reference book does. */
export function FigureLabel(properties: { number: number; title: string; children?: JSX.Element }) {
    return (
        <div {...stylex.attrs(lattice.cell, styles.label)}>
            <span>
                <span {...stylex.attrs(styles.figure)}>Fig. {properties.number}</span>
                {properties.title}
            </span>
            <span {...stylex.attrs(styles.labelControl)}>{properties.children}</span>
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
                    <span {...stylex.attrs(styles.highlight)}>{run.text}</span>
                ) : (
                    run.text
                ),
            )}
        </span>
    );
}

/** The entry styles. */
const styles = stylex.create({
    entry: {
        display: "grid",
        alignContent: "start",
        gridColumn: "1 / -1",
        minHeight: { default: tokens.entry, "@media (max-width: 1099px)": "auto" },
        paddingBlock: "3rem 2rem",
        paddingInline: tokens.inset,
        rowGap: "0.75rem",
        "@media (max-width: 767px)": { paddingBlock: "2.5rem 1.75rem" },
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
        paddingInline: tokens.inset,
    },
    labelControl: {
        marginLeft: "auto",
    },
    figure: {
        color: tokens.signal,
        marginRight: "1rem",
        whiteSpace: "nowrap",
        fontFamily: tokens.monoFont,
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
        fontFamily: text.family,
        fontSize: "0.9375rem",
        gap: "0.25rem 0.625rem",
        margin: 0,
    },
    headword: {
        color: tokens.signal,
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
        textDecorationColor: tokens.signal,
        textDecorationLine: "underline",
        textDecorationSkipInk: "none",
        textDecorationThickness: "0.12em",
        textUnderlineOffset: "0.14em",
    },
});
