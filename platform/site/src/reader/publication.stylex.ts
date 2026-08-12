import * as stylex from "@stylexjs/stylex";

import { tokens } from "../style/tokens.stylex";

const narrow = "@media (width < 60rem)";
const mobile = "@media (max-width: 767px)";
const tiny = "@media (max-width: 360px)";

/// Shared reader styles.
export const readerStyles = stylex.create({
    article: {
        alignContent: "start",
        display: "grid",
        gap: "2.5rem",
        maxWidth: "100%",
        minWidth: 0,
        width: "100%",
        [narrow]: {
            gap: "2rem",
        },
    },
    articlePublication: {
        color: tokens.ink,
        gap: 0,
    },
    location: {
        minWidth: 0,
        [narrow]: {
            gridColumn: "1 / -1",
            gridRow: 2,
        },
    },
    locationBottom: {
        [mobile]: {
            display: "none",
        },
    },
    menu: {
        minWidth: 0,
        "@media (min-width: 60rem)": {
            display: "none",
        },
        [narrow]: {
            gridColumn: 1,
            gridRow: 1,
        },
    },
    menuBody: {
        alignContent: "start",
        display: "grid",
        gap: "2rem",
        maxHeight: "min(32rem, calc(100svh - 10rem))",
        overflowY: "auto",
        padding: "1rem 0 0.5rem",
    },
    menuSummary: {
        alignItems: "center",
        color: tokens.text,
        cursor: "pointer",
        display: "flex",
        fontWeight: 600,
        gap: "0.75rem",
        justifyContent: "flex-start",
        listStyle: "none",
        minHeight: tokens.siteControlHeight,
    },
    reader: {
        display: "grid",
        fontSize: "1rem",
        gridTemplateColumns: "minmax(0, 1fr)",
        justifyContent: "center",
        marginInline: "auto",
        maxWidth: "45rem",
        padding: `3rem ${tokens.gutterRight} 6rem ${tokens.gutterLeft}`,
        width: "100%",
        "@media (min-width: 60rem)": {
            gap: "3rem",
            gridTemplateColumns: "16rem minmax(0, 42rem)",
            justifyContent: "start",
            maxWidth: tokens.siteWidth,
        },
        [narrow]: {
            padding: `1.25rem ${tokens.gutterRight} 4rem ${tokens.gutterLeft}`,
        },
        [mobile]: {
            paddingBottom: "2rem",
        },
    },
    readerPublication: {
        fontFamily: tokens.monoFont,
        fontSize: "0.9rem",
        gap: "clamp(3rem, 6vw, 4rem)",
        gridTemplateColumns: "13rem minmax(0, 46rem)",
        justifyContent: "start",
        maxWidth: tokens.siteWidth,
        paddingTop: "clamp(2rem, 4vw, 3rem)",
        [narrow]: {
            display: "block",
            maxWidth: "46rem",
        },
        "@media (max-width: 600px)": {
            paddingTop: "1rem",
        },
    },
    sidebar: {
        alignContent: "start",
        display: "none",
        fontSize: tokens.siteFontSize,
        gap: "2rem",
        "@media (min-width: 60rem)": {
            display: "grid",
            maxHeight: "calc(100svh - 8rem)",
            overflow: "auto",
            padding: "0 0.25rem 1rem 0",
            position: "sticky",
            top: "5rem",
        },
    },
    toolbar: {
        alignItems: "baseline",
        color: tokens.soft,
        display: "flex",
        flexWrap: "wrap",
        fontSize: tokens.siteFontSize,
        gap: "0.5rem",
        justifyContent: "space-between",
        paddingBottom: "0.75rem",
        [narrow]: {
            alignItems: "start",
            display: "grid",
            gap: "0.25rem 0.75rem",
            gridTemplateColumns: "minmax(0, 1fr) auto",
            justifyContent: "stretch",
        },
    },
    toolbarBottom: {
        display: "none",
        [mobile]: {
            borderTopColor: tokens.line,
            borderTopStyle: "solid",
            borderTopWidth: tokens.stroke,
            display: "grid",
            marginTop: "1rem",
            paddingBottom: 0,
            paddingTop: "0.75rem",
        },
    },
    toolbarPublication: {
        fontSize: "0.72rem",
    },
    toolbarTop: {
        [mobile]: {
            display: "none",
        },
    },
});

