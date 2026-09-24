import { color } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";

import { lattice } from "../style/lattice.stylex";
import { tokens } from "../style/tokens.stylex";
import { Plate } from "./plate";

/** The media query for phone-width screens. */
const mobile = "@media (max-width: 767px)";

/** The promise's width in ems, so its size can be set to span the wordmark exactly. */
const promiseMeasure = 23.66;

/** The wordmark's syllables, set apart by a dictionary dot. */
const syllables = ["DE", "STACK"];

/** Introduce Destack as a poster: the wordmark and promise on the left, the plate on the right. */
export function Hero() {
    return (
        <section {...stylex.attrs(lattice.frame, lattice.ruleBottom, styles.hero)}>
            {/* spread the wordmark across its cell, letter by letter */}
            <h1
                aria-label="Destack"
                {...stylex.attrs(lattice.ruleRight, lattice.ruleBottom, styles.wordmark)}
            >
                {/* hang the dictionary dot on the end of the first syllable */}
                {[...syllables[0]].map((letter, index) => (
                    <span aria-hidden="true">
                        <span {...stylex.attrs(styles.letter)}>{letter}</span>
                        {index === syllables[0].length - 1 && (
                            <span {...stylex.attrs(styles.dot)} />
                        )}
                    </span>
                ))}
                {[...syllables[1]].map((letter) => (
                    <span aria-hidden="true" {...stylex.attrs(styles.letter)}>
                        {letter}
                    </span>
                ))}
            </h1>

            {/* define the word, then make the promise */}
            <div {...stylex.attrs(lattice.ruleRight, styles.promise)}>
                <span {...stylex.attrs(styles.kicker)}>
                    <b {...stylex.attrs(styles.headword)}>de·stack</b>
                    <span {...stylex.attrs(styles.pronunciation)}>/diːˈstak/</span>
                    <i {...stylex.attrs(styles.partOfSpeech)}>verb</i>
                </span>
                <p {...stylex.attrs(styles.line)}>
                    <span {...stylex.attrs(styles.to)}>to</span> unify all your apps and agents with{" "}
                    <span {...stylex.attrs(styles.one)}>one</span> open stack
                </p>
            </div>
            <Plate style={styles.plate} />
        </section>
    );
}

/** The hero styles. */
const styles = stylex.create({
    hero: {
        gridTemplateRows: `calc(${tokens.row} * 1.7) calc(${tokens.row} * 1.3)`,
        [mobile]: { gridTemplateRows: "none" },
    },
    wordmark: {
        alignItems: "center",
        color: color.foreground,
        display: "flex",
        fontFamily: tokens.posterFont,
        fontSize: `min(calc(${tokens.column} * 1.6), calc(${tokens.row} * 1.62))`,
        fontWeight: 400,
        gridColumn: "1 / span 8",
        justifyContent: "space-between",
        gridRow: 1,
        lineHeight: 1,
        margin: 0,
        paddingBlock: 0,
        paddingInline: tokens.inset,
        paddingTop: "0.12em",
        [mobile]: {
            borderRightWidth: 0,
            fontSize: "min(13vw, 5.5rem)",
            gridColumn: "1 / -1",
            gridRow: "auto",
            paddingBlock: "2.5rem 1.5rem",
            whiteSpace: "nowrap",
        },
    },
    promise: {
        display: "flex",
        flexDirection: "column",
        gap: "0.25rem",
        gridColumn: "1 / span 8",
        gridRow: 2,
        justifyContent: "center",
        paddingInline: tokens.inset,
        [mobile]: {
            borderBottomColor: color.border,
            borderBottomStyle: "solid",
            borderBottomWidth: tokens.hairline,
            borderRightWidth: 0,
            gridColumn: "1 / -1",
            gridRow: "auto",
            paddingBlock: "1.25rem",
        },
    },
    letter: {
        display: "inline-block",
        transform: "scaleX(1.2)",
    },
    dot: {
        backgroundColor: tokens.signal,
        borderRadius: "50%",
        display: "inline-block",
        height: "0.17em",
        marginLeft: "0.07em",
        verticalAlign: "0.3em",
        width: "0.17em",
    },
    kicker: {
        alignItems: "baseline",
        display: "flex",
        fontSize: "0.9375rem",
        gap: "0.625rem",
    },
    headword: {
        color: tokens.signal,
        fontWeight: 700,
    },
    pronunciation: {
        color: color.mutedForeground,
        fontFamily: tokens.monoFont,
        fontSize: "0.8125rem",
    },
    partOfSpeech: {
        color: color.mutedForeground,
        fontStyle: "italic",
    },
    line: {
        fontSize: `calc((${tokens.column} * 8 - ${tokens.inset} * 2) / ${promiseMeasure})`,
        fontWeight: 500,
        letterSpacing: "-0.01em",
        lineHeight: 1.2,
        margin: 0,
        whiteSpace: "nowrap",
        [mobile]: { fontSize: "clamp(1.125rem, 6vw, 1.75rem)", whiteSpace: "normal" },
    },
    to: {
        color: color.mutedForeground,
    },
    one: {
        textDecorationColor: tokens.signal,
        textDecorationLine: "underline",
        textDecorationThickness: "3px",
        textUnderlineOffset: "0.18em",
    },
    plate: {
        gridColumn: "9 / span 4",
        gridRow: "1 / span 2",
        [mobile]: { aspectRatio: "3 / 2", gridColumn: "1 / -1", gridRow: "auto" },
    },
});
