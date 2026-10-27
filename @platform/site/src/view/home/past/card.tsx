import { frame } from "../../layout/frame.stylex";
import { font, motion } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import { createMemo, For } from "@destack/view";

import { cardVariables, tape } from "./card.stylex";
import { palette } from "../../palette.stylex";
import { AgentAvatar, isAgent } from "../figure/agent";
import { Glyph } from "../figure/glyph";
import { media } from "@destack/style/media.stylex";
import { screen } from "../../layout/screen.stylex";
import { typeScale, tracking } from "../../layout/type.stylex";

/** The height of a card: five hole rows, kept between a readable least and a most. */
const cardHeight = `clamp(2.75rem, calc(${frame.cellRow} * 5), 4rem)`;
/** The orange corner marks around a card on the open stack: two strokes at each corner. */
const cornerMarks = ["0 0", "100% 0", "0 100%", "100% 100%"]
    .flatMap((corner) => [
        `linear-gradient(#ff792e, #ff792e) ${corner} / 9px 1.5px`,
        `linear-gradient(#ff792e, #ff792e) ${corner} / 1.5px 9px`,
    ])
    .join(", ");

/** How far a shared item has risen into its band, from 0 to 1. */
const itemEntry = "var(--band-reveal, 1) * 4 - 1.6 - var(--item, 0) * 0.45";

/** One entity in the stack figure. */
export type Entity = {
    /** The visible label. */
    label: string;
    /** The local icon name under `/diagram`. */
    icon: string;
    /** The short role printed above the label, such as agent or app. */
    role: string;
    /** The icon tile's colour, absent for your space's own apps, which wear the brand's orange. */
    tint?: string;
    /** The icon's colour on its tile, or cream by default. */
    glyph?: string;
    /** The logos under `/logos` of the products the entity rents, shown in place of its tile. */
    logos?: readonly string[];
    /** Whether the entity is an app you made or remixed in your space, marked as remixable. */
    isRemix?: boolean;
};

/** Render an entity as a card with an icon chip and a label. */
export function Card(properties: {
    entity: Entity;
    kind: "plain" | "vendor" | "locked";
    xstyle?: style.Styles;
}) {
    return (
        <div
            class={
                style.attrs(
                    styles.card,
                    properties.kind === "vendor" && styles.vendorCard,
                    properties.kind === "locked" && styles.lockedCard,
                    properties.xstyle,
                ).class
            }
        >
            {properties.entity.logos === undefined ? (
                <Tile entity={properties.entity} isLocked={properties.kind === "locked"} />
            ) : (
                <span {...style.attrs(styles.logos)}>
                    {properties.entity.logos.map((logo) => (
                        <img alt="" src={`/logos/${logo}`} {...style.attrs(styles.logo)} />
                    ))}
                </span>
            )}
            <span {...style.attrs(styles.text)}>
                <span {...style.attrs(styles.role)}>{properties.entity.role}</span>
                <span {...style.attrs(styles.label)}>{properties.entity.label}</span>
            </span>
            {properties.entity.isRemix === true ? (
                <span title="Remix" {...style.attrs(styles.remix)}>
                    <Glyph name="remix" size={12} />
                </span>
            ) : undefined}
        </div>
    );
}

/** Render an entity's icon on a tile in its own colour, your agent as its avatar, or a lock. */
function Tile(properties: { entity: Entity; isLocked?: boolean }) {
    return (
        <>
            {/* show your agent as its avatar, filling the tile */}
            {isAgent(properties.entity.icon) && properties.isLocked !== true ? (
                <span {...style.attrs(styles.chip, styles.avatarChip)}>
                    <AgentAvatar size="fill" />
                </span>
            ) : (
                <span
                    style={
                        properties.isLocked === true
                            ? undefined
                            : {
                                  "background-color": properties.entity.tint,
                                  color:
                                      properties.entity.glyph ??
                                      (isBrand(properties.entity)
                                          ? palette.tileGlyph
                                          : palette.cream),
                              }
                    }
                    class={
                        style.attrs(
                            styles.chip,
                            isBrand(properties.entity) && styles.brandChip,
                            properties.isLocked === true && styles.lockedChip,
                        ).class
                    }
                >
                    <span
                        style={{
                            "mask-image": `url(/diagram/${iconPathOf(properties.entity, properties.isLocked === true)}.svg)`,
                        }}
                        {...style.attrs(styles.icon)}
                    />
                </span>
            )}
        </>
    );
}

/** Return the glyph path of an entity's tile: the lock, a solid app glyph, or its own glyph. */
function iconPathOf(entity: Entity, isLocked: boolean): string {
    if (isLocked) {
        return "lock";
    } else if (isBrand(entity)) {
        return `fill/${entity.icon}`;
    }

    return entity.icon;
}

/** Report whether an entity is one of your space's own, as a white glyph on the sky tile. */
function isBrand(entity: Entity): boolean {
    return entity.tint === undefined;
}

