import { color, font } from "@destack/theme/tokens.stylex";
import { media } from "@destack/style/media.stylex";
import { present } from "@destack/schema";
import * as style from "@destack/style";
import type { JSX } from "@destack/view";

import { ItemBadge, type ItemMark } from "./item";
import { screen } from "../../layout/screen.stylex";

/** One figure in a ledger row: its caption over its value. */
export type Cell = {
    /** The caption, set as a mono label. */
    caption: string;
    /** The unit the value counts in, shown when the caption is pointed at. */
    unit: string;
    /** The value, set in tabular figures. */
    value: string;
    /** Whether the value sets lighter, as a measure beside the row's main figure. */
    isQuiet: boolean;
    /** Whether the cell drops on narrow screens. */
    isWide: boolean;
    /** Whether the value is a floor, marked with a leading `>`. */
    isFloor: boolean;
};

/** One row of a ledger: what it is, its marks, its colour, and its figures. */
export type LedgerRow = {
    /** The job the row does, set as the mono label. */
    label: string;
    /** The name of the product or app on the row. */
    name: string;
    /** The marks of every product on the row. */
    marks: readonly ItemMark[];
    /** The colour of the row's block: its vendor's accent, or the engine's signal once owned. */
    tint: string;
    /** What the row means, shown as its title. */
    note: string;
    /** Whether nothing does the row's job yet, so it fades. */
    isEmpty: boolean;
    /** The row's figures, one per column. */
    cells: readonly Cell[];
    /** What sits beside the name, such as the faces in the page. */
    badge: JSX.Element;
};

/** Draw a ledger: a stack of blocks, one per row, joined into one band once every row is owned. */
export function Ledger(properties: {
    rows: readonly LedgerRow[];
    isWhole: boolean;
    lit: number | undefined;
    isPressable: boolean;
    onPoint: (row: number | undefined) => void;
    onPress: (row: number) => void;
}) {
    // light a block while a mouse or pen points at it, since a touch lights it by pressing
    const point = (event: PointerEvent, row: number | undefined) => {
        if (event.pointerType !== "touch") {
            properties.onPoint(row);
        }
    };

    return (
        <div data-component="Ledger" {...style.attrs(styles.ledger)}>
            <ol {...style.attrs(styles.blocks)}>
                {properties.rows.map((row, index) => (
                    <li
                        title={row.note}
                        onPointerEnter={(event) => point(event, index)}
                        onPointerLeave={(event) => point(event, undefined)}
                        onClick={() => properties.onPress(index)}
                        {...style.attributes(
                            [
                                styles.row,
                                styles.block,
                                properties.isPressable && styles.pressable,
                                properties.isWhole && styles.blockWhole,
                                row.isEmpty && styles.blockEmpty,
                            ],
                            inlineOf(row.tint, properties.lit === index),
                        )}
                    >
                        <span {...style.attrs(styles.claim)}>
                            <Marks marks={row.marks} />
                            <span {...style.attrs(styles.names)}>
                                <span {...style.attrs(styles.label)}>{row.label}</span>
                                <span {...style.attrs(styles.nameLine)}>
                                    <b {...style.attrs(styles.name)}>{row.name}</b>
                                    {row.badge}
                                </span>
                            </span>
                        </span>
                        {row.cells.map((cell) => (
                            <Figure cell={cell} />
                        ))}
                    </li>
                ))}
            </ol>
        </div>
    );
}

/** Draw one figure of a row: its caption over its value. */
function Figure(properties: { cell: Cell }) {
    return (
        <span {...style.attrs(styles.cell, properties.cell.isWide && styles.wide)}>
            <span title={properties.cell.unit} {...style.attrs(styles.label)}>
                {properties.cell.caption}
            </span>
            <span {...style.attrs(styles.value, properties.cell.isQuiet && styles.valueQuiet)}>
                <span {...style.attrs(styles.floor)}>{properties.cell.isFloor ? ">" : ""}</span>
                {properties.cell.value}
            </span>
        </span>
    );
}

