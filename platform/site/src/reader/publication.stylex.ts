import { color, fontFamily } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";

import { tokens } from "../style/tokens.stylex";

const narrow = "@media (width < 80rem)";
const mobile = "@media (max-width: 767px)";

/** Shared typography for articles, chapters, and their navigation. */
export const publicationStyles = stylex.create({
    layout: {
        display: "grid",
        fontFamily: fontFamily.default,
        fontSize: "var(--size-body)",
        columnGap: "2.5rem",
        gridTemplateColumns: "16rem minmax(0, 1fr)",
        marginInline: "auto",
        maxWidth: tokens.siteWidth,
        padding: `0 ${tokens.gutterRight} 3rem ${tokens.gutterLeft}`,
        width: "100%",
        [narrow]: {
            display: "block",
            maxWidth: tokens.siteWidth,
            padding: `0 ${tokens.gutterRight} 3rem ${tokens.gutterLeft}`,
        },
        [mobile]: {
            paddingTop: 0,
            paddingBottom: "2rem",
        },
    },
    article: {
        alignContent: "start",
        color: color.foreground,
        display: "grid",
        gridColumn: 2,
        minWidth: 0,
        width: "100%",
    },
    sidebar: {
        alignSelf: "start",
        display: "none",
        fontFamily: fontFamily.default,
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
        color: color.foreground,
        fontWeight: 600,
        textDecorationLine: "underline",
        textDecorationColor: color.primary,
        textDecorationThickness: "1px",
        textUnderlineOffset: "0.3em",
    },
    collectionLink: {
        color: color.mutedForeground,
        display: "block",
        fontSize: "var(--size-navigation)",
        lineHeight: 1.3,
        paddingBlock: "0.25rem",
        ":hover": {
            color: color.primary,
        },
    },
    collectionList: {
        display: "grid",
        gap: 0,
        listStyle: "none",
        margin: 0,
        padding: "var(--content-section-gap) 0 0",
    },
});
