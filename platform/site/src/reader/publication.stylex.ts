import { color, fontFamily } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";

import { tokens } from "../style/tokens.stylex";

/** The media query for narrow screens that stack the sidebar. */
const narrow = "@media (width < 60rem)";

/** Shared layout and navigation styles for articles, chapters, and directories. */
export const publicationStyles = stylex.create({
    layout: {
        flexGrow: 1,
        fontFamily: fontFamily.default,
        fontSize: "var(--size-body)",
    },
    sidebar: {
        fontSize: "var(--size-navigation)",
        gridColumn: "1 / span 3",
        [narrow]: { display: "none" },
    },
    sidebarContent: {
        maxHeight: "100svh",
        overflowY: "auto",
        overscrollBehaviorY: "contain",
        position: "sticky",
        scrollbarWidth: "thin",
        top: 0,
    },
    article: {
        alignContent: "start",
        color: color.foreground,
        display: "grid",
        gridColumn: "4 / span 9",
        minWidth: 0,
        [narrow]: { gridColumn: "1 / -1" },
    },
    body: {
        display: "grid",
        justifySelf: "center",
        maxWidth: `calc(44rem + ${tokens.inset} * 2)`,
        minWidth: 0,
        width: "100%",
        paddingBottom: "4rem",
        paddingInline: tokens.inset,
    },
    context: {
        alignItems: "center",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        display: "flex",
        gap: "1rem",
        height: tokens.bar,
        justifyContent: "space-between",
        paddingInline: tokens.inset,
    },
    contextTitle: {
        color: color.foreground,
        fontWeight: 600,
        ":hover": { color: color.primary },
    },
    contextBack: {
        color: color.mutedForeground,
        ":hover": { color: color.primary },
    },
    collectionList: {
        display: "grid",
        listStyle: "none",
        margin: 0,
        padding: `2.5rem ${tokens.inset}`,
    },
    collectionLink: {
        color: color.mutedForeground,
        display: "block",
        lineHeight: 1.3,
        paddingBlock: "0.25rem",
        ":hover": { color: color.primary },
    },
    active: {
        color: color.foreground,
        fontWeight: 600,
        textDecorationColor: color.primary,
        textDecorationLine: "underline",
        textDecorationThickness: "1px",
        textUnderlineOffset: "0.3em",
    },
    pagination: {
        alignItems: "center",
        borderTopColor: color.border,
        borderTopStyle: "solid",
        borderTopWidth: tokens.hairline,
        display: "flex",
        flexWrap: "wrap",
        fontSize: "var(--size-navigation)",
        fontWeight: 600,
        gap: "1rem 2rem",
        justifyContent: "space-between",
        minHeight: tokens.column,
        paddingInline: tokens.inset,
    },
    paginationLink: {
        color: color.foreground,
        ":hover": { color: color.primary },
    },
});
