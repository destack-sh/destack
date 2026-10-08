import { color, font } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import type { JSX } from "@destack/view";

/** Label a column of a figure in small capitals, pointing at what follows. */
export function Label(properties: { children: JSX.Element }) {
    return <p {...style.attrs(styles.label)}>{properties.children} ▸</p>;
}

/** The label styles. */
const styles = style.create({
    label: {
        color: color.mutedForeground,
        fontFamily: font.code,
        fontSize: "0.6875rem",
        letterSpacing: "0.1em",
        margin: 0,
        textTransform: "uppercase",
    },
});
