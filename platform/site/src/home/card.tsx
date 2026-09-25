import { fontFamily } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";
import { For } from "@destack/view";

import { tokens } from "../style/tokens.stylex";

/** The media query for phone-width screens. */
const mobile = "@media (max-width: 767px)";

/** The orange corner marks around a card on the open stack: two strokes at each corner. */
const cornerMarks = ["0 0", "100% 0", "0 100%", "100% 100%"]
    .flatMap((corner) => [
        `linear-gradient(#ff792e, #ff792e) ${corner} / 9px 1.5px`,
        `linear-gradient(#ff792e, #ff792e) ${corner} / 1.5px 9px`,
    ])
    .join(", ");

/** How far a shared item has risen into its band, from 0 to 1: after the band opens past its middle, one item after another. */
const itemEntry = "var(--band-reveal, 1) * 4 - 1.6 - var(--item, 0) * 0.45";

/** What a box shows under the searchlight. */
export type Reveal =
    /** An excerpt of one file. */
    | { kind: "code"; name: string; lines: readonly string[] }
    /** Named fields and their values. */
    | { kind: "fields"; rows: readonly (readonly [string, string])[] }
    /** Ciphertext from a closed vendor. */
    | { kind: "cipher" };

/** The ciphertext every closed box shows. */
const cipher = ["9f3a c17e 5b21 d4a0 88e1", "03bd e2f4 7a90 1c6e 4f28", "b7d1 56c0 e93a 2f8b 0d74"];

/** The tokens a code line highlights: strings, line marks, keys, keywords and hashes, and punctuation. */
const tokenPattern =
    /(?<string>"[^"]*"|'[^']*')|(?<mark>^\s*(?:\$|#|@@|- \[[ x]\]|[-+](?= )))|(?<key>^\s*[\w/@.{}-]+(?=:))|(?<keyword>\b(?:export|function|const|return|import|from|create|table|primary|key|not|null|references|text|date|true|false)\b|\b[0-9a-f]{6}\b)|(?<punctuation>[{}()[\];,<>=:])/g;

/** One highlighted run of a code line. */
type Token = { text: string; tone: keyof typeof tones };

/** One entity in the stack figure. */
export type Entity = {
    /** The visible label. */
    label: string;
    /** The local icon name under `/diagram`. */
    icon: string;
    /** The short role printed above the label, such as agent or app. */
    role: string;
    /** The icon tile's colour, or ink by default. */
    tint?: string;
    /** The icon's colour on its tile, or cream by default. */
    glyph?: string;
};

/** Render an entity as a card with an icon chip and a label. */
export function Card(properties: {
    entity: Entity;
    kind: "plain" | "vendor" | "locked";
    reveal?: Reveal;
    style?: stylex.Styles;
}) {
    return (
        <div
            class={
                stylex.attrs(
                    styles.card,
                    properties.kind === "vendor" && styles.vendorCard,
                    properties.kind === "locked" && styles.lockedCard,
                    properties.style,
                ).class
            }
        >
            <Tile entity={properties.entity} isLocked={properties.kind === "locked"} />
            <span {...stylex.attrs(styles.text)}>
                <span {...stylex.attrs(styles.role)}>{properties.entity.role}</span>
                <span {...stylex.attrs(styles.label)}>{properties.entity.label}</span>
            </span>
            {properties.reveal && <Reveals reveals={[properties.reveal]} />}
        </div>
    );
}

/** Render an entity's icon on a rounded tile in its own colour, or as an outlined lock when locked away. */
function Tile(properties: { entity: Entity; isLocked?: boolean }) {
    return (
        <span
            style={
                properties.isLocked
                    ? undefined
                    : {
                          "background-color": properties.entity.tint ?? tokens.signalInk,
                          color: properties.entity.glyph ?? tokens.cream,
                      }
            }
            class={stylex.attrs(styles.chip, properties.isLocked && styles.lockedChip).class}
        >
            <span
                style={{
                    "mask-image": `url(/diagram/${properties.isLocked ? "lock" : properties.entity.icon}.svg)`,
                }}
                {...stylex.attrs(styles.icon)}
            />
        </span>
    );
}

