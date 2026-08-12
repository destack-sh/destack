import * as stylex from "@stylexjs/stylex";

import { tokens } from "./tokens.stylex";

export const styles = stylex.create({
    root: {
        backgroundColor: tokens.page,
        color: tokens.text,
        minHeight: "100vh",
        "::selection": {
            backgroundColor: tokens.accent,
            color: tokens.ink,
        },
    },
});