/// Breadcrumb styles.
export const breadcrumbStyles = stylex.create({
    link: {
        color: tokens.text,
        fontWeight: 600,
        ":hover": {
            color: tokens.accent,
        },
    },
    root: {
        alignItems: "baseline",
        display: "flex",
        flexWrap: "wrap",
        gap: "0.5rem",
    },
});

/// Article contents styles.
export const contentsStyles = stylex.create({
    active: {
        color: tokens.text,
        fontWeight: 600,
    },
    heading: {
        color: tokens.text,
        fontFamily: tokens.monoFont,
        fontSize: "0.72rem",
        fontWeight: 600,
        letterSpacing: "0.06em",
        margin: "0 0 0.5rem",
        textTransform: "uppercase",
    },
    link: {
        display: "block",
        paddingBlock: "0.3rem",
        ":hover": {
            color: tokens.text,
        },
    },
    list: {
        display: "grid",
        gap: 0,
        listStyle: "none",
        margin: 0,
        padding: 0,
    },
    nested: {
        paddingLeft: "0.75rem",
    },
    root: {
        color: tokens.soft,
        display: "none",
        fontSize: "0.78rem",
        lineHeight: 1.5,
        "@media (min-width: 60rem)": {
            display: "block",
        },
    },
    rootMenu: {
        display: "block",
    },
});

/// Source-control styles.
export const sourceStyles = stylex.create({
    action: {
        backgroundColor: "transparent",
        borderWidth: 0,
        color: tokens.text,
        font: "inherit",
        padding: 0,
        textDecoration: "none",
        ":hover": {
            color: tokens.accent,
        },
    },
    actions: {
        alignItems: "baseline",
        color: tokens.soft,
        display: "flex",
        flexWrap: "wrap",
        fontFamily: tokens.monoFont,
        fontSize: "0.78rem",
        gap: "0.5rem 0.9rem",
        justifyContent: "flex-end",
        [narrow]: {
            display: "none",
        },
    },
    controls: {
        minWidth: 0,
        [narrow]: {
            gridColumn: 2,
            gridRow: 1,
        },
    },
    group: {
        alignItems: "baseline",
        display: "inline-flex",
        gap: "0.5rem",
        whiteSpace: "nowrap",
    },
    label: {
        color: tokens.soft,
    },
    menu: {
        display: "none",
        [narrow]: {
            display: "block",
            position: "relative",
        },
    },
    menuBody: {
        backgroundColor: tokens.page,
        borderColor: tokens.ink,
        borderRadius: tokens.panelRadius,
        borderStyle: "solid",
        borderWidth: tokens.stroke,
        display: "flex",
        flexWrap: "wrap",
        fontFamily: tokens.monoFont,
        fontSize: "0.78rem",
        gap: "0.25rem 0.75rem",
        justifyContent: "flex-end",
        padding: "0.5rem 0.75rem",
        position: "absolute",
        right: 0,
        top: "100%",
        width: `min(42rem, calc(100vw - ${tokens.gutterLeft} - ${tokens.gutterRight}))`,
        zIndex: 4,
    },
    menuSummary: {
        alignItems: "center",
        color: tokens.text,
        cursor: "pointer",
        display: "flex",
        fontFamily: tokens.monoFont,
        fontSize: "0.78rem",
        fontWeight: 600,
        gap: "0.75rem",
        listStyle: "none",
        minHeight: tokens.siteControlHeight,
    },
    menuTokens: {
        color: tokens.soft,
        display: "none",
        minHeight: tokens.siteControlHeight,
        width: "100%",
        [tiny]: {
            alignItems: "center",
            display: "inline-flex",
        },
    },
    tokens: {
        whiteSpace: "nowrap",
        [tiny]: {
            display: "none",
        },
    },
});

