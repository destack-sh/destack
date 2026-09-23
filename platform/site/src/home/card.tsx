import { color, fontFamily } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";
import { For } from "@destack/view";

import { tokens } from "../style/tokens.stylex";

const mobile = "@media (max-width: 767px)";

/// What a box shows under the searchlight.
export type Reveal =
    /// An excerpt of one file.
    | { kind: "code"; name: string; lines: readonly string[] }
    /// Named fields and their values.
    | { kind: "fields"; rows: readonly (readonly [string, string])[] }
    /// Ciphertext from a closed vendor.
    | { kind: "cipher" };

/// The lines a searchlight column shows at once before it scrolls.
const visibleLines = 3;

/// The ciphertext every closed box shows.
const cipher = [
    "9f3a c17e 5b21 d4a0 88e1",
    "03bd e2f4 7a90 1c6e 4f28",
    "b7d1 56c0 e93a 2f8b 0d74",
    "6ae2 f105 3c9d 8b47 e0a6",
];

/// The tokens a code line highlights: strings, line marks, keys, keywords and hashes, and punctuation.
const tokenPattern =
    /(?<string>"[^"]*"|'[^']*')|(?<mark>^\s*(?:\$|#|@@|- \[[ x]\]|[-+](?= )))|(?<key>^\s*[\w/@.{}-]+(?=:))|(?<keyword>\b(?:export|function|const|return|import|from|create|table|primary|key|not|null|references|text|date|true|false)\b|\b[0-9a-f]{6}\b)|(?<punctuation>[{}()[\];,<>=:])/g;

/// One highlighted run of a code line.
type Token = { text: string; tone: keyof typeof tones };

/// One entity in the stack figure.
export type Entity = {
    /// The visible label.
    label: string;
    /// The local icon name under `/diagram`.
    icon: string;
    /// The short role printed above the label, such as agent or app.
    role: string;
};

/// Render an entity as a card with an icon chip and a label.
export function Card(props: {
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
                    props.kind === "vendor" && styles.vendorCard,
                    props.kind === "locked" && styles.lockedCard,
                    props.style,
                ).class
            }
        >
            <span
                class={
                    stylex.attrs(
                        styles.chip,
                        props.kind === "vendor" && styles.vendorChip,
                        props.kind === "locked" && styles.lockedChip,
                    ).class
                }
            >
                <span
                    style={{
                        "mask-image": `url(/diagram/${props.kind === "locked" ? "lock" : props.entity.icon}.svg)`,
                    }}
                    {...stylex.attrs(styles.icon)}
                />
            </span>
            <span {...stylex.attrs(styles.text)}>
                <span {...stylex.attrs(styles.role)}>{props.entity.role}</span>
                <span {...stylex.attrs(styles.label)}>{props.entity.label}</span>
            </span>
            {props.reveal && <Reveals reveals={[props.reveal]} />}
        </div>
    );
}

/// Render a layer shared by every app as one wide card listing what it holds.
export function Band(props: {
    entity: Entity;
    items: readonly Entity[];
    active: number;
    reveals: readonly Reveal[];
}) {
    return (
        <div {...stylex.attrs(styles.card, styles.band)}>
            <span {...stylex.attrs(styles.chip)}>
                <span
                    style={{ "mask-image": `url(/diagram/${props.entity.icon}.svg)` }}
                    {...stylex.attrs(styles.icon)}
                />
            </span>
            <span {...stylex.attrs(styles.text)}>
                <span {...stylex.attrs(styles.role)}>{props.entity.role}</span>
                <span {...stylex.attrs(styles.label)}>{props.entity.label}</span>
            </span>
            <span {...stylex.attrs(styles.items)}>
                <For each={props.items}>
                    {(item, index) => (
                        <span
                            class={
                                stylex.attrs(
                                    styles.item,
                                    props.active === index() && styles.itemActive,
                                ).class
                            }
                        >
                            <span
                                style={{ "mask-image": `url(/diagram/${item.icon}.svg)` }}
                                {...stylex.attrs(styles.itemIcon)}
                            />
                            {item.label}
                        </span>
                    )}
                </For>
            </span>
            <Reveals reveals={props.reveals} />
        </div>
    );
}

/// Render what a box holds in columns, shown only where the searchlight falls on it.
function Reveals(props: { reveals: readonly Reveal[] }) {
    return (
        <span data-inside aria-hidden="true" class={stylex.attrs(styles.reveals).class}>
            {props.reveals.map((reveal) => (
                <RevealColumn reveal={reveal} />
            ))}
        </span>
    );
}

/// Render one column of a reveal, scrolling on a loop when it holds more than fits.
function RevealColumn(props: { reveal: Reveal }) {
    const rows = revealRows(props.reveal);
    const isScrolling = rows.length > visibleLines;
    const timing = { "animation-duration": `${rows.length * 1.8}s` };

    return (
        <span class={stylex.attrs(styles.column, !isScrolling && styles.columnStill).class}>
            {props.reveal.kind === "code" && (
                <span class={stylex.attrs(styles.name).class}>{props.reveal.name}</span>
            )}
            <span
                style={isScrolling ? timing : undefined}
                class={
                    stylex.attrs(
                        props.reveal.kind === "fields" ? styles.fieldTrack : styles.track,
                        isScrolling && styles.trackScrolling,
                    ).class
                }
            >
                {rows}
                {isScrolling && revealRows(props.reveal)}
            </span>
        </span>
    );
}

