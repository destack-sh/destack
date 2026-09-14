import * as stylex from "@stylexjs/stylex";

import { tokens } from "../style/tokens.stylex";

const narrow = "@media (width < 80rem)";
const mobile = "@media (max-width: 767px)";

/** Shared typography for articles, chapters, and their navigation. */
export const publicationStyles = stylex.create({
    layout: {
        display: "grid",
        fontFamily: tokens.textFont,
        fontSize: "var(--size-body)",
        columnGap: "2.5rem",
        gridTemplateColumns: "16rem minmax(0, 1fr)",
        marginInline: "auto",
        maxWidth: tokens.siteWidth,
        padding: `1.5rem ${tokens.gutterRight} 4rem ${tokens.gutterLeft}`,
        width: "100%",
        [narrow]: {
            display: "block",
            maxWidth: "48rem",
            padding: `1rem ${tokens.gutterRight} 3rem ${tokens.gutterLeft}`,
        },
        [mobile]: {
            paddingTop: "0.75rem",
            paddingBottom: "2rem",
        },
    },
    article: {
        alignContent: "start",
        color: tokens.ink,
        display: "grid",
        gridColumn: 2,
        minWidth: 0,
        width: "100%",
    },
    sidebar: {
        alignSelf: "start",
        display: "none",
        fontFamily: tokens.textFont,
        fontSize: "var(--size-navigation)",
        gridColumn: 1,
        "@media (min-width: 80rem)": {
            display: "block",
            position: "sticky",
            top: "1.5rem",
            maxHeight: "calc(100svh - 3rem)",
            overflowY: "auto",
            overscrollBehaviorY: "contain",
            scrollbarWidth: "thin",
        },
    },
    active: {
        color: tokens.ink,
        fontWeight: 600,
        textDecorationLine: "underline",
        textDecorationColor: tokens.accent,
        textDecorationThickness: "1px",
        textUnderlineOffset: "0.3em",
    },
    header: {
        display: "grid",
        gap: "0.75rem",
        paddingBlock: "2.5rem 0",
        "@media (max-width: 767px)": {
            paddingBlock: "1rem 0",
        },
    },
    title: {
        color: tokens.ink,
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
        maxWidth: "var(--width-prose)",
    },
    collectionTitle: {
        textTransform: "lowercase",
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
