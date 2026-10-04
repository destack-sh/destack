import { color, text } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";

import { tokens } from "../style/tokens.stylex";

/** One sense of a form: its definition, the passages underlined in it, and the sentence under it. */
export type Sense = {
    /** The definition. */
    definition: string;
    /** The passages of the definition drawn underlined, in order: whole words, phrases or parts of words. */
    highlight: readonly string[];
    /** The plain sentence that says what the definition means. */
    sentence: string;
};

/** One dictionary form of the word: its syllables, pronunciation, part of speech and senses. */
export type Form = {
    /** The headword's syllables, set with the dictionary dot between them. */
    syllables: readonly string[];
    /** The pronunciation in IPA. */
    pronunciation: string;
    /** The part of speech, such as verb or noun. */
    partOfSpeech: string;
    /** The senses, numbered when there are several. */
    senses: readonly Sense[];
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

/** Head a section with a dictionary entry: the headword line, then each sense with its sentence under it. */
export function Entry(properties: { form: Form; isTitle?: boolean }) {
    const isNumbered = () => properties.form.senses.length > 1;
    const senses = () => (
        <span {...stylex.attrs(styles.senses)}>
            {properties.form.senses.map((sense, index) => (
                <span {...stylex.attrs(styles.sense, isNumbered() && styles.numbered)}>
                    {isNumbered() && <span {...stylex.attrs(styles.number)}>{index + 1}</span>}
                    <Definition sense={sense} />
                    <span {...stylex.attrs(styles.sentence)}>{sense.sentence}</span>
                </span>
            ))}
        </span>
    );

    return (
        <header {...stylex.attrs(styles.entry)}>
            <p {...stylex.attrs(styles.kicker)}>
                <b {...stylex.attrs(styles.headword)}>
                    <Syllables syllables={properties.form.syllables} />
                </b>
                <span>{properties.form.pronunciation}</span>
                <i>{properties.form.partOfSpeech}</i>
            </p>
            {properties.isTitle === true ? (
                <h1 {...stylex.attrs(styles.statement)}>{senses()}</h1>
            ) : (
                <h2 {...stylex.attrs(styles.statement)}>{senses()}</h2>
            )}
        </header>
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
        <>
            {runs().map((run) =>
                run.isHighlighted ? (
                    <span {...stylex.attrs(styles.highlight)}>{run.text}</span>
                ) : (
                    run.text
                ),
            )}
        </>
    );
}

/** The entry styles. */
const styles = stylex.create({
    entry: {
        gridColumn: "1 / -1",
        minWidth: 0,
        paddingBottom: "1.5rem",
        paddingInline: tokens.inset,
        paddingTop: "4rem",
        "@media (max-width: 767px)": { paddingBottom: "1.75rem", paddingTop: "2.75rem" },
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
        marginBottom: "1rem",
    },
    headword: {
        color: tokens.signal,
        fontSize: "1.0625rem",
        fontWeight: 700,
    },
    interpunct: {
        marginInline: "0.05em",
    },
    statement: {
        fontSize: "clamp(1.5rem, 2.2vw, 1.875rem)",
        fontWeight: 500,
        letterSpacing: "-0.015em",
        lineHeight: 1.22,
        margin: 0,
        maxWidth: "56rem",
        textWrap: "pretty",
    },
    senses: {
        display: "grid",
        rowGap: "0.9em",
    },
    sense: {
        display: "block",
    },
    numbered: {
        paddingLeft: "1.5em",
        position: "relative",
    },
    number: {
        color: tokens.signal,
        fontWeight: 700,
        left: 0,
        position: "absolute",
    },
    sentence: {
        color: color.mutedForeground,
        display: "block",
        fontSize: "0.75em",
        fontWeight: 400,
        lineHeight: 1.3,
        marginTop: "0.4em",
        textWrap: "balance",
    },
    highlight: {
        textDecorationColor: tokens.signal,
        textDecorationLine: "underline",
        textDecorationSkipInk: "none",
        textDecorationThickness: "0.12em",
        textUnderlineOffset: "0.14em",
    },
});
