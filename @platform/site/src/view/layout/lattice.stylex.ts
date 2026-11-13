import { measure } from "./measure.stylex";
import { frame } from "@destack/style/frame.stylex";
import { color, font, stroke } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { screen } from "./screen.stylex";

/** The rule colour: a faint, opaque tint of the text, so rules that cross or overlap stay even. */
const RULE = `color-mix(in srgb, ${color.foreground} 10%, ${color.background})`;

/** The ink of the rails, a little stronger than the rules so their dash and dot still read. */
const RAIL_INK = `color-mix(in srgb, ${color.foreground} 20%, ${color.background})`;

/** A rail as a drawing's centre line: a long dash, a gap, a dot, a gap, down the page. */
const CHAIN = `repeating-linear-gradient(to bottom, ${RAIL_INK} 0 18px, transparent 0 22px, ${RAIL_INK} 0 24px, transparent 0 28px)`;

/** The two hairlines edging a band, eight pixels apart, the band's height. */
const BAND_EDGES = `linear-gradient(to bottom, ${RULE} 0 1px, transparent 0 7px, ${RULE} 0 8px, transparent 0)`;

/** The ink of a mark where two rules cross: the signal orange. */
const MARK_INK = color.primary;

/** The length of each arm pair of a crossing mark, odd so it centres on a hairline. */
const MARK = "13px";

/** How far an interval overlaps its neighbours, half a mark less half a hairline. */
const EDGE = "6px";

/** The ink of the hatching in the gap between two sections. */
const HATCH_INK = `color-mix(in srgb, ${color.foreground} 14%, ${color.background})`;

/** The hatching of the gap between two sections, like the cut face in a section drawing. */
const HATCH = `repeating-linear-gradient(-45deg, ${HATCH_INK} 0 ${stroke.border}, transparent 0 6px)`;

/** The spacing of the dot grid. */
const DOT_GRID = "16px";

/** The spacing of the margins' dot grid: every other dot of the page's, so the two stay aligned. */
const MARGIN_DOT_GRID = "32px";

/** The ink of a ruler's ticks along a running head's top rule. */
const TICK_INK = `color-mix(in srgb, ${color.foreground} 32%, ${color.background})`;

/** A ruler's ticks across the frame: one per column, and quarter columns between. */
const TICKS = {
    column: `repeating-linear-gradient(to right, ${TICK_INK} 0 ${stroke.border}, transparent 0 calc(${measure.width} / ${frame.columns}))`,
    quarter: `repeating-linear-gradient(to right, ${TICK_INK} 0 ${stroke.border}, transparent 0 calc(${measure.width} / ${frame.columns} / 4))`,
};

/** A crop mark's stroke, in the ruler's ink. */
const CROP_LINE = `linear-gradient(${TICK_INK}, ${TICK_INK})`;

/** The length of a crop mark at a figure's corner. */
const CROP = "9px";

/** How far a crop mark's frame reaches past the figure, its mark plus a gap of three pixels. */
const CROP_GAP = "12px";

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

/** A crossing mark's ring around a rail, centred on its arms like a registration mark. */
const RING_START = `radial-gradient(circle at calc(${MARK} / 2) 50%, transparent 3px, ${MARK_INK} 3.5px 4.25px, transparent 4.75px)`;

/** The ring of the crossing mark on the right rail. */
const RING_END = `radial-gradient(circle at calc(100% - ${MARK} / 2) 50%, transparent 3px, ${MARK_INK} 3.5px 4.25px, transparent 4.75px)`;

/** The width of a row of crossing marks' arms, reaching half an arm past each rail. */
const ARMS_WIDTH = `calc(${measure.width} + ${MARK} - ${stroke.border})`;

/** The width of each margin beside the frame. */
const MARGIN = `calc((100% - ${measure.width}) / 2)`;

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

/** The rules other site styles draw: the hairline colour, the hatched band and the stave. */
export const rule = style.defineConsts({
    /** The colour of every grid rule. */
    color: "color-mix(in srgb, var(--destack-color-foreground) 10%, var(--destack-color-background))",
    /** A heavy rule as a hatched band, drawn across a strip at the band's height. */
    band: "repeating-linear-gradient(-45deg, color-mix(in srgb, var(--destack-color-foreground) 32%, transparent) 0 1.5px, transparent 0 5px)",
    /** The height of a hatched band. */
    bandHeight: "8px",
    /** A stack of hairlines spreading apart downwards, as layers come off a stack. */
    stave: "linear-gradient(to bottom, color-mix(in srgb, var(--destack-color-foreground) 24%, transparent) 0 1px, transparent 0 3px, color-mix(in srgb, var(--destack-color-foreground) 24%, transparent) 0 4px, transparent 0 8px, color-mix(in srgb, var(--destack-color-foreground) 24%, transparent) 0 9px, transparent 0 15px, color-mix(in srgb, var(--destack-color-foreground) 24%, transparent) 0 16px, transparent 0 24px, color-mix(in srgb, var(--destack-color-foreground) 24%, transparent) 0 25px, transparent 0)",
    /** The height of a stave, its last hairline at the bottom. */
    staveHeight: "25px",
});

