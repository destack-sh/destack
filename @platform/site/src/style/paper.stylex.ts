import { color } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";

import { tokens } from "./tokens.stylex";

/** The flat surfaces of the figures: hairline cards and the keys pressed on the page. */
export const paper = stylex.create({
    /** A card with a hairline rim, in the theme's card colour. */
    card: {
        backgroundColor: color.card,
        borderColor: tokens.rule,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        color: color.cardForeground,
    },

    /** A key: a flat button with a hairline rim that darkens on hover. */
    key: {
        alignItems: "center",
        backgroundColor: { default: color.background, ":hover": color.muted },
        borderColor: tokens.rule,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        color: color.foreground,
        cursor: "pointer",
        display: "inline-flex",
        fontFamily: "inherit",
        fontSize: "0.9375rem",
        fontWeight: 600,
        height: "2.5rem",
        justifyContent: "center",
        paddingInline: "1.125rem",
        textDecoration: "none",
        transitionDuration: "120ms",
        transitionProperty: "background-color, border-color",
        whiteSpace: "nowrap",
    },

    /** The signal fill of the primary key. */
    primary: {
        backgroundColor: { default: tokens.signal, ":hover": "#ff8a47" },
        borderColor: tokens.signal,
        color: tokens.signalInk,
    },

    /** A smaller key for actions inside a figure. */
    small: {
        fontSize: "0.8125rem",
        height: "2rem",
        paddingInline: "0.75rem",
    },
});
