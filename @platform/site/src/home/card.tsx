import { text } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";
import { createMemo, For } from "@destack/view";

import { tokens } from "../style/tokens.stylex";

/** The height of a card: five hole rows, kept between a readable least and a most, so taller rows gain room around the cards. */
const cardHeight = `clamp(2.75rem, calc(${tokens.cellRow} * 5), 4rem)`;
/** The media query for tablet-width screens. */
const tablet = "@media (min-width: 768px) and (max-width: 1099px)";
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
    /** The logos under `/logos` of the products the entity rents, shown in place of its tile. */
    logos?: readonly string[];
};

/** Render an entity as a card with an icon chip and a label. */
export function Card(properties: {
    entity: Entity;
    kind: "plain" | "vendor" | "locked";
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
            {properties.entity.logos === undefined ? (
                <Tile entity={properties.entity} isLocked={properties.kind === "locked"} />
            ) : (
                <span {...stylex.attrs(styles.logos)}>
                    {properties.entity.logos.map((logo) => (
                        <img alt="" src={`/logos/${logo}`} {...stylex.attrs(styles.logo)} />
                    ))}
                </span>
            )}
            <span {...stylex.attrs(styles.text)}>
                <span {...stylex.attrs(styles.role)}>{properties.entity.role}</span>
                <span {...stylex.attrs(styles.label)}>{properties.entity.label}</span>
            </span>
        </div>
    );
}

/** Render an entity's icon on a rounded tile in its own colour, or as an outlined lock when locked away. */
function Tile(properties: { entity: Entity; isLocked?: boolean }) {
    return (
        <span
            style={
                properties.isLocked === true
                    ? undefined
                    : {
                          "background-color": properties.entity.tint ?? tokens.signalInk,
                          color: properties.entity.glyph ?? tokens.cream,
                      }
            }
            class={
                stylex.attrs(styles.chip, properties.isLocked === true && styles.lockedChip).class
            }
        >
            <span
                style={{
                    "mask-image": `url(/diagram/${properties.isLocked === true ? "lock" : properties.entity.icon}.svg)`,
                }}
                {...stylex.attrs(styles.icon)}
            />
        </span>
    );
}

/** Render a layer shared by every app as one wide card listing what it holds. */
export function Band(properties: {
    entity: Entity;
    items: readonly Entity[];
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
                                <span {...stylex.attrs(styles.role, styles.itemRole)}>
                                    {item.role}
                                </span>
                                <span {...stylex.attrs(styles.label)}>{item.label}</span>
                            </span>
                        </span>
                    )}
                </For>
            </span>
        </div>
    );
}

/** Render a strip of duct tape with torn ends and a scrawled label. */
export function DuctTape(properties: {
    label: string;
    gap: number;
    turn: number;
    left: string;
    style?: stylex.Styles;
}) {
    const outline =
        "M4 3 L2 6 L5 9 L1 12 L4 15 L1 18 L4 21 L3 23 L68 23 L70 20 L67 17 L71 14 L68 11 L71 8 L68 5 L70 3 Z";

    // keep the last strip on while it peels off, alongside the fresh one going on
    const strips = createMemo<readonly string[]>((previous) => {
        const last = previous?.at(-1);

        return last === undefined || last === properties.label
            ? (previous ?? [properties.label])
            : [last, properties.label];
    });

    return (
        <svg
            aria-hidden="true"
            viewBox="0 0 72 26"
            data-tape={properties.gap}
            style={{ left: properties.left, rotate: `${properties.turn}deg` }}
            {...stylex.attrs(styles.tape, properties.style)}
        >
            {/* peel the old strip off and slap a fresh one on whenever the label changes */}
            <For each={strips()}>
                {(label, index) => (
                    <g
                        {...stylex.attrs(
                            index() < strips().length - 1 ? styles.tapeWorn : styles.tapeFresh,
                        )}
                    >
                        <path d={outline} {...stylex.attrs(styles.tapeStrip)} />
                        <path d="M7 7 H65" {...stylex.attrs(styles.tapeShine)} />
                        <text
                            x="36"
                            y="15.5"
                            text-anchor="middle"
                            dominant-baseline="central"
                            textLength={label.length > 6 ? "52" : undefined}
                            lengthAdjust="spacingAndGlyphs"
                            {...stylex.attrs(styles.tapeText)}
                        >
                            {label}
                        </text>
                    </g>
                )}
            </For>
        </svg>
    );
}

/** A fresh strip of duct tape pressed onto a gap from slightly larger. */
const slap = stylex.keyframes({
    from: { opacity: 0, transform: "scale(1.06)" },
});

