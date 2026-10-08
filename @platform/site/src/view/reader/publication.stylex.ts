import { frame } from "../layout/frame.stylex";
import { media } from "@destack/style/media.stylex";
import { color, font, stroke, weight } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";

/** The site's reading size, 17px, a step above the theme's 16px body for long articles. */
const READING_SIZE = "1.0625rem";

/** The leading of the site's articles. */
const READING_LEADING = 1.7;

/** The media query for narrow screens that stack the sidebar. */
const narrow = "@media (width < 60rem)";

/** Shared layout and navigation styles for articles, chapters, and directories. */
export const publicationStyles = style.create({
    layout: {
        flexGrow: 1,
        fontFamily: font.text,
    },
    sidebar: {
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
        justifySelf: "start",
        maxWidth: `calc(44rem + ${frame.inset} * 2)`,
        minWidth: 0,
        width: "100%",
        paddingBottom: "4rem",
        paddingInline: frame.inset,
    },
    reading: {
        fontSize: READING_SIZE,
        lineHeight: READING_LEADING,
    },
    prose: {
        maxInlineSize: "none",
        paddingTop: "1.25rem",
    },
    intro: {
        maxInlineSize: "none",
        paddingBottom: "1.5rem",
    },
    outline: {
        color: color.foreground,
    },
    context: {
        alignItems: "center",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        display: "flex",
        gap: "1rem",
        height: frame.bar,
        justifyContent: "space-between",
        paddingInline: frame.inset,
    },
    contextTitle: {
        color: {
            default: color.foreground,
            ":hover": { default: null, [media.hover]: color.primary },
        },
        fontWeight: weight.semibold,
    },
    contextBack: {
        color: {
            default: color.mutedForeground,
            ":hover": { default: null, [media.hover]: color.primary },
        },
    },
    collectionList: {
        display: "grid",
        listStyle: "none",
        margin: 0,
        padding: `2.5rem ${frame.inset}`,
    },
    collectionLink: {
        color: {
            default: color.mutedForeground,
            ":hover": { default: null, [media.hover]: color.primary },
        },
        display: "block",
        lineHeight: 1.3,
        paddingBlock: "0.25rem",
    },
    active: {
        color: color.foreground,
        fontWeight: weight.semibold,
    },
    pagination: {
        alignItems: "center",
        borderTopColor: color.border,
        borderTopStyle: "solid",
        borderTopWidth: stroke.border,
        display: "flex",
        flexWrap: "wrap",
        fontWeight: weight.semibold,
        rowGap: "1rem",
        columnGap: "2rem",
        justifyContent: "space-between",
        minHeight: frame.column,
        paddingInline: frame.inset,
    },
    paginationLink: {
        color: {
            default: color.foreground,
            ":hover": { default: null, [media.hover]: color.primary },
        },
    },
});