/** Render a layer shared by every app as one wide card listing what it holds. */
export function Band(properties: {
    entity: Entity;
    items: readonly (Entity & { reveal: Reveal })[];
    active: readonly string[];
}) {
    return (
        <div {...stylex.attrs(styles.card, styles.band)}>
            <Tile entity={properties.entity} />
            <span {...stylex.attrs(styles.text, styles.bandText)}>
                <span {...stylex.attrs(styles.role)}>{properties.entity.role}</span>
                <span {...stylex.attrs(styles.label)}>{properties.entity.label}</span>
            </span>
            <span {...stylex.attrs(styles.items)}>
                <For each={properties.items}>
                    {(item, index) => (
                        <span
                            style={{ "--item": String(index()) }}
                            data-item={item.label}
                            data-live={properties.active.includes(item.label) ? "" : undefined}
                            class={
                                stylex.attrs(
                                    styles.item,
                                    properties.active.includes(item.label) && styles.itemActive,
                                ).class
                            }
                        >
                            <Tile entity={item} />
                            <span {...stylex.attrs(styles.text)}>
                                <span {...stylex.attrs(styles.role)}>{item.role}</span>
                                <span {...stylex.attrs(styles.label)}>{item.label}</span>
                            </span>
                            <Reveals reveals={[item.reveal]} />
                        </span>
                    )}
                </For>
            </span>
        </div>
    );
}

/** Render what a box holds in columns, shown only where the searchlight falls on it. */
function Reveals(properties: { reveals: readonly Reveal[] }) {
    return (
        <span data-inside aria-hidden="true" class={stylex.attrs(styles.reveals).class}>
            {properties.reveals.map((reveal) => (
                <RevealColumn reveal={reveal} />
            ))}
        </span>
    );
}

/** Render one column of a reveal: a file name for code, then its three lines. */
function RevealColumn(properties: { reveal: Reveal }) {
    return (
        <span class={stylex.attrs(styles.column, styles.columnStill).class}>
            {properties.reveal.kind === "code" && (
                <span class={stylex.attrs(styles.name).class}>{properties.reveal.name}</span>
            )}
            <span
                class={
                    stylex.attrs(
                        properties.reveal.kind === "fields" ? styles.fieldTrack : styles.track,
                    ).class
                }
            >
                {revealRows(properties.reveal)}
            </span>
        </span>
    );
}

/** Return the rendered rows of a reveal: highlighted code lines, field pairs, or ciphertext lines. */
function revealRows(reveal: Reveal) {
    // highlight each code line
    if (reveal.kind === "code") {
        return reveal.lines.map((line) => (
            <span class={stylex.attrs(styles.line).class}>
                {highlight(line).map((token) => (
                    <span class={stylex.attrs(tones[token.tone]).class}>{token.text}</span>
                ))}
            </span>
        ));
    }
    // align each field with its value
    else if (reveal.kind === "fields") {
        return reveal.rows.map(([field, value]) => (
            <>
                <span class={stylex.attrs(styles.line, fieldTones[field] ?? tones.key).class}>
                    {field}
                </span>
                <span class={stylex.attrs(styles.line).class}>{value}</span>
            </>
        ));
    }
    // scramble closed boxes
    else {
        return cipher.map((line) => (
            <span class={stylex.attrs(styles.line, tones.cipher).class}>{line}</span>
        ));
    }
}

/** Split a code line into highlighted tokens and the plain text between them. */
function highlight(line: string): Token[] {
    // split the line at each token match
    const tokens: Token[] = [];
    let end = 0;
    for (const match of line.matchAll(tokenPattern)) {
        const tone = Object.entries(match.groups!).find(([, text]) => text !== undefined)![0];
        if (match.index > end) {
            tokens.push({ text: line.slice(end, match.index), tone: "plain" });
        }
        tokens.push({ text: match[0], tone: tone as Token["tone"] });
        end = match.index + match[0].length;
    }

    // keep the rest of the line plain
    if (end < line.length) {
        tokens.push({ text: line.slice(end), tone: "plain" });
    }

    return tokens;
}

