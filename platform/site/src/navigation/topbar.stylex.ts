import * as stylex from "@stylexjs/stylex";

import { tokens } from "../style/tokens.stylex";

const hover = { color: tokens.accent };

export const styles = stylex.create({
    actions: {
        alignItems: "center",
        display: "flex",
        fontWeight: 400,
        gap: "1.5rem",
        justifyContent: "flex-end",
        justifySelf: "end",
        minWidth: 0,
    },
    body: {
        alignItems: "center",
        display: "grid",
        fontFamily: tokens.monoFont,
        fontSize: "0.8rem",
        fontWeight: 600,
        gap: "1.5rem",
        gridTemplateColumns: "auto minmax(0, 1fr) auto",
        letterSpacing: "0.06em",
        margin: "0 auto",
        maxWidth: tokens.siteWidth,
        minHeight: "3.25rem",
        paddingLeft: tokens.gutterLeft,
        paddingRight: tokens.gutterRight,
        textTransform: "uppercase",
        width: "100%",
    },
    brand: {
        alignItems: "center",
        display: "inline-flex",
        flex: "none",
        fontWeight: 600,
        minHeight: tokens.siteControlHeight,
        ":hover": hover,
    },
    link: {
        alignItems: "center",
        display: "inline-flex",
        flex: "none",
        minHeight: tokens.siteControlHeight,
        ":hover": hover,
    },
    primary: {
        alignItems: "center",
        display: "flex",
        fontWeight: 400,
        gap: "1.5rem",
        justifyContent: "flex-end",
        minWidth: 0,
        overflow: "hidden",
    },
    root: {
        backgroundColor: tokens.page,
        borderBottomColor: tokens.ink,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.stroke,
        borderTopColor: tokens.accent,
        borderTopStyle: "solid",
        borderTopWidth: tokens.stroke,
        color: tokens.text,
        maxWidth: "100vw",
    },
});