/** A worn strip of duct tape fading off as its silo sinks. */
const peel = stylex.keyframes({
    to: { opacity: 0 },
});

/** The card styles. */
const styles = stylex.create({
    card: {
        alignItems: "center",
        backgroundColor: tokens.cream,
        borderColor: `color-mix(in srgb, ${tokens.signalInk} 30%, transparent)`,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: tokens.signalInk,
        display: "flex",
        fontFamily: text.family,
        gap: "0.5rem",
        height: cardHeight,
        paddingInline: "0.375rem 0.625rem",
        position: "relative",
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
        [tablet]: { height: "3.25rem" },
        [mobile]: {
            flexDirection: "column",
            gap: "0.25rem",
            height: "auto",
            justifyContent: "center",
            paddingBlock: "0.375rem",
            paddingInline: "0.25rem",
            textAlign: "center",
            whiteSpace: "normal",
            width: "100%",
        },
    },
    vendorCard: {
        color: tokens.signalInk,
    },
    lockedCard: {
        backgroundColor: `color-mix(in srgb, ${tokens.cream} 18%, transparent)`,
        borderColor: `color-mix(in srgb, ${tokens.signalInk} 70%, transparent)`,
        borderWidth: "1px",
        color: tokens.signalInk,
        cursor: "not-allowed",
        opacity: 0.8,
    },
    band: {
        boxShadow: "none",
        height: cardHeight,
        width: "100%",
        paddingBlock: 0,
        paddingInlineEnd: 0,
        [tablet]: { height: "3.25rem" },
        [mobile]: {
            alignItems: "stretch",
            columnGap: 0,
            display: "grid",
            gridTemplateColumns: "22% minmax(0, 1fr)",
            gridTemplateRows: "auto auto",
            height: "auto",
            justifyItems: "center",
            paddingBlock: 0,
            paddingInline: 0,
            rowGap: "0.25rem",
        },
    },
    bandText: {
        flexShrink: 0,
        width: "8.25rem",
        [mobile]: { gridColumn: 1, gridRow: 2, paddingBottom: "0.375rem", width: "auto" },
    },
    items: {
        alignSelf: "stretch",
        display: "grid",
        flexGrow: 1,
        gridAutoColumns: "minmax(0, 1fr)",
        gridAutoFlow: "column",
        marginLeft: "0.75rem",
        [mobile]: { gridColumn: 2, gridRow: "1 / span 2", justifySelf: "stretch", marginLeft: 0 },
    },
    item: {
        position: "relative",
        alignItems: "center",
        borderLeftColor: `color-mix(in srgb, ${tokens.signalInk} 16%, transparent)`,
        borderLeftStyle: "solid",
        borderLeftWidth: "1px",
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
        [mobile]: {
            flexDirection: "column",
            gap: "0.25rem",
            justifyContent: "center",
            paddingBlock: "0.375rem",
            paddingInline: "0.125rem",
        },
    },
    itemRole: {
        [mobile]: { display: "none" },
    },
    itemActive: {
        boxShadow: `inset 0 -1px 0 color-mix(in srgb, ${tokens.signal} 55%, transparent)`,
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
        [mobile]: { gridColumn: 1, gridRow: 1, marginTop: "0.375rem" },
    },
    logos: {
        display: "flex",
        flexShrink: 0,
        gap: "0.1875rem",
    },
    logo: {
        borderRadius: "4px",
        filter: "saturate(0.7)",
        height: "1.25rem",
        width: "1.25rem",
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
        [mobile]: { fontSize: "0.5625rem", letterSpacing: "0.02em" },
    },
    label: {
        fontSize: "0.875rem",
        fontWeight: 700,
        [mobile]: { fontSize: "0.6875rem" },
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
    tapeFresh: {
        animationDelay: "400ms",
        animationDuration: "240ms",
        animationFillMode: "backwards",
        animationName: slap,
        animationTimingFunction: "cubic-bezier(0.23, 1, 0.32, 1)",
        transformBox: "fill-box",
        transformOrigin: "center",
        "@media (prefers-reduced-motion: reduce)": { animationName: "none" },
    },
    tapeWorn: {
        animationDuration: "240ms",
        animationFillMode: "forwards",
        animationName: peel,
        animationTimingFunction: "ease",
        "@media (prefers-reduced-motion: reduce)": { animationName: "none", opacity: 0 },
    },
    tapeText: {
        fill: "#0b1a20",
        fontFamily: tokens.monoFont,
        fontSize: "11px",
        fontWeight: 700,
        letterSpacing: "0.03em",
    },
});
