import * as stylex from "@stylexjs/stylex";

import { tokens } from "../style/tokens.stylex";

export const styles = stylex.create({
    main: {
        display: "grid",
        minHeight: 0,
        minWidth: 0,
        width: "100%",
    },
    paper: {
        isolation: "isolate",
        position: "relative",
    },
    paperGrain: {
        backgroundImage: 'url("/grain.svg")',
        backgroundRepeat: "repeat",
        backgroundSize: "8rem 8rem",
        inset: 0,
        mixBlendMode: "multiply",
        opacity: 0.24,
        pointerEvents: "none",
        position: "fixed",
        zIndex: 0,
    },
    paperLayer: {
        position: "relative",
        zIndex: 1,
    },
    root: {
        backgroundColor: tokens.page,
        color: tokens.text,
        display: "grid",
        fontFamily: tokens.textFont,
        gridTemplateRows: "auto minmax(0, 1fr)",
        minHeight: "100svh",
        minWidth: 0,
        overflowX: "clip",
    },
});
