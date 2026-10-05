import { color } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";
import type { JSX } from "@destack/view";

import { tokens } from "../style/tokens.stylex";

/** Label a column of a figure in small capitals, pointing at what follows. */
export function Label(properties: { children: JSX.Element }) {
    return <p {...stylex.attrs(styles.label)}>{properties.children} ▸</p>;
}

/** The label styles. */
const styles = stylex.create({
    label: {
        color: color.mutedForeground,
        fontFamily: tokens.monoFont,
        fontSize: "0.6875rem",
        letterSpacing: "0.1em",
        margin: 0,
        textTransform: "uppercase",
    },
});
