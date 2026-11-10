import { frame } from "./frame.stylex";
import { color, font, stroke } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { screen } from "./screen.stylex";

/** The rule colour: a faint, opaque tint of the text, so rules that cross or overlap stay even. */
const RULE = `color-mix(in srgb, ${color.foreground} 10%, ${color.background})`;

/** The ink of a mark where two rules cross: the signal orange. */
const MARK_INK = color.primary;

/** The length of each arm pair of a crossing mark, odd so it centres on a hairline. */
const MARK = "9px";

/** How far an interval overlaps its neighbours, so its marks fit around its edge rules. */
const EDGE = "4px";

/** The spacing of the dot grid. */
const DOT_GRID = "16px";

/** The spacing of the margins' dot grid: every other dot of the page's, so the two stay aligned. */
const MARGIN_DOT_GRID = "32px";

/** The ink of a ruler's ticks along a running head's top rule. */
const TICK_INK = `color-mix(in srgb, ${color.foreground} 32%, ${color.background})`;

/** A ruler's ticks across the frame: one per column, and quarter columns between. */
const TICKS = {
    column: `repeating-linear-gradient(to right, ${TICK_INK} 0 ${stroke.border}, transparent 0 calc(${frame.width} / 12))`,
    quarter: `repeating-linear-gradient(to right, ${TICK_INK} 0 ${stroke.border}, transparent 0 calc(${frame.width} / 48))`,
};

/** The blank gap that closes a section before the next one's running head. */
const GAP = "3rem";

/** The height of the hatched band that carries a section's running head. */
const BAND = "2.5rem";

/** A paper grain of faint ink specks, noise baked into a tile so no filter renders on scroll. */
const GRAIN = `url("/grain.webp")`;

/** The size the grain's tile is shown at, half its pixels for sharp grain on dense screens. */
const GRAIN_SIZE = "180px 180px";

/** The ink of the dot grid, a little stronger than the rules so single dots still read. */
const DOT_INK = `color-mix(in srgb, ${color.foreground} 22%, ${color.background})`;

/** The dot grid of bands and figure margins: one dot at the corner of each tile. */
const DOTS = `radial-gradient(circle at 1px 1px, ${DOT_INK} 0.9px, transparent 1.25px)`;

/** A row of crossing marks' horizontal arms, one centred on each rail. */
const MARK_ARMS = `linear-gradient(to right, ${MARK_INK} ${MARK}, transparent 0 calc(100% - ${MARK}), ${MARK_INK} 0)`;

/** A row of crossing marks' vertical arms, one on each rail. */
const MARK_STEMS = `linear-gradient(to right, ${MARK_INK} ${stroke.border}, transparent 0 calc(100% - ${stroke.border}), ${MARK_INK} 0)`;

/** The width of a row of crossing marks' arms, reaching half an arm past each rail. */
const ARMS_WIDTH = `calc(${frame.width} + ${MARK} - ${stroke.border})`;

/** The width of each margin beside the frame. */
const MARGIN = `calc((100% - ${frame.width}) / 2)`;

/** A margin of the dot grid beside the frame, under the page's content, tiled from the rails. */
const margin = {
    backgroundImage: DOTS,
    backgroundSize: `${MARGIN_DOT_GRID} ${MARGIN_DOT_GRID}`,
    bottom: 0,
    content: '""',
    pointerEvents: "none",
    position: "absolute",
    top: 0,
    width: MARGIN,
    zIndex: -1,
} as const;

/** The page under every site page: grain, and dot grid in the margins beside the frame. */
export const paper = style.create({
    /** The page's ground. */
    ground: {
        backgroundColor: color.background,
        backgroundImage: GRAIN,
        backgroundSize: GRAIN_SIZE,
        "::before": { ...margin, backgroundPosition: "right top", left: 0 },
        "::after": { ...margin, backgroundPosition: "left top", right: 0 },
    },

    /** A patch of page over what scrolls under it, grained like the page. */
    patch: {
        backgroundColor: color.background,
        backgroundImage: GRAIN,
        backgroundSize: GRAIN_SIZE,
    },
});

/** The colours lattice cells paint, which a band such as the sky sets for its own cells. */
export const surface = style.defineVars({
    /** The colour a cell paints over the rules. */
    cell: color.background,
});

