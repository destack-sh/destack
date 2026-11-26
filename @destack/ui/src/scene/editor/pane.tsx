import * as style from "@destack/style";
import { color, size, space, stroke } from "@destack/theme/tokens.stylex";
import type { JSX } from "@destack/view";

/** The styles of a pane, its header row and its body. */
const styles = style.create({
    pane: {
        display: "grid",
        gridTemplateRows: "auto minmax(0, 1fr)",
        height: "100%",
        minWidth: 0,
    },
    header: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
        height: size[3],
        paddingInline: space[3],
        borderBottomWidth: stroke.border,
        borderBottomStyle: "solid",
        borderBottomColor: color.border,
        overflow: "hidden",
    },
    body: {
        minHeight: 0,
        overflow: "auto",
    },
    side: {
        backgroundColor: color.sidebar,
    },
});

/** Frame a pane of the editor: a header row as high as every pane's, and a scrolling body. */
export function Pane(properties: {
    /** What the header row shows. */
    readonly header: JSX.Element;
    /** Whether it is a side pane, on the sidebar's ground. */
    readonly isSide?: boolean;
    /** The body. */
    readonly children: JSX.Element;
}): JSX.Element {
    return (
        <section {...style.attrs(styles.pane, properties.isSide === true && styles.side)}>
            <header {...style.attrs(styles.header)}>{properties.header}</header>
            <div {...style.attrs(styles.body)}>{properties.children}</div>
        </section>
    );
}
