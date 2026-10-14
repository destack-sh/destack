import { color, font } from "@destack/theme/tokens.stylex";
import { media } from "@destack/style/media.stylex";
import { present } from "@destack/schema";
import * as style from "@destack/style";
import { For, type JSX } from "@destack/view";

import { ItemBadge, type ItemMark } from "./item";
import { ledgerSizes } from "./ledger.stylex";
import { screen } from "../../layout/screen.stylex";

/** The size of a quiet figure, in rem. */
const QUIET_SIZE = 0.8125;

/** The size of a row's main figure, in rem. */
const VALUE_SIZE = 0.9375;

/** The size of a caption, in rem. */
const LABEL_SIZE = 0.6875;

/** The tracking a caption adds to each letter, as a share of its size. */
const LABEL_TRACKING = 0.1;

/** The ledger width below which it shows only each row's name and cost, leaving the names room. */
const NARROW = "@container (max-width: 26rem)";

/** The padding each row keeps inside its block, which its subgrid takes from the edge columns. */
const ROW_INSET = "0.75rem";

/** One column of a ledger: its caption and unit, set once above the rows. */
export type Column = {
    /** The caption, set as a mono label. */
    caption: string;
    /** The unit the column counts in, set under the caption. */
    unit: string;
    /** What the column measures, shown when the caption is pointed at. */
    note: string;
    /** Whether the column drops on narrow screens. */
    isWide: boolean;
};

/** One figure in a ledger row. */
export type Cell = {
    /** The value, set in tabular figures. */
    value: string;
    /** Whether the value sets lighter, as a measure beside the row's main figure. */
    isQuiet: boolean;
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
    /** The plan the row is billed on, shown with its note when pointed at. */
    plan: string;
    /** Whether nothing does the row's job yet, so it fades. */
    isEmpty: boolean;
    /** The row's figures, one per column. */
    cells: readonly Cell[];
    /** What sits beside the name, such as the faces in the page. */
    badge: JSX.Element;
};

