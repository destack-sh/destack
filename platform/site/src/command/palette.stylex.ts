import * as stylex from "@stylexjs/stylex";

import { tokens } from "../style/tokens.stylex";

/// Command palette styles.
export const styles = stylex.create({
    close: {
        backgroundColor: "transparent",
        borderWidth: 0,
        color: tokens.soft,
        font: "inherit",
        padding: 0,
    },
    context: {
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    empty: {
        color: tokens.soft,
        margin: 0,
        padding: "1rem",
    },
    excerpt: {
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    frame: {
        display: "grid",
        minHeight: 0,
        minWidth: 0,
    },
    indicator: {
        color: tokens.accent,
    },
    input: {
        backgroundColor: "transparent",
        borderWidth: 0,
        color: "inherit",
        font: "inherit",
        minWidth: 0,
        outlineWidth: 0,
        "::placeholder": {
            color: tokens.soft,
            opacity: 1,
        },
    },
    inputLabel: {
        alignItems: "center",
        borderBottomColor: tokens.line,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.stroke,
        display: "grid",
        gap: "0.5rem",
        gridTemplateColumns: "1rem minmax(0, 1fr) auto",
        padding: "0.75rem 1rem",
    },
    inputPrompt: {
        color: tokens.accent,
    },
    mark: {
        backgroundColor: tokens.orange,
        borderRadius: "0.2rem",
        color: tokens.ink,
        paddingInline: "0.1rem",
    },
    palette: {
        backgroundColor: tokens.cream,
        borderColor: tokens.ink,
        borderRadius: tokens.panelRadius,
        borderStyle: "solid",
        borderWidth: tokens.stroke,
        color: tokens.ink,
        fontFamily: tokens.monoFont,
        fontSize: tokens.siteFontSize,
        margin: "10svh auto auto",
        maxHeight: "min(42rem, calc(100svh - 2rem))",
        maxWidth: "none",
        padding: 0,
        width: "min(46rem, calc(100vw - 2rem))",
        "::backdrop": {
            backgroundColor: "rgb(5 46 64 / 72%)",
        },
    },
    result: {
        display: "grid",
        gap: "0.25rem",
        minWidth: 0,
    },
    resultButton: {
        alignItems: "start",
        backgroundColor: "transparent",
        borderWidth: 0,
        color: tokens.soft,
        display: "grid",
        font: "inherit",
        gap: "0.75rem",
        gridTemplateColumns: "1rem 10rem minmax(0, 1fr) auto",
        padding: "0.65rem 1rem",
        textAlign: "left",
        width: "100%",
        ":hover": {
            backgroundColor: tokens.creamDeep,
            color: tokens.ink,
        },
        "@media (max-width: 640px)": {
            gridTemplateColumns: "1rem minmax(0, 1fr) auto",
        },
    },
    resultContextMobile: {
        "@media (max-width: 640px)": {
            display: "none",
        },
    },
    resultLabel: {
        color: tokens.ink,
        fontWeight: 600,
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    resultList: {
        listStyle: "none",
        margin: 0,
        maxHeight: "min(34rem, calc(100svh - 8rem))",
        overflowY: "auto",
        padding: 0,
    },
    resultRow: {
        borderTopColor: tokens.line,
        borderTopStyle: "solid",
        borderTopWidth: tokens.stroke,
    },
    resultRowFirst: {
        borderTopWidth: 0,
    },
    selected: {
        backgroundColor: tokens.creamDeep,
        color: tokens.ink,
    },
    shortcut: {
        color: tokens.soft,
        font: "inherit",
        whiteSpace: "nowrap",
    },
    toggle: {
        alignItems: "center",
        backgroundColor: "transparent",
        borderWidth: 0,
        color: "inherit",
        display: "inline-flex",
        font: "inherit",
        letterSpacing: "inherit",
        minHeight: tokens.siteControlHeight,
        padding: 0,
        textTransform: "inherit",
        ":hover": {
            color: tokens.accent,
        },
    },
});