/// Manual navigation styles.
export const manualStyles = stylex.create({
    active: {
        color: tokens.ink,
        fontWeight: 600,
    },
    book: {
        alignContent: "start",
        color: tokens.ink,
        display: "grid",
        gap: 0,
    },
    bookLink: {
        color: tokens.soft,
        display: "grid",
        fontSize: "0.78rem",
        gap: "0.4rem",
        gridTemplateColumns: "1.8rem minmax(0, 1fr)",
        lineHeight: 1.3,
        paddingBlock: "0.35rem",
        ":hover": {
            color: tokens.ink,
        },
    },
    bookList: {
        display: "grid",
        gap: 0,
        listStyle: "none",
        margin: 0,
        padding: 0,
    },
    bookNumber: {
        color: tokens.orange,
        fontFamily: tokens.monoFont,
        fontSize: "0.68rem",
    },
    bookTitle: {
        color: tokens.ink,
        fontFamily: tokens.monoFont,
        fontSize: "0.78rem",
        fontWeight: 600,
        letterSpacing: "0.06em",
        paddingBottom: "0.75rem",
        textTransform: "uppercase",
        ":hover": {
            color: tokens.orange,
        },
    },
    depth: (depth: number) => ({
        paddingLeft: `${depth * 0.55}rem`,
    }),
    folio: {
        color: tokens.orange,
        fontSize: "0.7rem",
        fontWeight: 600,
        letterSpacing: "0.06em",
        margin: "1.5rem 0 0",
        textTransform: "uppercase",
    },
    pagination: {
        borderTopColor: tokens.ink,
        borderTopStyle: "solid",
        borderTopWidth: tokens.stroke,
        display: "flex",
        flexWrap: "wrap",
        fontFamily: tokens.monoFont,
        fontWeight: 600,
        gap: "1rem 2rem",
        justifyContent: "space-between",
        marginTop: "3rem",
        paddingTop: "1rem",
    },
    search: {
        alignItems: "center",
        backgroundColor: "transparent",
        borderWidth: 0,
        color: tokens.soft,
        display: "grid",
        font: "inherit",
        fontSize: "0.78rem",
        gap: "0.4rem",
        gridTemplateColumns: "1rem minmax(0, 1fr)",
        minHeight: tokens.siteControlHeight,
        padding: "0.35rem 0",
        textAlign: "left",
        ":hover": {
            color: tokens.text,
        },
    },
    searchPrompt: {
        color: tokens.accent,
        fontFamily: tokens.monoFont,
    },
});

/// Journal navigation and article styles.
export const journalStyles = stylex.create({
    active: {
        color: tokens.text,
        fontWeight: 600,
    },
    articleHeader: {
        display: "grid",
        gap: "0.6rem",
        marginTop: "1.5rem",
    },
    articleSubtitle: {
        color: tokens.soft,
        fontSize: "0.86rem",
        lineHeight: 1.5,
        margin: 0,
        maxWidth: "39rem",
    },
    articleTitle: {
        fontFamily: tokens.monoFont,
        fontSize: "clamp(1.85rem, 4vw, 2.3rem)",
        fontWeight: 600,
        letterSpacing: "-0.025em",
        lineHeight: 1.1,
        margin: 0,
    },
    book: {
        alignContent: "start",
        display: "grid",
        gap: "0.75rem",
    },
    bookLink: {
        color: tokens.soft,
        display: "grid",
        fontSize: "0.78rem",
        gap: "0.1rem",
        ":hover": {
            color: tokens.text,
        },
    },
    bookList: {
        display: "grid",
        gap: "0.4rem",
        listStyle: "none",
        margin: 0,
        padding: 0,
    },
    bookTitle: {
        fontFamily: tokens.monoFont,
        fontSize: "0.78rem",
        fontWeight: 700,
        letterSpacing: "0.06em",
        textTransform: "uppercase",
        width: "max-content",
        ":hover": {
            color: tokens.accent,
        },
    },
    date: {
        fontFamily: tokens.monoFont,
        fontSize: "0.72rem",
    },
    location: {
        alignItems: "baseline",
        display: "flex",
        flexWrap: "wrap",
        gap: "0.5rem 1.5rem",
    },
    locationDate: {
        fontFamily: tokens.monoFont,
        fontSize: "0.78rem",
    },
    pagination: {
        borderTopColor: tokens.line,
        borderTopStyle: "solid",
        borderTopWidth: tokens.stroke,
        display: "flex",
        flexWrap: "wrap",
        fontFamily: tokens.monoFont,
        fontWeight: 600,
        gap: "1rem 2rem",
        justifyContent: "space-between",
        paddingTop: "1.25rem",
    },
});