/** Draw a row's mark, or the marks of several apps as one folder tile of up to four. */
function Marks(properties: { marks: readonly ItemMark[] }) {
    return (
        <>
            {properties.marks.length === 1 ? (
                <span {...style.attrs(styles.marks)}>
                    <ItemBadge mark={present(properties.marks[0], "row mark")} />
                </span>
            ) : (
                <span {...style.attrs(styles.marks, styles.folder)}>
                    {properties.marks.slice(0, 4).map((mark) => (
                        <span {...style.attrs(styles.mini)}>
                            <ItemBadge mark={mark} />
                        </span>
                    ))}
                </span>
            )}
        </>
    );
}

/** Return a block's inline colours: a fill in its tint, stronger while lit, and a solid edge. */
function inlineOf(tint: string, isLit: boolean) {
    return {
        "background-color": `color-mix(in srgb, ${tint} ${isLit ? "24%" : "12%"}, transparent)`,
        "border-inline-start-color": tint,
    };
}

/** The ledger styles. */
const styles = style.create({
    ledger: {
        display: "grid",
        gridTemplateRows: "minmax(0, 1fr)",
        minHeight: { default: 0, [screen.belowDesktop]: "30rem", [media.maxMd]: 0 },
    },
    row: {
        alignItems: "center",
        borderInlineStartColor: "transparent",
        borderInlineStartStyle: "solid",
        borderInlineStartWidth: "3px",
        boxSizing: "border-box",
        columnGap: "0.75rem",
        display: "grid",
        gridTemplateColumns: {
            default: "minmax(0, 1fr) 4.5rem 4.5rem 4.75rem",
            [media.maxMd]: "minmax(0, 1fr) 4.75rem",
        },
        margin: 0,
        paddingInline: "0.75rem",
    },
    blocks: {
        display: "flex",
        flexDirection: "column",
        gap: 0,
        listStyle: "none",
        margin: 0,
        minHeight: 0,
        padding: 0,
    },
    block: {
        paddingBlock: { default: 0, [media.maxMd]: "0.5rem" },
        borderRadius: "4px",
        boxShadow: `inset 0 1.5px 0 ${color.background}, inset 0 -1.5px 0 ${color.background}`,
        flexBasis: { default: 0, [media.maxMd]: "auto" },
        flexGrow: 1,
        minHeight: "2.25rem",
        transitionDuration: { default: "480ms", [media.motionReduce]: "0ms" },
        transitionProperty: "background-color, border-color, border-radius, box-shadow",
        transitionTimingFunction: "ease",
    },
    pressable: {
        cursor: "pointer",
    },
    blockWhole: {
        borderRadius: 0,
        boxShadow: "none",
    },
    blockEmpty: {
        color: color.mutedForeground,
        opacity: 0.55,
    },
    claim: {
        alignItems: "center",
        display: "flex",
        gap: "0.75rem",
        minWidth: 0,
    },
    marks: {
        display: "flex",
        flexShrink: 0,
        height: "1.75rem",
        width: "1.75rem",
    },
    folder: {
        alignContent: "center",
        backgroundColor: color.muted,
        borderRadius: "6px",
        boxShadow: `inset 0 0 0 1px ${color.border}`,
        boxSizing: "border-box",
        display: "grid",
        gap: "2px",
        gridTemplateColumns: "repeat(2, 0.65rem)",
        justifyContent: "center",
    },
    mini: {
        display: "flex",
        zoom: 0.37,
    },
    names: {
        display: "grid",
        minWidth: 0,
        overflow: "hidden",
    },
    label: {
        color: color.mutedForeground,
        fontFamily: font.code,
        fontSize: "0.625rem",
        letterSpacing: "0.1em",
        lineHeight: "0.875rem",
        overflow: "hidden",
        textOverflow: "ellipsis",
        textTransform: "uppercase",
        whiteSpace: "nowrap",
    },
    nameLine: {
        alignItems: "center",
        display: "flex",
        gap: "0.5rem",
        minWidth: 0,
    },
    name: {
        fontSize: "1rem",
        fontWeight: 600,
        lineHeight: "1.25rem",
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    cell: {
        display: "grid",
        justifyItems: "end",
    },
    value: {
        fontFamily: font.code,
        fontSize: "1rem",
        fontVariantNumeric: "tabular-nums",
        fontWeight: 600,
        lineHeight: "1.25rem",
        whiteSpace: "pre",
    },
    valueQuiet: {
        fontWeight: 400,
    },
    wide: {
        display: { default: "grid", [media.maxMd]: "none" },
    },
    floor: {
        display: "inline-block",
        textAlign: "start",
        width: "1.25ch",
    },
});
