import { color } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";

import { tokens } from "./tokens.stylex";

/** The media query for phone-width screens. */
const mobile = "@media (max-width: 767px)";

/** The twelve-column frame and the rules drawn between its cells. */
export const lattice = stylex.create({
    /** A full-width band on the shared columns, closed by the frame edges. */
    frame: {
        borderInlineColor: color.border,
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

    /** A rule along the right edge of a cell. */
    ruleRight: {
        borderRightColor: color.border,
        borderRightStyle: "solid",
        borderRightWidth: tokens.hairline,
    },

    /** A rule along the bottom edge of a cell or band. */
    ruleBottom: {
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
    },
});
