import { color, font } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { For, type JSX } from "@destack/view";
import { typeScale, tracking } from "../../layout/type.stylex";

/** Label a column of a figure in small capitals, pointing at what follows. */
export function Label(properties: { children: JSX.Element }) {
    return <p {...style.attrs(styles.label)}>{properties.children} ▸</p>;
}

/** Label a column with tabs in small capitals, the shown one pointing at what follows. */
export function LabelTabs(properties: {
    tabs: readonly string[];
    shown: number;
    onShow: (index: number) => void;
}) {
    return (
        <div role="tablist" {...style.attrs(styles.tabs)}>
            <For each={properties.tabs}>
                {(tab, index) => (
                    <button
                        type="button"
                        role="tab"
                        aria-selected={index() === properties.shown ? "true" : "false"}
                        onClick={() => properties.onShow(index())}
                        {...style.attrs(
                            styles.label,
                            styles.tab,
                            index() === properties.shown && styles.tabShown,
                        )}
                    >
                        {tab}
                        <span
                            aria-hidden="true"
                            {...style.attrs(index() !== properties.shown && styles.unshown)}
                        >
                            {" ▸"}
                        </span>
                    </button>
                )}
            </For>
        </div>
    );
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
    tabs: {
        display: "flex",
        gap: "1rem",
        minWidth: 0,
        overflow: "hidden",
    },
    tab: {
        backgroundColor: "transparent",
        borderStyle: "none",
        cursor: "pointer",
        opacity: { default: 0.55, ":hover": { default: null, [media.hover]: 1 } },
        padding: 0,
    },
    tabShown: {
        opacity: 1,
    },
    unshown: {
        visibility: "hidden",
    },
    label: {
        color: color.mutedForeground,
        fontFamily: font.code,
        fontSize: typeScale.label,
        letterSpacing: tracking.label,
        margin: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        textTransform: "uppercase",
        whiteSpace: "nowrap",
    },
});