/** The twelve-column frame and the rules drawn between its cells. */
export const lattice = style.create({
    /** A full-width band on the shared columns, closed by the frame edges. */
    frame: {
        borderImageSlice: 1,
        borderImageSource: CHAIN,
        borderInlineStyle: "solid",
        borderInlineWidth: { default: stroke.border, [media.maxMd]: 0 },
        display: "grid",
        gridTemplateColumns: `repeat(${frame.columns}, minmax(0, 1fr))`,
        marginInline: "auto",
        maxWidth: measure.width,
        minWidth: 0,
        position: "relative",
        width: "100%",
    },

    /** A homepage section one screen tall on desktop, so every section shares one height. */
    section: {
        height: { default: measure.section, [media.maxLg]: "auto" },
    },

    /** A band whose cells sit a hatched band apart. */
    ruled: {
        backgroundClip: "padding-box",
        backgroundImage: rule.band,
        gap: rule.bandHeight,
    },

    /** A ruled band's cell in the page colour, its band's hairlines run across the window. */
    cell: {
        backgroundColor: surface.cell,
        backgroundImage: GRAIN,
        backgroundSize: GRAIN_SIZE,
        borderImageOutset: `${rule.bandHeight} 100vw 0`,
        borderImageSlice: "8 0 0 0",
        // edge the band with hairlines that run on across the margins, the hatch staying in the frame
        borderImageSource: { default: BAND_EDGES, ":first-child": "none" },
        borderImageWidth: `${rule.bandHeight} 0 0 0`,
        minWidth: 0,
    },

    /** A band along the right edge of a block, dividing it from the block beside it. */
    bandRight: {
        backgroundImage: rule.band,
        backgroundPosition: "right top",
        backgroundRepeat: "no-repeat",
        backgroundSize: `${rule.bandHeight} 100%`,
        paddingInlineEnd: rule.bandHeight,
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
        padding: `1.5rem ${measure.inset}`,
    },

    /** The space between two sections: a closing rule, a blank gap, then the next head on dots. */
    interval: {
        backgroundImage: {
            default: [
                // mark the rails where the closing rule and both band rules cross them
                RING_START,
                RING_END,
                RING_START,
                RING_END,
                RING_START,
                RING_END,
                MARK_ARMS,
                MARK_ARMS,
                MARK_ARMS,
                MARK_STEMS,
                MARK_STEMS,
                MARK_STEMS,
                // rule the close and both band edges
                `linear-gradient(${RULE}, ${RULE})`,
                `linear-gradient(${RULE}, ${RULE})`,
                `linear-gradient(${RULE}, ${RULE})`,
                // hatch the gap between the close and the band
                HATCH,
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
                "center top",
                "center top",
                `center top ${GAP}`,
                `center top ${GAP}`,
                "center bottom",
                "center bottom",
                `center top ${EDGE}`,
                `center top calc(${EDGE} + ${GAP})`,
                `center bottom ${EDGE}`,
                "center top",
                `center top ${GAP}`,
                "center bottom",
                `center top ${EDGE}`,
                `center top calc(${EDGE} + ${GAP})`,
                `center bottom ${EDGE}`,
                `center top calc(${EDGE} + ${stroke.border})`,
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
                `${ARMS_WIDTH} ${MARK}`,
                `${ARMS_WIDTH} ${MARK}`,
                `${ARMS_WIDTH} ${MARK}`,
                `${ARMS_WIDTH} ${MARK}`,
                `${ARMS_WIDTH} ${MARK}`,
                `${ARMS_WIDTH} ${MARK}`,
                `${ARMS_WIDTH} ${stroke.border}`,
                `${ARMS_WIDTH} ${stroke.border}`,
                `${ARMS_WIDTH} ${stroke.border}`,
                `${measure.width} ${MARK}`,
                `${measure.width} ${MARK}`,
                `${measure.width} ${MARK}`,
                `100% ${stroke.border}`,
                `100% ${stroke.border}`,
                `100% ${stroke.border}`,
                `calc(${measure.width} - 2 * ${stroke.border}) calc(${GAP} - ${stroke.border})`,
                `${measure.width} 8px`,
                `${measure.width} 4px`,
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
        // lay the rail marks over the sections the interval overlaps
        zIndex: 2,
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
        // run the rails through the gap as centre lines
        "::after": {
            borderBlockWidth: 0,
            borderImageSlice: 1,
            borderImageSource: CHAIN,
            borderInlineStyle: "solid",
            borderInlineWidth: stroke.border,
            boxSizing: "border-box",
            content: '""',
            display: { default: "block", [media.maxMd]: "none" },
            insetBlock: 0,
            left: "50%",
            pointerEvents: "none",
            position: "absolute",
            transform: "translateX(-50%)",
            width: measure.width,
        },
    },

    /** The section cut of an interval: a hatched block and a letter at each rail. */
    cut: {
        alignItems: "center",
        bottom: `calc(${EDGE} + ${BAND})`,
        color: MARK_INK,
        columnGap: "0.5rem",
        display: { default: "none", "@media (min-width: 1360px)": "flex" },
        fontFamily: font.code,
        fontSize: "0.75rem",
        fontWeight: 600,
        letterSpacing: "0.1em",
        pointerEvents: "none",
        position: "absolute",
        // centre the cut on the band's top rule
        transform: "translateY(50%)",
    },

    /** The cut left of the left rail, its stroke reaching for the rail. */
    cutStart: {
        right: `calc(50% + ${measure.width} / 2 + 8px)`,
    },

    /** The cut right of the right rail, drawn as the left one's mirror. */
    cutEnd: {
        flexDirection: "row-reverse",
        left: `calc(50% + ${measure.width} / 2 + 8px)`,
    },

    /** The end of a cut line: a short hatched block in the signal orange. */
    cutStroke: {
        backgroundImage: `repeating-linear-gradient(-45deg, currentColor 0 1.5px, transparent 0 4px)`,
        borderColor: "currentColor",
        borderStyle: "solid",
        borderWidth: stroke.border,
        height: "7px",
        width: "1.5rem",
    },

    /** A stave along the top of a band, its hairlines spreading apart into the band. */
    stave: {
        backgroundImage: rule.stave,
        backgroundRepeat: "no-repeat",
        backgroundSize: `100% ${rule.staveHeight}`,
        height: rule.staveHeight,
        insetInline: 0,
        pointerEvents: "none",
        position: "absolute",
        top: 0,
    },

    /** The zone strip along a sheet's edge: the twelve columns numbered over a ruler. */
    zones: {
        backgroundImage: {
            default: `${TICKS.column}, ${TICKS.quarter}, ${rule.band}`,
            [media.maxMd]: "none",
        },
        backgroundPosition: `left bottom ${rule.bandHeight}, left bottom ${rule.bandHeight}, left bottom`,
        backgroundRepeat: "no-repeat",
        backgroundSize: `100% 8px, 100% 4px, 100% ${rule.bandHeight}`,
        display: { default: "grid", [media.maxMd]: "none" },
        gridTemplateColumns: `repeat(${frame.columns}, minmax(0, 1fr))`,
        height: "1.75rem",
    },

    /** A zone strip under a sheet's bottom edge, its ruler hung from the top. */
    zonesBottom: {
        backgroundPosition: `left top ${rule.bandHeight}, left top ${rule.bandHeight}, left top`,
    },

    /** A zone's number, centred in its column. */
    zone: {
        alignSelf: "center",
        color: TICK_INK,
        fontFamily: font.code,
        fontSize: "0.625rem",
        letterSpacing: "0.1em",
        textAlign: "center",
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
        maxWidth: measure.width,
        paddingInline: measure.inset,
        position: "absolute",
    },

    /** A guide word of a running head, in small capitals over the dots. */
    guide: {
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

    /** A row of crossing marks where a full-width edge meets both rails, laid over the edge. */
    crossings: {
        backgroundImage: `${RING_START}, ${RING_END}, ${MARK_ARMS}, ${MARK_STEMS}`,
        backgroundPosition: "center",
        backgroundRepeat: "no-repeat",
        backgroundSize: `${ARMS_WIDTH} ${MARK}, ${ARMS_WIDTH} ${MARK}, ${ARMS_WIDTH} ${stroke.border}, ${measure.width} ${MARK}`,
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

    /** A figure's frame with crop marks at its corners, as a print sheet trims its plates. */
    crop: {
        position: "relative",
        "::before": {
            backgroundImage: [
                CROP_LINE,
                CROP_LINE,
                CROP_LINE,
                CROP_LINE,
                CROP_LINE,
                CROP_LINE,
                CROP_LINE,
                CROP_LINE,
            ].join(", "),
            backgroundPosition: [
                `left 0 top ${CROP_GAP}`,
                `left ${CROP_GAP} top 0`,
                `right 0 top ${CROP_GAP}`,
                `right ${CROP_GAP} top 0`,
                `left 0 bottom ${CROP_GAP}`,
                `left ${CROP_GAP} bottom 0`,
                `right 0 bottom ${CROP_GAP}`,
                `right ${CROP_GAP} bottom 0`,
            ].join(", "),
            backgroundRepeat: "no-repeat",
            backgroundSize: [
                `${CROP} ${stroke.border}`,
                `${stroke.border} ${CROP}`,
                `${CROP} ${stroke.border}`,
                `${stroke.border} ${CROP}`,
                `${CROP} ${stroke.border}`,
                `${stroke.border} ${CROP}`,
                `${CROP} ${stroke.border}`,
                `${stroke.border} ${CROP}`,
            ].join(", "),
            content: '""',
            inset: `calc(-1 * ${CROP_GAP})`,
            pointerEvents: "none",
            position: "absolute",
        },
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