/** Render a layer shared by every app as one wide card listing what it holds. */
export function Band(properties: {
    entity: Entity;
    items: readonly Entity[];
    active: readonly string[];
}) {
    return (
        <div {...style.attrs(styles.card, styles.band)}>
            <Tile entity={properties.entity} />
            <span {...style.attrs(styles.text, styles.bandText)}>
                <span {...style.attrs(styles.role)}>{properties.entity.role}</span>
                <span {...style.attrs(styles.label)}>{properties.entity.label}</span>
            </span>
            <span {...style.attrs(styles.items)}>
                <For each={properties.items}>
                    {(item, index) => (
                        <span
                            style={{ "--item": String(index()) }}
                            data-item={item.label}
                            data-live={properties.active.includes(item.label) ? "" : undefined}
                            class={
                                style.attrs(
                                    styles.item,
                                    properties.active.includes(item.label) && styles.itemActive,
                                ).class
                            }
                        >
                            <Tile entity={item} />
                            <span {...style.attrs(styles.text)}>
                                <span {...style.attrs(styles.role, styles.itemRole)}>
                                    {item.role}
                                </span>
                                <span {...style.attrs(styles.label)}>{item.label}</span>
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
    xstyle?: style.Styles;
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
            {...style.attrs(styles.tape, properties.xstyle)}
        >
            {/* peel the old strip off and slap a fresh one on whenever the label changes */}
            <For each={strips()}>
                {(label, index) => (
                    <g
                        {...style.attrs(
                            index() < strips().length - 1 ? styles.tapeWorn : styles.tapeFresh,
                        )}
                    >
                        <path d={outline} {...style.attrs(styles.tapeStrip)} />
                        <path d="M7 7 H65" {...style.attrs(styles.tapeShine)} />
                        <text
                            x="36"
                            y="15.5"
                            text-anchor="middle"
                            dominant-baseline="central"
                            textLength={label.length > 6 ? "52" : undefined}
                            lengthAdjust="spacingAndGlyphs"
                            {...style.attrs(styles.tapeText)}
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
const slap = style.keyframes({
    from: { opacity: 0, transform: "scale(1.06)" },
});

/** A worn strip of duct tape fading off as its silo sinks. */
const peel = style.keyframes({
    to: { opacity: 0 },
});

/** The card styles. */
const styles = style.create({
    remix: {
        color: palette.signal,
        display: "flex",
        marginInlineStart: "0.125rem",
    },
    card: {
        alignItems: "center",
        backgroundColor: palette.cream,
        borderColor: `color-mix(in srgb, ${palette.ink} 30%, transparent)`,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: palette.ink,
        display: "flex",
        fontFamily: font.text,
        gap: { default: "0.5rem", [media.maxMd]: "0.25rem" },
        height: { default: cardHeight, [screen.tablet]: "3.25rem", [media.maxMd]: "auto" },
        paddingInlineStart: "0.375rem",
        paddingInlineEnd: "0.625rem",
        position: "relative",
        whiteSpace: { default: "nowrap", [media.maxMd]: "normal" },
        width: { default: "max-content", [media.maxMd]: "100%" },
        zIndex: 1,
        "::before": {
            backgroundImage: cornerMarks,
            backgroundRepeat: "no-repeat",
            content: "''",
            inset: "-7px",
            opacity: cardVariables.marks,
            pointerEvents: "none",
            position: "absolute",
            transition: `opacity ${motion.durationMedium} ${motion.easingStandard}`,
        },
        flexDirection: { default: null, [media.maxMd]: "column" },
        justifyContent: { default: null, [media.maxMd]: "center" },
        paddingBlock: { default: null, [media.maxMd]: "0.375rem" },
        textAlign: { default: null, [media.maxMd]: "center" },
    },
    vendorCard: {
        color: palette.ink,
    },
    lockedCard: {
        backgroundColor: `color-mix(in srgb, ${palette.cream} 18%, transparent)`,
        borderColor: `color-mix(in srgb, ${palette.ink} 70%, transparent)`,
        borderWidth: "1px",
        color: palette.ink,
        cursor: "not-allowed",
        opacity: 0.8,
    },
    band: {
        boxShadow: "none",
        height: { default: cardHeight, [screen.tablet]: "3.25rem", [media.maxMd]: "auto" },
        width: "100%",
        paddingBlock: { default: 0, [media.maxMd]: 0 },
        paddingInlineEnd: 0,
        alignItems: { default: "center", [media.maxMd]: "stretch" },
        columnGap: { default: null, [media.maxMd]: 0 },
        display: { default: "flex", [media.maxMd]: "grid" },
        gridTemplateColumns: { default: null, [media.maxMd]: "22% minmax(0, 1fr)" },
        gridTemplateRows: { default: null, [media.maxMd]: "auto auto" },
        justifyItems: { default: null, [media.maxMd]: "center" },
        rowGap: { default: null, [media.maxMd]: "0.25rem" },
    },
    bandText: {
        flexShrink: 0,
        width: { default: "8.25rem", [media.maxMd]: "auto" },
        gridColumn: { default: null, [media.maxMd]: 1 },
        gridRow: { default: null, [media.maxMd]: 2 },
        paddingBottom: { default: null, [media.maxMd]: "0.375rem" },
    },
    items: {
        alignSelf: "stretch",
        display: "grid",
        flexGrow: 1,
        gridAutoColumns: "minmax(0, 1fr)",
        gridAutoFlow: "column",
        marginLeft: { default: "0.75rem", [media.maxMd]: 0 },
        gridColumn: { default: null, [media.maxMd]: 2 },
        gridRow: { default: null, [media.maxMd]: "1 / span 2" },
        justifySelf: { default: null, [media.maxMd]: "stretch" },
    },
    item: {
        position: "relative",
        alignItems: "center",
        borderLeftColor: `color-mix(in srgb, ${palette.ink} 16%, transparent)`,
        borderLeftStyle: "solid",
        borderLeftWidth: "1px",
        boxShadow: `inset 0 0 0 ${palette.signal}`,
        color: palette.ink,
        display: "flex",
        fontSize: typeScale.small,
        fontWeight: 600,
        gap: { default: "0.5rem", [media.maxMd]: "0.25rem" },
        minWidth: 0,
        paddingInline: { default: "0.75rem", [media.maxMd]: "0.125rem" },
        transitionDuration: motion.durationMedium,
        transitionProperty: "box-shadow, opacity, filter",
        transitionTimingFunction: motion.easingStandard,
        transitionDelay: cardVariables.cascade,
        opacity: `clamp(0, ${itemEntry}, 1)`,
        translate: `0 calc((1 - clamp(0, ${itemEntry}, 1)) * 0.75rem)`,
        whiteSpace: "nowrap",
        flexDirection: { default: null, [media.maxMd]: "column" },
        justifyContent: { default: null, [media.maxMd]: "center" },
        paddingBlock: { default: null, [media.maxMd]: "0.375rem" },
    },
    itemRole: { display: { default: null, [media.maxMd]: "none" } },
    itemActive: {
        boxShadow: `inset 0 -1px 0 color-mix(in srgb, ${palette.signal} 55%, transparent)`,
    },
    chip: {
        opacity: cardVariables.ink,
        transition: `opacity ${cardVariables.inkTime} ${motion.easingStandard} ${cardVariables.inkDelay}`,
        alignItems: "center",
        borderRadius: "6px",
        boxShadow: `inset 0 0 0 1.5px color-mix(in srgb, ${palette.ink} 22%, transparent)`,
        display: "flex",
        flexShrink: 0,
        height: "1.625rem",
        justifyContent: "center",
        width: "1.625rem",
        gridColumn: { default: null, [media.maxMd]: 1 },
        gridRow: { default: null, [media.maxMd]: 1 },
        marginTop: { default: null, [media.maxMd]: "0.375rem" },
    },
    avatarChip: {
        boxShadow: "none",
    },
    brandChip: {
        backgroundImage: palette.skyField,
        boxShadow: `inset 0 1px 0 color-mix(in srgb, ${palette.cream} 14%, transparent), inset 0 0 0 0.5px color-mix(in srgb, ${palette.cream} 10%, transparent)`,
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
        color: palette.ink,
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
        opacity: cardVariables.ink,
        transition: `opacity ${cardVariables.inkTime} ${motion.easingStandard} ${cardVariables.inkDelay}`,
        display: "flex",
        flexDirection: "column",
        gap: "0.0625rem",
        lineHeight: 1.15,
        alignItems: { default: null, [media.maxMd]: "center" },
    },
    role: {
        fontFamily: font.code,
        fontSize: typeScale.label,
        fontWeight: 400,
        letterSpacing: tracking.label,
        opacity: 0.78,
        textTransform: "uppercase",
    },
    label: {
        fontSize: { default: typeScale.small, [media.maxMd]: typeScale.label },
        fontWeight: 600,
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
        fill: tape.strip,
        stroke: `color-mix(in srgb, ${palette.ink} 60%, transparent)`,
        strokeLinejoin: "round",
        strokeWidth: 1,
        vectorEffect: "non-scaling-stroke",
    },
    tapeShine: {
        opacity: 0.8,
        stroke: tape.shine,
        strokeWidth: 1.25,
        vectorEffect: "non-scaling-stroke",
    },
    tapeFresh: {
        animationDelay: "240ms",
        animationDuration: motion.durationShort,
        animationFillMode: "backwards",
        animationName: slap,
        animationTimingFunction: motion.easingEmphasised,
        transformBox: "fill-box",
        transformOrigin: "center",
    },
    tapeWorn: {
        animationDuration: motion.durationShort,
        animationFillMode: "forwards",
        animationName: peel,
        animationTimingFunction: motion.easingStandard,
        opacity: { default: null, [media.motionReduce]: 0 },
    },
    tapeText: {
        fill: tape.ink,
        fontFamily: font.code,
        fontSize: typeScale.label,
        fontWeight: 600,
    },
});