/** Render a strip of duct tape with torn ends and a scrawled label. */
export function DuctTape(properties: {
    label: string;
    gap: number;
    left: string;
    style?: stylex.Styles;
}) {
    const outline =
        "M4 3 L2 6 L5 9 L1 12 L4 15 L1 18 L4 21 L3 23 L68 23 L70 20 L67 17 L71 14 L68 11 L71 8 L68 5 L70 3 Z";

    return (
        <svg
            aria-hidden="true"
            viewBox="0 0 72 26"
            data-tape={properties.gap}
            style={{ left: properties.left }}
            {...stylex.attrs(styles.tape, properties.style)}
        >
            <path d={outline} {...stylex.attrs(styles.tapeStrip)} />
            <path d="M7 7 H65" {...stylex.attrs(styles.tapeShine)} />
            <text
                x="36"
                y="15.5"
                text-anchor="middle"
                dominant-baseline="central"
                {...stylex.attrs(styles.tapeText)}
            >
                {properties.label}
            </text>
        </svg>
    );
}

/** The colours of each highlighted token tone. */
const tones = stylex.create({
    plain: {},
    string: { color: "#b9d98f" },
    mark: { color: tokens.signal },
    key: { color: "#8fcfdc" },
    keyword: { color: tokens.signal },
    punctuation: { color: "#6f8f99" },
    cipher: { color: "#6f8f99", letterSpacing: "0.12em" },
});

/** The tones of the access verdicts a field can show. */
const fieldTones: { [field: string]: stylex.Styles | undefined } = {
    ALLOWED: stylex.create({ tone: { color: "#8fd694" } }).tone,
    BLOCKED: stylex.create({ tone: { color: "#ff6b5b" } }).tone,
    ASKS: tones.keyword,
};

