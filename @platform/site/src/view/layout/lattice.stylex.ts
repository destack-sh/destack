import { frame } from "./frame.stylex";
import { color, stroke } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { screen } from "./screen.stylex";

/** The twelve-column frame and the rules drawn between its cells. */
export const lattice = style.create({
    /** A full-width band on the shared columns, closed by the frame edges. */
    frame: {
        borderInlineColor: color.border,
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
        width: "100%",
    },

    /** A homepage section one screen tall on desktop, so every section shares one height. */
    section: {
        height: { default: frame.section, [screen.belowDesktop]: "auto" },
    },

    /** A band whose cells sit on the rule colour a hairline apart, so every rule between them is drawn once and meets its neighbours, the colour kept inside the band's own edge rules so those are drawn once too. */
    ruled: {
        backgroundClip: "padding-box",
        backgroundColor: color.border,
        gap: stroke.border,
    },

    /** A cell of a ruled band, painted in the page colour over the rules. */
    cell: {
        backgroundColor: color.background,
        minWidth: 0,
    },

    /** A figure cell of a ruled band: a label, what it shows, and the terms under it, inset alike in every band. */
    figureCell: {
        display: "grid",
        gap: "0.75rem",
        gridTemplateRows: "auto minmax(0, 1fr) auto",
        minHeight: 0,
        padding: `1.5rem ${frame.inset}`,
    },

    /** An empty band between two sections, closed by the frame edges so the frame runs unbroken down the page. */
    interval: {
        height: { default: "6rem", [media.maxMd]: "3rem" },
    },

    /** A rule along the right edge of a cell. */
    ruleRight: {
        borderRightColor: color.border,
        borderRightStyle: "solid",
        borderRightWidth: stroke.border,
    },

    /** A rule along the bottom edge of a cell or band. */
    ruleBottom: {
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
    },
});
