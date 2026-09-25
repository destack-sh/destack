import { color } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";

import { lattice } from "../style/lattice.stylex";
import { tokens } from "../style/tokens.stylex";
import { stillStars } from "../effect/goo";
import { Install } from "./install";
import { Plate } from "./plate";

/** The media query for screens narrower than the desktop frame. */
const narrow = "@media (max-width: 1099px)";
/** The media query for tablet-width screens. */
const tablet = "@media (min-width: 768px) and (max-width: 1099px)";
/** The media query for phone-width screens. */
const mobile = "@media (max-width: 767px)";

/** The promise's width in ems, so its size can be set to span the wordmark exactly. */
const promiseMeasure = 23.66;

/** The promise, word by word, so each word can turn with the universe on its own. */
const promise = "to unify all your apps and agents with one open stack".split(" ");

/** The wordmark's syllables, set apart by a dictionary dot. */
const syllables = ["DE", "STACK"];

/** Introduce Destack as a poster: the wordmark and promise on the left, the plate with the download on the right. */
export function Hero() {
    return (
        <section {...stylex.attrs(lattice.frame, lattice.ruleBottom, styles.hero)}>
            {/* spread the wordmark across its cell, letter by letter */}
            <h1
                data-universe="parts"
                aria-label="Destack"
                {...stylex.attrs(lattice.ruleRight, lattice.ruleBottom, styles.wordmark)}
            >
                {/* hang the dictionary dot on the end of the first syllable */}
                {[...syllables[0]].map((letter, index) => (
                    <span aria-hidden="true">
                        <span
                            style={{ "background-image": stillStars }}
                            {...stylex.attrs(styles.letter)}
                        >
                            {letter}
                        </span>
                        {index === syllables[0].length - 1 && (
                            <span {...stylex.attrs(styles.dot)} />
                        )}
                    </span>
                ))}
                {[...syllables[1]].map((letter) => (
                    <span
                        aria-hidden="true"
                        style={{ "background-image": stillStars }}
                        {...stylex.attrs(styles.letter)}
                    >
                        {letter}
                    </span>
                ))}
            </h1>

            {/* define the word, then make the promise */}
            <div data-universe {...stylex.attrs(lattice.ruleRight, styles.promise)}>
                <span {...stylex.attrs(styles.kicker)}>
                    <b {...stylex.attrs(styles.headword)}>de·stack</b>
                    <span {...stylex.attrs(styles.pronunciation)}>/diːˈstak/</span>
                    <i {...stylex.attrs(styles.partOfSpeech)}>verb</i>
                </span>
                <p data-universe="parts" {...stylex.attrs(styles.line)}>
                    {promise.map((word, index) => (
                        <>
                            {index > 0 && " "}
                            <span data-word={word} {...stylex.attrs(word === "one" && styles.one)}>
                                {word}
                            </span>
                        </>
                    ))}
                </p>
            </div>
            <Plate style={styles.plate}>
                <Install />
            </Plate>
        </section>
    );
}

/** The slow drift of the starry space inside the wordmark's letters. */
const drift = stylex.keyframes({
    from: { backgroundPosition: "0 0" },
    to: { backgroundPosition: "240px 120px" },
});

/** The hero styles. */
const styles = stylex.create({
    hero: {
        gridTemplateRows: `calc(${tokens.row} * 1.7) calc(${tokens.row} * 1.3)`,
        [narrow]: { gridTemplateRows: "none" },
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
        [narrow]: {
            borderRightWidth: 0,
            gridColumn: "1 / -1",
            gridRow: "auto",
            whiteSpace: "nowrap",
        },
        [tablet]: {
            fontSize: `min(calc(${tokens.siteWidth} * 0.15), 7.5rem)`,
            paddingBlock: "2rem 1rem",
        },
        [mobile]: {
            fontSize: "min(13vw, 5.5rem)",
            paddingBlock: "2.5rem 1.5rem",
        },
    },
    promise: {
        display: "flex",
        flexDirection: "column",
        gap: "0.25rem",
        gridColumn: "1 / span 8",
        gridRow: 2,
        justifyContent: "center",
        paddingBottom: "1.125rem",
        paddingInline: tokens.inset,
        [narrow]: {
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
        animationDuration: "90s",
        animationIterationCount: "infinite",
        animationName: drift,
        animationTimingFunction: "linear",
        backgroundAttachment: "fixed",
        backgroundClip: "text",
        backgroundColor: tokens.space,
        color: "transparent",
        display: "inline-block",
        transform: "scaleX(1.2)",
        WebkitBackgroundClip: "text",
        WebkitTextStroke: `1.5px ${tokens.cream}`,
        "@media (prefers-reduced-motion: reduce)": { animationName: "none" },
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
        [tablet]: {
            fontSize: `calc((${tokens.siteWidth} - ${tokens.inset} * 2) / ${promiseMeasure})`,
        },
        [mobile]: { fontSize: "clamp(1.125rem, 6vw, 1.75rem)", whiteSpace: "normal" },
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
        [narrow]: { gridColumn: "1 / -1", gridRow: "auto" },
        [tablet]: { height: "9rem" },
        [mobile]: { aspectRatio: "16 / 11" },
    },
});
