import { color } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";

import { tokens } from "./tokens.stylex";

/** The media query for phone-width screens. */
const mobile = "@media (max-width: 767px)";

/** The twelve-column frame and the rules drawn between its cells. */
export const lattice = stylex.create({
    /** A full-width band on the shared columns, closed by the frame edges. */
    frame: {
        borderInlineColor: tokens.rule,
        borderInlineStyle: "solid",
        borderInlineWidth: tokens.hairline,
        display: "grid",
        gridTemplateColumns: "repeat(12, minmax(0, 1fr))",
        marginInline: "auto",
        maxWidth: tokens.siteWidth,
        minWidth: 0,
        width: "100%",
        [mobile]: {
            borderInlineWidth: 0,
            gridTemplateColumns: "repeat(4, minmax(0, 1fr))",
        },
    },

    /** A homepage section one screen tall on desktop, so every section shares one height. */
    section: {
        height: { default: tokens.section, "@media (max-width: 1099px)": "auto" },
    },

    /** A band whose cells sit on the rule colour a hairline apart, so every rule between them is drawn once and meets its neighbours, the colour kept inside the band's own edge rules so those are drawn once too. */
    ruled: {
        backgroundClip: "padding-box",
        backgroundColor: tokens.rule,
        gap: tokens.hairline,
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
        padding: `1.5rem ${tokens.inset}`,
    },

    /** An empty band between two sections, closed by the frame edges so the frame runs unbroken down the page. */
    interval: {
        height: { default: "6rem", [mobile]: "3rem" },
    },

    /** A rule along the right edge of a cell. */
    ruleRight: {
        borderRightColor: tokens.rule,
        borderRightStyle: "solid",
        borderRightWidth: tokens.hairline,
    },

    /** A rule along the bottom edge of a cell or band. */
    ruleBottom: {
        borderBottomColor: tokens.rule,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
    },
});