/// Return the rendered rows of a reveal: highlighted code lines, field pairs, or ciphertext lines.
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

/// Split a code line into highlighted tokens and the plain text between them.
function highlight(line: string): Token[] {
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

/// Render a strip of duct tape with torn ends and a scrawled label.
export function DuctTape(props: { label: string; gap: number; style?: stylex.Styles }) {
    const outline =
        "M4 3 L2 6 L5 9 L1 12 L4 15 L1 18 L4 21 L3 23 L68 23 L70 20 L67 17 L71 14 L68 11 L71 8 L68 5 L70 3 Z";

    return (
        <svg
            aria-hidden="true"
            viewBox="0 0 72 26"
            data-tape={props.gap}
            {...stylex.attrs(styles.tape, props.style)}
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
                {props.label}
            </text>
        </svg>
    );
}

const scroll = stylex.keyframes({
    from: { transform: "translateY(0)" },
    to: { transform: "translateY(-50%)" },
});

const tones = stylex.create({
    plain: {},
    string: { color: "#b9d98f" },
    mark: { color: tokens.signal },
    key: { color: "#8fcfdc" },
    keyword: { color: tokens.signal },
    punctuation: { color: "#6f8f99" },
    cipher: { color: "#6f8f99", letterSpacing: "0.12em" },
});

/// The tones of the access verdicts a field can show.
const fieldTones: { [field: string]: stylex.Styles | undefined } = {
    ALLOWED: stylex.create({ tone: { color: "#8fd694" } }).tone,
    BLOCKED: stylex.create({ tone: { color: "#ff6b5b" } }).tone,
    ASKS: tones.keyword,
};

const styles = stylex.create({
    card: {
        alignItems: "center",
        backgroundColor: color.background,
        borderColor: "color-mix(in srgb, currentColor 55%, transparent)",
        borderRadius: "0",
        borderStyle: "solid",
        borderWidth: "1px",
        color: color.foreground,
        display: "flex",
        fontFamily: fontFamily.default,
        gap: "0.75rem",
        height: `calc(${tokens.cellRow} * 5)`,
        paddingInline: "0.875rem 1rem",
        position: "relative",
        whiteSpace: "nowrap",
        width: "max-content",
        zIndex: 1,
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
        backgroundColor: "#ffffff",
        color: tokens.signalInk,
    },
    lockedCard: {
        backgroundColor: "rgb(255 255 255 / 30%)",
        borderColor: `color-mix(in srgb, ${tokens.signalInk} 40%, transparent)`,
        borderStyle: "dashed",
        color: tokens.signalInk,
    },
    band: {
        borderColor: "color-mix(in srgb, currentColor 55%, transparent)",
        height: `calc(${tokens.cellRow} * 5)`,
        width: "100%",
        paddingInline: "0.875rem",
        [mobile]: { flexDirection: "row", height: "2.75rem", paddingBlock: 0 },
    },
    items: {
        display: "flex",
        gap: "0.375rem",
        marginLeft: "auto",
        [mobile]: { display: "none" },
    },
    item: {
        alignItems: "center",
        borderColor: "color-mix(in srgb, currentColor 30%, transparent)",
        borderStyle: "solid",
        borderWidth: "1px",
        color: color.foreground,
        display: "flex",
        fontSize: "0.8125rem",
        fontWeight: 500,
        gap: "0.375rem",
        paddingBlock: "0.25rem",
        paddingInline: "0.4375rem 0.5625rem",
        transition: "color 700ms ease, border-color 700ms ease",
        transitionDelay: "var(--cascade, 0ms)",
        whiteSpace: "nowrap",
    },
    itemIcon: {
        backgroundColor: "currentColor",
        flexShrink: 0,
        height: "1rem",
        maskPosition: "center",
        maskRepeat: "no-repeat",
        maskSize: "contain",
        width: "1rem",
    },
    itemActive: {
        borderColor: tokens.signal,
        color: tokens.signal,
    },
    chip: {
        alignItems: "center",
        display: "flex",
        flexShrink: 0,
        height: "1.5rem",
        justifyContent: "center",
        width: "1.5rem",
    },
    vendorChip: {
        color: tokens.signalInk,
    },
    lockedChip: {
        opacity: 0.8,
    },
    icon: {
        backgroundColor: "currentColor",
        height: "1.5rem",
        maskPosition: "center",
        maskRepeat: "no-repeat",
        maskSize: "contain",
        width: "1.5rem",
    },
    text: {
        display: "flex",
        flexDirection: "column",
        gap: "0.0625rem",
        lineHeight: 1.15,
        [mobile]: { alignItems: "center" },
    },
    role: {
        fontFamily: tokens.monoFont,
        fontSize: "0.6875rem",
        fontWeight: 500,
        letterSpacing: "0.1em",
        opacity: 0.78,
        textTransform: "uppercase",
    },
    label: {
        fontSize: "0.875rem",
        fontWeight: 600,
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
    trackScrolling: {
        animationIterationCount: "infinite",
        animationName: scroll,
        animationPlayState: "var(--looking, paused)",
        animationTimingFunction: "linear",
        "@media (prefers-reduced-motion: reduce)": { animationName: "none" },
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
