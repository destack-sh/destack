import * as stylex from "@stylexjs/stylex";

import { tokens } from "../style/tokens.stylex";

/** Shared typography for articles, chapters, and their navigation. */
export const publicationStyles = stylex.create({
    active: {
        color: tokens.ink,
        fontWeight: 600,
        textDecorationLine: "underline",
        textDecorationColor: tokens.accent,
        textDecorationThickness: "1px",
        textUnderlineOffset: "0.3em",
    },
    header: {
        borderBottomColor: tokens.line,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        display: "grid",
        gap: "0.75rem",
        paddingBlock: "1.5rem 2rem",
        "@media (max-width: 767px)": {
            paddingBlock: "1rem 1.25rem",
        },
    },
    title: {
        fontFamily: tokens.displayFont,
        fontSize: "var(--size-page-title)",
        fontWeight: 500,
        letterSpacing: "-0.015em",
        lineHeight: 1.08,
        margin: 0,
        overflowWrap: "anywhere",
    },
    description: {
        color: tokens.ink,
        fontFamily: tokens.textFont,
        fontSize: "var(--size-page-description)",
        lineHeight: 1.5,
        margin: 0,
        maxWidth: "42rem",
    },
    collectionTitle: {
        alignItems: "center",
        borderBottomColor: tokens.line,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        color: tokens.ink,
        display: "flex",
        fontFamily: tokens.textFont,
        fontSize: "var(--size-navigation)",
        fontWeight: 600,
        minHeight: tokens.publicationRow,
        ":hover": {
            color: tokens.accent,
        },
    },
    collectionList: {
        display: "grid",
        gap: 0,
        listStyle: "none",
        margin: 0,
        padding: "1.25rem 0 0",
    },
});