/** The rule colours other site styles draw grid rules in. */
export const rule = style.defineConsts({
    /** The colour of every grid rule. */
    color: "color-mix(in srgb, var(--destack-color-foreground) 10%, var(--destack-color-background))",
});

/** The twelve-column frame and the rules drawn between its cells. */
export const lattice = style.create({
    /** A full-width band on the shared columns, closed by the frame edges. */
    frame: {
        borderInlineColor: RULE,
        borderInlineStyle: "solid",
        borderInlineWidth: { default: stroke.border, [media.maxMd]: 0 },
        display: "grid",
        gridTemplateColumns: {
            default: "repeat(12, minmax(0, 1fr))",
            [media.maxMd]: "repeat(4, minmax(0, 1fr))",
        },
        marginInline: "auto",
        maxWidth: frame.width,
        minWidth: 0,
        position: "relative",
        width: "100%",
    },

    /** A homepage section one screen tall on desktop, so every section shares one height. */
    section: {
        height: { default: frame.section, [screen.belowDesktop]: "auto" },
    },

    /** A band whose cells sit on the rule colour a hairline apart. */
    ruled: {
        backgroundClip: "padding-box",
        backgroundColor: RULE,
        gap: stroke.border,
    },

    /** A cell of a ruled band in the page colour, its top rule run across the window. */
    cell: {
        backgroundColor: surface.cell,
        backgroundImage: GRAIN,
        backgroundSize: GRAIN_SIZE,
        borderImageOutset: `${stroke.border} 100vw 0`,
        borderImageSlice: "1 0 0 0",
        borderImageSource: {
            default: `linear-gradient(${RULE}, ${RULE})`,
            ":first-child": "none",
        },
        borderImageWidth: `${stroke.border} 0 0 0`,
        minWidth: 0,
    },

    /** A figure cell of a ruled band, dotted behind its figure and unrendered off screen. */
    figureCell: {
        backgroundImage: `${DOTS}, ${GRAIN}`,
        backgroundSize: `${DOT_GRID} ${DOT_GRID}, ${GRAIN_SIZE}`,
        containIntrinsicSize: "auto 32rem",
        contentVisibility: { default: "auto", ":is([data-near] *)": "visible" },
        display: "grid",
        gap: "0.75rem",
        gridTemplateRows: "auto minmax(0, 1fr) auto",
        minHeight: 0,
        padding: `1.5rem ${frame.inset}`,
    },

    /** The space between two sections: a closing rule, a blank gap, then the next head on dots. */
    interval: {
        backgroundImage: {
            default: [
                // mark the rails where the closing rule and both band rules cross them
                MARK_ARMS,
                MARK_ARMS,
                MARK_ARMS,
                MARK_STEMS,
                MARK_STEMS,
                MARK_STEMS,
                // rule the close and both band edges, and run the rails through
                `linear-gradient(${RULE}, ${RULE})`,
                `linear-gradient(${RULE}, ${RULE})`,
                `linear-gradient(${RULE}, ${RULE})`,
                `linear-gradient(to right, ${RULE} ${stroke.border}, transparent 0 calc(100% - ${stroke.border}), ${RULE} 0)`,
                // measure the band's top rule like a ruler, by column and quarter column
                TICKS.column,
                TICKS.quarter,
            ].join(", "),
            [media.maxMd]: [
                `linear-gradient(${RULE}, ${RULE})`,
                `linear-gradient(${RULE}, ${RULE})`,
                `linear-gradient(${RULE}, ${RULE})`,
            ].join(", "),
        },
        backgroundPosition: {
            default: [
                `center top ${EDGE}`,
                `center top calc(${EDGE} + ${GAP})`,
                `center bottom ${EDGE}`,
                "center top",
                `center top ${GAP}`,
                "center bottom",
                `center top ${EDGE}`,
                `center top calc(${EDGE} + ${GAP})`,
                `center bottom ${EDGE}`,
                "center",
                `center top calc(${EDGE} + ${GAP} + ${stroke.border})`,
                `center top calc(${EDGE} + ${GAP} + ${stroke.border})`,
            ].join(", "),
            [media.maxMd]: [
                `center top ${EDGE}`,
                `center top calc(${EDGE} + ${GAP})`,
                `center bottom ${EDGE}`,
            ].join(", "),
        },
        backgroundRepeat: "no-repeat",
        backgroundSize: {
            default: [
                `${ARMS_WIDTH} ${stroke.border}`,
                `${ARMS_WIDTH} ${stroke.border}`,
                `${ARMS_WIDTH} ${stroke.border}`,
                `${frame.width} ${MARK}`,
                `${frame.width} ${MARK}`,
                `${frame.width} ${MARK}`,
                `100% ${stroke.border}`,
                `100% ${stroke.border}`,
                `100% ${stroke.border}`,
                `${frame.width} 100%`,
                `${frame.width} 8px`,
                `${frame.width} 4px`,
            ].join(", "),
            [media.maxMd]: [
                `100% ${stroke.border}`,
                `100% ${stroke.border}`,
                `100% ${stroke.border}`,
            ].join(", "),
        },
        height: `calc(${GAP} + ${BAND} + 2 * ${EDGE})`,
        marginBlock: `calc(-1 * ${EDGE})`,
        position: "relative",
        "::before": {
            backgroundImage: DOTS,
            backgroundPosition: "center",
            backgroundSize: `${DOT_GRID} ${DOT_GRID}`,
            bottom: `calc(${EDGE} + ${stroke.border})`,
            content: '""',
            height: `calc(${BAND} - 2 * ${stroke.border})`,
            insetInline: 0,
            pointerEvents: "none",
            position: "absolute",
        },
    },

    /** The running head of an interval, set on the frame's width over the dots. */
    head: {
        alignItems: "center",
        bottom: EDGE,
        display: "flex",
        height: BAND,
        insetInline: 0,
        justifyContent: "space-between",
        marginInline: "auto",
        maxWidth: frame.width,
        paddingInline: frame.inset,
        position: "absolute",
    },

    /** A guide word of a running head, in small capitals on a patch of page. */
    guide: {
        backgroundColor: color.background,
        color: color.mutedForeground,
        fontFamily: font.code,
        fontSize: "0.6875rem",
        letterSpacing: "0.1em",
        lineHeight: "1.5rem",
        paddingInline: "0.625rem",
        textTransform: "uppercase",
        whiteSpace: "nowrap",
    },

    /** The headword guide of a running head, dropped on phones where the entry repeats it. */
    guideWord: {
        display: { default: "inline", [screen.phone]: "none", [screen.smallPhone]: "none" },
    },

    /** The section number of a running head, in the signal orange. */
    number: {
        color: color.primary,
        fontWeight: 600,
    },

    /** A grid coordinate beside a rail at a running head's top rule, on wide windows only. */
    coordinate: {
        color: MARK_INK,
        display: { default: "none", "@media (min-width: 1360px)": "block" },
        fontFamily: font.code,
        fontSize: "0.625rem",
        letterSpacing: "0.1em",
        position: "absolute",
        top: 0,
        transform: "translateY(-50%)",
    },

    /** A coordinate left of the left rail. */
    coordinateStart: {
        right: "calc(100% + 0.875rem)",
    },

    /** A coordinate right of the right rail. */
    coordinateEnd: {
        left: "calc(100% + 0.875rem)",
    },

    /** A row of crossing marks where a full-width edge meets both rails, laid over the edge. */
    crossings: {
        backgroundImage: `${MARK_ARMS}, ${MARK_STEMS}`,
        backgroundPosition: "center",
        backgroundRepeat: "no-repeat",
        backgroundSize: `${ARMS_WIDTH} ${stroke.border}, ${frame.width} ${MARK}`,
        display: { default: "block", [media.maxMd]: "none" },
        height: MARK,
        insetInline: 0,
        pointerEvents: "none",
        position: "absolute",
        zIndex: 2,
    },

    /** Crossing marks on a band's bottom edge. */
    crossingsBottom: {
        bottom: `calc(-1 * ${EDGE})`,
    },

    /** Crossing marks on a band's top edge. */
    crossingsTop: {
        top: `calc(-1 * ${EDGE})`,
    },

    /** A rule along the right edge of a cell. */
    ruleRight: {
        borderRightColor: RULE,
        borderRightStyle: "solid",
        borderRightWidth: stroke.border,
    },

    /** A rule along the bottom edge of a cell or band, run across the window. */
    ruleBottom: {
        borderBottomColor: RULE,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        position: "relative",
        "::after": {
            backgroundColor: RULE,
            bottom: `calc(-1 * ${stroke.border})`,
            content: '""',
            height: stroke.border,
            left: "50%",
            pointerEvents: "none",
            position: "absolute",
            transform: "translateX(-50%)",
            width: "200vw",
        },
    },
});
