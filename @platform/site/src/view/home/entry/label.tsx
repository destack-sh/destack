import { color, font } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import type { JSX } from "@destack/view";

/** Label a column of a figure in small capitals, pointing at what follows. */
export function Label(properties: { children: JSX.Element }) {
    return <p {...style.attrs(styles.label)}>{properties.children} ▸</p>;
}

/** Set a column's label with a control at the end of its row, such as a dial. */
export function LabelRow(properties: { children: JSX.Element; end: JSX.Element }) {
    return (
        <div {...style.attrs(styles.row)}>
            <div {...style.attrs(styles.start)}>{properties.children}</div>
            {properties.end}
        </div>
    );
}

/** The label styles. */
const styles = style.create({
    row: {
        alignItems: "baseline",
        display: "flex",
        gap: "0.75rem",
        justifyContent: "space-between",
        minWidth: 0,
    },
    start: {
        minWidth: 0,
    },
    label: {
        color: color.mutedForeground,
        fontFamily: font.code,
        fontSize: "0.6875rem",
        letterSpacing: "0.1em",
        margin: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        textTransform: "uppercase",
        whiteSpace: "nowrap",
    },
});