/** The card styles. */
const styles = stylex.create({
    card: {
        alignItems: "center",
        backgroundColor: tokens.cream,
        borderColor: tokens.signalInk,
        borderRadius: "0",
        borderStyle: "solid",
        borderWidth: "2px",
        boxShadow: `4px 4px 0 var(--card-shadow, ${tokens.signalInk})`,
        color: tokens.signalInk,
        display: "flex",
        fontFamily: fontFamily.default,
        gap: "0.5rem",
        height: `calc(${tokens.cellRow} * 5)`,
        paddingInline: "0.375rem 0.625rem",
        position: "relative",
        transition: "box-shadow 600ms ease",
        whiteSpace: "nowrap",
        width: "max-content",
        zIndex: 1,
        "::before": {
            backgroundImage: cornerMarks,
            backgroundRepeat: "no-repeat",
            content: "''",
            inset: "-7px",
            opacity: "var(--card-marks, 0)",
            pointerEvents: "none",
            position: "absolute",
            transition: "opacity 600ms ease",
        },
        [mobile]: {
            flexDirection: "column",
            gap: "0.25rem",
            height: "auto",
            justifyContent: "center",
            paddingBlock: "0.375rem",
            paddingInline: "0.25rem",
            width: "100%",
        },
    },
    vendorCard: {
        color: tokens.signalInk,
    },
    lockedCard: {
        backgroundColor: `color-mix(in srgb, ${tokens.cream} 18%, transparent)`,
        borderColor: `color-mix(in srgb, ${tokens.signalInk} 70%, transparent)`,
        borderWidth: "1.5px",
        boxShadow: "none",
        color: tokens.signalInk,
        cursor: "not-allowed",
        opacity: 0.8,
    },
    band: {
        boxShadow: "none",
        height: `calc(${tokens.cellRow} * 5)`,
        width: "100%",
        paddingBlock: 0,
        paddingInlineEnd: 0,
        [mobile]: { flexDirection: "row", height: "2.75rem", paddingBlock: 0 },
    },
    bandText: {
        flexShrink: 0,
        width: "8.25rem",
    },
    items: {
        alignSelf: "stretch",
        display: "grid",
        flexGrow: 1,
        gridAutoColumns: "minmax(0, 1fr)",
        gridAutoFlow: "column",
        marginLeft: "0.75rem",
        [mobile]: { display: "none" },
    },
    item: {
        position: "relative",
        alignItems: "center",
        borderLeftColor: tokens.signalInk,
        borderLeftStyle: "solid",
        borderLeftWidth: "2px",
        boxShadow: `inset 0 0 0 ${tokens.signal}`,
        color: tokens.signalInk,
        display: "flex",
        fontSize: "0.75rem",
        fontWeight: 600,
        gap: "0.5rem",
        minWidth: 0,
        paddingInline: "0.75rem",
        transition: "box-shadow 500ms ease, opacity 500ms ease, filter 500ms ease",
        transitionDelay: "var(--cascade, 0ms)",
        opacity: `clamp(0, ${itemEntry}, 1)`,
        translate: `0 calc((1 - clamp(0, ${itemEntry}, 1)) * 0.75rem)`,
        whiteSpace: "nowrap",
    },
    itemActive: {
        boxShadow: `inset 0 -5px 0 ${tokens.signal}`,
    },
    chip: {
        opacity: "var(--ink, 1)",
        transition: "opacity var(--ink-time, 0ms) ease var(--ink-delay, 0ms)",
        alignItems: "center",
        borderRadius: "6px",
        boxShadow: `inset 0 0 0 1.5px color-mix(in srgb, ${tokens.signalInk} 22%, transparent)`,
        display: "flex",
        flexShrink: 0,
        height: "1.625rem",
        justifyContent: "center",
        width: "1.625rem",
    },
    lockedChip: {
        backgroundColor: "transparent",
        borderColor: "currentColor",
        borderStyle: "solid",
        borderWidth: "1.5px",
        color: tokens.signalInk,
    },
    icon: {
        backgroundColor: "currentColor",
        height: "1.125rem",
        maskPosition: "center",
        maskRepeat: "no-repeat",
        maskSize: "contain",
        width: "1.125rem",
    },

    text: {
        opacity: "var(--ink, 1)",
        transition: "opacity var(--ink-time, 0ms) ease var(--ink-delay, 0ms)",
        display: "flex",
        flexDirection: "column",
        gap: "0.0625rem",
        lineHeight: 1.15,
        [mobile]: { alignItems: "center" },
    },
    role: {
        fontFamily: tokens.monoFont,
        fontSize: "0.625rem",
        fontWeight: 500,
        letterSpacing: "0.06em",
        opacity: 0.78,
        textTransform: "uppercase",
    },
    label: {
        fontSize: "0.875rem",
        fontWeight: 700,
        [mobile]: { fontSize: "0.6875rem" },
    },
    reveals: {
        backgroundColor: tokens.night,
        color: tokens.cream,
        columnGap: "1rem",
        display: "flex",
        fontFamily: tokens.monoFont,
        fontSize: "0.625rem",
        inset: "-1px",
        lineHeight: "0.8125rem",
        maskImage:
            "radial-gradient(circle var(--lens-radius, 0px) at var(--lens-x, -999px) var(--lens-y, -999px), #000 calc(var(--lens-radius, 0px) - 1px), transparent var(--lens-radius, 0px))",
        overflow: "hidden",
        paddingBlock: "0.25rem",
        paddingInline: "0.625rem",
        pointerEvents: "none",
        position: "absolute",
        whiteSpace: "pre",
        zIndex: 2,
    },
    column: {
        display: "flex",
        flex: "1 1 0",
        flexDirection: "column",
        minWidth: 0,
        overflow: "hidden",
        position: "relative",
    },
    columnStill: {
        justifyContent: "center",
    },
    name: {
        backgroundColor: tokens.night,
        color: "#6f8f99",
        fontSize: "0.5625rem",
        paddingLeft: "0.5rem",
        position: "absolute",
        right: 0,
        top: 0,
        zIndex: 1,
    },
    track: {
        display: "flex",
        flexDirection: "column",
    },
    fieldTrack: {
        columnGap: "0.5rem",
        display: "grid",
        gridTemplateColumns: "max-content 1fr",
    },
    line: {
        minHeight: "0.8125rem",
        overflow: "hidden",
        textOverflow: "ellipsis",
    },
    tape: {
        height: "26px",
        left: 0,
        marginLeft: "-36px",
        marginTop: "-13px",
        overflow: "visible",
        pointerEvents: "none",
        position: "absolute",
        top: 0,
        width: "72px",
        zIndex: 2,
    },
    tapeStrip: {
        fill: "#cdd3d6",
        stroke: `color-mix(in srgb, ${tokens.signalInk} 60%, transparent)`,
        strokeLinejoin: "round",
        strokeWidth: 1,
        vectorEffect: "non-scaling-stroke",
    },
    tapeShine: {
        opacity: 0.8,
        stroke: "#f1f3f4",
        strokeWidth: 1.25,
        vectorEffect: "non-scaling-stroke",
    },
    tapeText: {
        fill: "#0b1a20",
        fontFamily: tokens.monoFont,
        fontSize: "11px",
        fontWeight: 700,
        letterSpacing: "0.06em",
    },
});
