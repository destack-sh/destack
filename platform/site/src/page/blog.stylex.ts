import * as stylex from "@stylexjs/stylex";

import { tokens } from "../style/tokens.stylex";

/// Blog archive styles.
export const styles = stylex.create({
    archive: {
        listStyle: "none",
        margin: 0,
        padding: 0,
    },
    copy: {
        display: "grid",
        gap: "0.375rem",
        minWidth: 0,
    },
    date: {
        color: tokens.soft,
        fontFamily: tokens.monoFont,
        fontSize: "0.78rem",
        whiteSpace: "nowrap",
    },
    header: {
        alignItems: "baseline",
        borderTopColor: tokens.ink,
        borderTopStyle: "solid",
        borderTopWidth: tokens.stroke,
        display: "flex",
        gap: "1.5rem",
        justifyContent: "space-between",
        paddingBlock: "1rem",
    },
    heading: {
        fontFamily: tokens.monoFont,
        fontSize: "clamp(2rem, 5vw, 2.75rem)",
        fontWeight: 500,
        letterSpacing: "-0.025em",
        lineHeight: 1,
        margin: 0,
    },
    index: {
        fontFamily: tokens.monoFont,
        fontSize: "0.95rem",
        marginInline: "auto",
        maxWidth: tokens.siteWidth,
        padding: `3.5rem ${tokens.gutterRight} 5rem ${tokens.gutterLeft}`,
        width: "100%",
    },
    postCount: {
        color: tokens.soft,
        fontFamily: tokens.monoFont,
        fontSize: "0.78rem",
    },
    postLink: {
        alignItems: "baseline",
        display: "grid",
        gap: "1.5rem",
        gridTemplateColumns: "10rem minmax(0, 1fr)",
        paddingBlock: "1.25rem",
        "@media (max-width: 640px)": {
            gap: "0.5rem",
            gridTemplateColumns: "minmax(0, 1fr)",
        },
    },
    postRow: {
        borderTopColor: tokens.line,
        borderTopStyle: "solid",
        borderTopWidth: tokens.stroke,
    },
    postRowFirst: {
        borderTopWidth: 0,
    },
    subtitle: {
        color: tokens.soft,
        lineHeight: 1.5,
    },
    title: {
        fontFamily: tokens.monoFont,
        fontSize: "1.1rem",
        fontWeight: 600,
        lineHeight: 1.25,
    },
});