/** Draw a ledger: a stack of blocks, one per row, joined into one band once every row is owned. */
export function Ledger(properties: {
    columns: readonly Column[];
    widths: readonly number[];
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

    // size each figure column, the last one padded by the row's inset
    const tracks = () =>
        properties.widths.map((characters, column) =>
            widthOf(
                characters,
                present(properties.columns[column], "column"),
                column === properties.widths.length - 1 ? ROW_INSET : "0rem",
            ),
        );

    return (
        <div data-component="Ledger" {...style.attrs(styles.ledger)}>
            <div
                {...style.attrs(
                    styles.grid,
                    styles.columns(tracks().join(" "), present(tracks().at(-1), "last track")),
                )}
            >
                {/* column heads, each caption over its unit */}
                <div aria-hidden="true" {...style.attrs(styles.row, styles.head)}>
                    <span />
                    <For each={properties.columns}>
                        {(column) => (
                            <span
                                title={column.note}
                                {...style.attrs(styles.cell, column.isWide && styles.wide)}
                            >
                                <span {...style.attrs(styles.label)}>{column.caption}</span>
                                <span {...style.attrs(styles.label)}>{column.unit}</span>
                            </span>
                        )}
                    </For>
                </div>
                <ol {...style.attrs(styles.blocks)}>
                    <For each={properties.rows} keyed={false}>
                        {(row, index) => (
                            <li
                                title={`${row().note}\n${row().plan}`}
                                onPointerEnter={(event) => point(event, index)}
                                onPointerLeave={(event) => point(event, undefined)}
                                onClick={() => properties.onPress(index)}
                                {...style.attributes(
                                    [
                                        styles.row,
                                        styles.block,
                                        properties.isPressable && styles.pressable,
                                        properties.isWhole && styles.blockWhole,
                                        row().isEmpty && styles.blockEmpty,
                                    ],
                                    inlineOf(row().tint, properties.lit === index),
                                )}
                            >
                                <span {...style.attrs(styles.claim)}>
                                    <Marks marks={row().marks} />
                                    <span {...style.attrs(styles.names)}>
                                        <span {...style.attrs(styles.label)}>{row().label}</span>
                                        <span {...style.attrs(styles.nameLine)}>
                                            <b {...style.attrs(styles.name)}>{row().name}</b>
                                            {row().badge}
                                        </span>
                                    </span>
                                </span>
                                <For each={row().cells} keyed={false}>
                                    {(cell, column) => (
                                        <Figure
                                            cell={cell()}
                                            isWide={
                                                present(properties.columns[column], "column").isWide
                                            }
                                        />
                                    )}
                                </For>
                            </li>
                        )}
                    </For>
                </ol>
            </div>
        </div>
    );
}

/** Draw one figure of a row. */
function Figure(properties: { cell: Cell; isWide: boolean }) {
    return (
        <span {...style.attrs(styles.cell, properties.isWide && styles.wide)}>
            <span {...style.attrs(styles.value, properties.cell.isQuiet && styles.valueQuiet)}>
                {properties.cell.isFloor ? ">" : ""}
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

/** Return a column's width: its widest figure or its caption, plus any edge padding. */
function widthOf(characters: number, column: Column, edge: string): string {
    // measure in the ledger's own code face, its ch set at a main figure's size
    const size = column.isWide ? QUIET_SIZE / VALUE_SIZE : 1;
    const figure = characters * size;
    const letters = column.caption.length;
    const caption = `${(letters * (LABEL_SIZE / VALUE_SIZE)).toFixed(3)}ch + ${(letters * LABEL_TRACKING * LABEL_SIZE).toFixed(3)}rem`;

    return `calc(max(${figure.toFixed(3)}ch, calc(${caption})) + ${edge})`;
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
    columns: (figures: string, last: string) => ({
        gridTemplateColumns: {
            default: `minmax(0, 1fr) ${figures}`,
            [media.maxMd]: `minmax(0, 1fr) ${last}`,
            [NARROW]: `minmax(0, 1fr) ${last}`,
        },
    }),
    ledger: {
        containerType: "inline-size",
        display: "grid",
        minHeight: { default: 0, [screen.belowDesktop]: "30rem", [media.maxMd]: 0 },
    },
    grid: {
        columnGap: "0.75rem",
        display: "grid",
        fontFamily: font.code,
        fontSize: `${VALUE_SIZE}rem`,
        gridTemplateRows: "auto minmax(0, 1fr)",
        minHeight: 0,
    },
    row: {
        alignItems: "center",
        borderInlineStartColor: "transparent",
        borderInlineStartStyle: "solid",
        borderInlineStartWidth: "3px",
        boxSizing: "border-box",
        display: "grid",
        gridColumn: "1 / -1",
        gridTemplateColumns: "subgrid",
        margin: 0,
        paddingInline: ROW_INSET,
    },
    blocks: {
        display: "grid",
        gridAutoRows: { default: "minmax(2.25rem, 1fr)", [media.maxMd]: "auto" },
        gridColumn: "1 / -1",
        gridTemplateColumns: "subgrid",
        rowGap: 0,
        listStyle: "none",
        margin: 0,
        minHeight: 0,
        padding: 0,
    },
    block: {
        paddingBlock: { default: 0, [media.maxMd]: "0.5rem" },
        borderRadius: "4px",
        boxShadow: `inset 0 1.5px 0 ${color.background}, inset 0 -1.5px 0 ${color.background}`,
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
        fontFamily: font.text,
        gap: "0.625rem",
        minWidth: 0,
    },
    marks: {
        display: "flex",
        flexShrink: 0,
        height: "1.75rem",
        width: "1.75rem",
        zoom: 1.125,
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
        fontSize: `${VALUE_SIZE}rem`,
        fontVariantNumeric: "tabular-nums",
        fontWeight: 600,
        lineHeight: "1.25rem",
        whiteSpace: "pre",
    },
    valueQuiet: {
        color: color.mutedForeground,
        fontSize: `${QUIET_SIZE}rem`,
        fontWeight: 400,
    },
    head: {
        borderInlineStartColor: "transparent",
        boxSizing: "border-box",
        height: ledgerSizes.head,
        paddingBlock: "0.25rem",
    },
    wide: {
        display: { default: "grid", [media.maxMd]: "none", [NARROW]: "none" },
    },
});
