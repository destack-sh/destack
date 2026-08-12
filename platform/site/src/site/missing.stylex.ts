import * as stylex from "@stylexjs/stylex";

import { tokens } from "../style/tokens.stylex";

/// Missing-page styles.
export const styles = stylex.create({
    action: {
        alignItems: "center",
        backgroundColor: tokens.orange,
        borderColor: tokens.ink,
        borderRadius: tokens.panelRadius,
        borderStyle: "solid",
        borderWidth: tokens.stroke,
        display: "inline-flex",
        fontFamily: tokens.monoFont,
        fontWeight: 700,
        padding: "0.45rem 1.15rem",
        width: "max-content",
        ":hover": {
            color: tokens.cream,
        },
    },
    label: {
        color: tokens.soft,
        fontFamily: tokens.monoFont,
        fontSize: "0.78rem",
        margin: 0,
    },
    page: {
        alignContent: "center",
        display: "grid",
        gap: "1.25rem",
        marginInline: "auto",
        maxWidth: "48rem",
        padding: `4rem ${tokens.gutterRight} 5rem ${tokens.gutterLeft}`,
        width: "100%",
    },
    title: {
        fontFamily: tokens.monoFont,
        fontSize: "clamp(2.2rem, 5vw, 3.25rem)",
        fontWeight: 700,
        letterSpacing: "-0.04em",
        lineHeight: 1,
        margin: 0,
    },
});
