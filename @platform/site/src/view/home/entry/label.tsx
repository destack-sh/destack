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

/** Label a figure's pane, with tabs between its panes below the desktop width. */
export function PaneLabel(properties: {
    children: JSX.Element;
    panes: readonly string[];
    shown: number;
    onShow: (pane: number) => void;
}) {
    return (
        <>
            {/* the pane's label beside the other pane, on desktops */}
            <div {...style.attrs(styles.wide)}>
                <Label>{properties.children}</Label>
            </div>

            {/* tabs between the panes, one at a time, below the desktop width */}
            <div {...style.attrs(styles.narrow)}>
                <LabelTabs
                    tabs={properties.panes}
                    shown={properties.shown}
                    onShow={properties.onShow}
                />
            </div>
        </>
    );
}

/** Set a column's label with a control after it, wrapped below when both do not fit. */
export function LabelRow(properties: { children: JSX.Element; end: JSX.Element }) {
    return (
        <div {...style.attrs(styles.row)}>
            <div {...style.attrs(styles.start)}>{properties.children}</div>
            {properties.end}
        </div>
    );
}

/** The panes of a figure, one of which hides below the desktop width. */
export const pane = style.create({
    /** A figure cell hidden below the desktop width, its grid restated above it. */
    hidden: {
        display: { default: "grid", [media.maxLg]: "none" },
    },
});

/** The label styles. */
const styles = style.create({
    row: {
        alignItems: "baseline",
        columnGap: "0.75rem",
        display: "flex",
        flexWrap: "wrap",
        justifyContent: "space-between",
        // hold every label row at the height of its tallest control, so the windows below line up
        minHeight: "1.25rem",
        minWidth: 0,
        rowGap: "0.5rem",
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
        borderBottomColor: "transparent",
        borderBottomStyle: "solid",
        borderBottomWidth: "2px",
        borderInlineWidth: 0,
        borderTopWidth: 0,
        cursor: "pointer",
        opacity: { default: 0.6, ":hover": { default: null, [media.hover]: 1 } },
        paddingBlock: "0.25rem",
        paddingInline: 0,
    },
    tabShown: {
        borderBottomColor: color.primary,
        color: color.foreground,
        opacity: 1,
    },
    wide: {
        display: { default: "contents", [media.maxLg]: "none" },
    },
    narrow: {
        display: { default: "none", [media.maxLg]: "contents" },
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
