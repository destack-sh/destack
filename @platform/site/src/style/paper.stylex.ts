import * as stylex from "@destack/style";

import { tokens } from "./tokens.stylex";

/** The cream paper of the drawings: cards with an ink rim and offset shadow, and the keys pressed on the page. */
export const paper = stylex.create({
    /** A cream card with an ink rim and an offset shadow. */
    card: {
        backgroundColor: tokens.cream,
        borderColor: tokens.signalInk,
        borderStyle: "solid",
        borderWidth: "2px",
        boxShadow: `4px 4px 0 var(--card-shadow, ${tokens.signalInk})`,
        color: tokens.signalInk,
    },

    /** A key: a card-like button with a small offset shadow that lifts on hover. */
    key: {
        alignItems: "center",
        backgroundColor: tokens.cream,
        borderColor: tokens.signalInk,
        borderStyle: "solid",
        borderWidth: "2px",
        boxShadow: {
            default: `3px 3px 0 var(--card-shadow, ${tokens.signalInk})`,
            ":hover": `4px 4px 0 var(--card-shadow, ${tokens.signalInk})`,
        },
        color: tokens.signalInk,
        cursor: "pointer",
        display: "inline-flex",
        fontFamily: "inherit",
        fontSize: "1rem",
        fontWeight: 600,
        height: "2.875rem",
        justifyContent: "center",
        paddingInline: "1.25rem",
        textDecoration: "none",
        transform: { default: "none", ":hover": "translate(-1px, -1px)" },
        transitionDuration: "80ms",
        transitionProperty: "transform, box-shadow",
        whiteSpace: "nowrap",
    },

    /** The signal fill of the primary key. */
    primary: {
        backgroundColor: tokens.signal,
    },

    /** A smaller key for actions inside a cell. */
    small: {
        fontSize: "0.875rem",
        height: "2.125rem",
        paddingInline: "0.875rem",
    },
});
