import { color, fontFamily } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";

import { lattice } from "../style/lattice.stylex";
import { tokens } from "../style/tokens.stylex";

/** The media query for screens narrower than the desktop frame, where the entries stack. */
const narrow = "@media (max-width: 1099px)";

/** The longest phrase's width in ems, so all three phrases share one size that fits four columns. */
const phraseMeasure = 15.4;
/** The longest sense's width in ems, so every sense fits its column on one line. */
const sensesMeasure = 23.5;

/** One sense of an entry: the words before its key words, the key words, and the words after. */
type Sense = readonly [before: string, key: string, after: string];

/** The word's three entries: what you do, what it is, and what you end up with. */
const entries: readonly {
    headword: readonly [string, string];
    pronunciation: string;
    partOfSpeech: string;
    phrase: string;
    highlight: string;
    senses: readonly Sense[];
}[] = [
    {
        headword: ["de", "stack"],
        pronunciation: "/diːˈstak/",
        partOfSpeech: "verb",
        phrase: "to unify all your apps and agents",
        highlight: "unify",
        senses: [
            ["to", "remix your SaaS", "into software you own"],
            ["to", "ship the app you need", "this afternoon"],
            ["to", "put agents to work", "on all your apps"],
        ],
    },
    {
        headword: ["De", "stack"],
        pronunciation: "/ˈdiːstak/",
        partOfSpeech: "noun",
        phrase: "a standardised software stack",
        highlight: "standardised",
        senses: [
            ["a", "batteries-included", ", web-first stack"],
            ["a", "multiplayer backend", "on SQL, OTEL and S3"],
            ["a personal", "Git, npm and App Store", ""],
        ],
    },
    {
        headword: ["de", "stacked"],
        pronunciation: "/diːˈstakt/",
        partOfSpeech: "adjective",
        phrase: "open, sovereign and portable",
        highlight: "sovereign",
        senses: [
            ["", "open source", ", open standards, open data"],
            ["", "observable and permissioned", ", bits to pixels"],
            ["", "portable", "from your laptop to the cloud"],
        ],
    },
];

/** Introduce Destack as one word in three parts of speech: verb, noun and adjective. */
export function Hero() {
    return (
        <section {...stylex.attrs(lattice.frame, lattice.ruleBottom, styles.hero)}>
            {entries.map((entry, index) => (
                <div
                    data-universe
                    data-entry={entry.partOfSpeech}
                    {...stylex.attrs(styles.entry, index < entries.length - 1 && lattice.ruleRight)}
                >
                    {/* set the headword, its pronunciation and its part of speech; the verb titles the page */}
                    <div {...stylex.attrs(styles.kicker)}>
                        {index === 0 ? (
                            <h1 aria-label="Destack" {...stylex.attrs(styles.headword)}>
                                <Headword syllables={entry.headword} />
                            </h1>
                        ) : (
                            <b {...stylex.attrs(styles.headword)}>
                                <Headword syllables={entry.headword} />
                            </b>
                        )}
                        <span {...stylex.attrs(styles.pronunciation)}>{entry.pronunciation}</span>
                        <i {...stylex.attrs(styles.partOfSpeech)}>{entry.partOfSpeech}</i>
                    </div>

                    {/* state the meaning, then number the senses, each with its key words highlighted */}
                    <p data-universe="parts" {...stylex.attrs(styles.phrase)}>
                        {entry.phrase.split(" ").map((word, wordIndex) => (
                            <>
                                {wordIndex > 0 && " "}
                                <span
                                    data-word={word}
                                    {...stylex.attrs(word === entry.highlight && styles.highlight)}
                                >
                                    {word}
                                </span>
                            </>
                        ))}
                    </p>
                    <ol {...stylex.attrs(styles.senses)}>
                        {entry.senses.map(([before, key, after], senseIndex) => (
                            <li {...stylex.attrs(styles.sense)}>
                                <span {...stylex.attrs(styles.number)}>{senseIndex + 1}</span>
                                <span>
                                    {before === "" ? "" : `${before} `}
                                    <b {...stylex.attrs(styles.key)}>{key}</b>
                                    {after === "" || after.startsWith(",") ? after : ` ${after}`}
                                </span>
                            </li>
                        ))}
                    </ol>
                </div>
            ))}
        </section>
    );
}

/** Set a headword's syllables with the dictionary dot between them. */
function Headword(properties: { syllables: readonly string[] }) {
    return (
        <>
            {properties.syllables[0]}
            <span aria-hidden="true" {...stylex.attrs(styles.interpunct)}>
                ·
            </span>
            {properties.syllables[1]}
        </>
    );
}

/** The hero styles. */
const styles = stylex.create({
    hero: {
        gridTemplateRows: tokens.hero,
        [narrow]: { gridTemplateRows: "auto" },
    },
    entry: {
        display: "flex",
        flexDirection: "column",
        gap: "0.625rem",
        gridColumn: "span 4",
        justifyContent: "center",
        minWidth: 0,
        paddingInline: tokens.inset,
        paddingTop: "0.75rem",
        [narrow]: {
            borderBottomColor: color.border,
            borderBottomStyle: "solid",
            borderBottomWidth: tokens.hairline,
            borderRightWidth: 0,
            gridColumn: "1 / -1",
            paddingBlock: "1.5rem",
        },
    },
    kicker: {
        alignItems: "baseline",
        display: "flex",
        gap: "0.625rem",
        height: "1.5rem",
        whiteSpace: "nowrap",
    },
    headword: {
        color: tokens.signal,
        fontSize: "1.0625rem",
        fontWeight: 700,
        margin: 0,
    },
    interpunct: {
        marginInline: "0.05em",
    },
    pronunciation: {
        color: color.mutedForeground,
        fontFamily: fontFamily.default,
        fontSize: "0.9375rem",
    },
    partOfSpeech: {
        color: color.mutedForeground,
        fontSize: "0.9375rem",
        fontStyle: "italic",
    },
    phrase: {
        fontSize: `calc((${tokens.column} * 4 - ${tokens.inset} * 2) / ${phraseMeasure})`,
        fontWeight: 500,
        letterSpacing: "-0.01em",
        lineHeight: 1.2,
        margin: 0,
        whiteSpace: "nowrap",
        [narrow]: { fontSize: "clamp(1.5rem, 5vw, 2rem)", whiteSpace: "normal" },
    },
    highlight: {
        textDecorationColor: tokens.signal,
        textDecorationLine: "underline",
        textDecorationThickness: "3px",
        textUnderlineOffset: "0.16em",
    },
    senses: {
        color: color.mutedForeground,
        display: "grid",
        fontSize: `min(0.9375rem, calc((${tokens.column} * 4 - ${tokens.inset} * 2) / ${sensesMeasure}))`,
        gap: "0.25rem",
        lineHeight: 1.45,
        listStyle: "none",
        margin: 0,
        marginTop: "0.25rem",
        padding: 0,
        [narrow]: { fontSize: "0.9375rem" },
    },
    sense: {
        display: "flex",
        gap: "0.625rem",
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    key: {
        color: color.foreground,
        fontWeight: 650,
    },
    number: {
        color: tokens.signal,
        flexShrink: 0,
        fontWeight: 700,
    },
});
