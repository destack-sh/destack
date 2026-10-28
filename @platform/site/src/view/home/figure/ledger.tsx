import { color, font, motion, shadow, stroke } from "@destack/theme/tokens.stylex";
import { media } from "@destack/style/media.stylex";
import { present } from "@destack/schema";
import * as style from "@destack/style";
import { For, onSettled } from "@destack/view";

import { browserStyles } from "./browser";
import { Digits } from "./digits";
import { ItemBadge, type ItemMark } from "./item";
import { InlinePicker, type Option, PickerName } from "./picker";
import { chrome } from "./chrome.stylex";
import { ledgerVariables } from "./ledger.stylex";
import { screen } from "../../layout/screen.stylex";
import { typeScale, tracking } from "../../layout/type.stylex";

/** The size of a quiet figure, in rem, as typeScale.small sets it. */
const QUIET_SIZE = 0.8125;

/** The size of a row's main figure, in rem, as typeScale.body sets it. */
const VALUE_SIZE = 0.9375;

/** The size of a caption, in rem, as typeScale.label sets it. */
const LABEL_SIZE = 0.6875;

/** The tracking a caption adds to each letter, as a share of its size. */
const LABEL_TRACKING = 0.1;

/** The tracking a caption adds in a narrow ledger, tight so the figures fit beside the names. */
const NARROW_LABEL_TRACKING = 0.02;

/** The least width of the name column, which stays put while the figures scroll under it. */
const NAME_WIDTH = "8.75rem";

/** The least width of the name column in a narrow ledger, enough for a job and a short name. */
const NARROW_NAME_WIDTH = "8rem";

/** A ledger narrower than its name and figures at full size, as beside a tablet's wall. */
const NARROW_LEDGER = "@container (max-width: 460px)";

/** A ledger narrower than every column at narrow tracking, as on a phone: the bill needs 392px. */
const CRAMPED_LEDGER = "@container (max-width: 400px)";

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
    /** Whether the column's figures set lighter, as measures beside the row's main figure. */
    isQuiet: boolean;
    /** Whether the column hides in a cramped ledger, so the others fit without scrolling. */
    isSpare: boolean;
};

/** One figure in a ledger row. */
export type Cell = {
    /** The value, set in tabular figures. */
    value: string;
    /** Whether the value sets lighter, as a measure beside the row's main figure. */
    isQuiet: boolean;
    /** Whether the value is a floor, marked with a leading `>`. */
    isFloor: boolean;
    /** The line set above the value, level with the row's label, such as a saving. */
    caption: string | undefined;
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
    /** The plan the row is billed on, shown when pointed at. */
    plan: string;
    /** Whether nothing does the row's job yet, so it fades. */
    isEmpty: boolean;
    /** The row's figures, one per column. */
    cells: readonly Cell[];
    /** The products the row can show instead, its own among them, or none to choose from. */
    options: readonly Option[];
};

/** Draw a ledger: a stack of blocks, one per row, joined into one band once every row is owned. */
export function Ledger(properties: {
    isWindow: boolean;
    columns: readonly Column[];
    widths: readonly number[];
    rows: readonly LedgerRow[];
    isWhole: boolean;
    lit: number | undefined;
    isPressable: boolean;
    onPoint: (row: number | undefined) => void;
    onPress: (row: number) => void;
    onChoose: (row: number, name: string) => void;
}) {
    // light a block while a mouse or pen points at it, since a touch lights it by pressing
    const point = (event: PointerEvent, row: number | undefined) => {
        if (event.pointerType !== "touch") {
            properties.onPoint(row);
        }
    };

    // press a block, unless the click landed on its name's picker or in the menu it opens
    const press = (event: MouseEvent, row: number) => {
        if (!(event.target instanceof Element && event.target.closest("button, [popover]"))) {
            properties.onPress(row);
        }
    };

    // start scrolled to the last column, the cost, where the figures overflow a narrow ledger
    let scroller: HTMLDivElement | undefined;
    onSettled(() => {
        // require the rendered scroller
        if (!scroller) {
            throw new TypeError("the ledger rendered without its scroller");
        }
        scroller.scrollLeft = scroller.scrollWidth;
    });

    // size each figure column at a caption tracking, the last one padded by the row's inset
    const tracks = (captionTracking: number, isCramped: boolean) =>
        properties.widths
            .map((characters, column) => {
                const shown = present(properties.columns[column], "column");

                return isCramped && shown.isSpare
                    ? undefined
                    : widthOf(
                          characters,
                          shown,
                          column === properties.widths.length - 1 ? ROW_INSET : "0rem",
                          captionTracking,
                      );
            })
            .filter((track) => track !== undefined)
            .join(" ");

    return (
        <div data-component="Ledger" {...style.attrs(styles.ledger)}>
            <div {...style.attrs(styles.frame, properties.isWindow && styles.window)}>
                {/* table, scrolling its figures under the names where they overflow */}
                <div
                    ref={scroller}
                    {...style.attrs(
                        styles.grid,
                        styles.columns(
                            tracks(LABEL_TRACKING, false),
                            tracks(NARROW_LABEL_TRACKING, false),
                            tracks(NARROW_LABEL_TRACKING, true),
                        ),
                    )}
                >
                    {/* the title bar: the window's lights, then each caption over its unit */}
                    <div
                        {...style.attrs(
                            styles.row,
                            styles.head,
                            properties.isWindow && styles.headWindow,
                        )}
                    >
                        <span
                            {...style.attrs(
                                styles.corner,
                                properties.isWindow && styles.cornerWindow,
                            )}
                        >
                            {properties.isWindow && (
                                <span aria-hidden="true" {...style.attrs(browserStyles.lights)} />
                            )}
                        </span>
                        <For each={properties.columns}>
                            {(column) => (
                                <span
                                    aria-hidden="true"
                                    title={column.note}
                                    {...style.attrs(styles.cell, column.isSpare && styles.spare)}
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
                                    title={row().plan}
                                    onPointerEnter={(event) => point(event, index)}
                                    onPointerLeave={(event) => point(event, undefined)}
                                    onClick={(event) => press(event, index)}
                                    {...style.attrs(
                                        styles.row,
                                        styles.block,
                                        properties.isPressable && styles.pressable,
                                        properties.isWindow && styles.blockFlush,
                                        properties.isWhole && styles.blockWhole,
                                        row().isEmpty && styles.blockEmpty,
                                        styles.tinted(
                                            fillOf(row().tint, properties.lit === index),
                                            row().tint,
                                        ),
                                    )}
                                >
                                    <span {...style.attrs(styles.claim)}>
                                        {row().options.length > 1 ? (
                                            <InlinePicker
                                                label={row().label}
                                                options={row().options}
                                                shown={row().name}
                                                onChoose={(name) =>
                                                    properties.onChoose(index, name)
                                                }
                                            >
                                                <Claim row={row()} isPicked />
                                            </InlinePicker>
                                        ) : (
                                            <Claim row={row()} isPicked={false} />
                                        )}
                                    </span>
                                    <For each={row().cells} keyed={false}>
                                        {(cell, column) => (
                                            <Figure
                                                cell={cell()}
                                                isSpare={
                                                    present(properties.columns[column], "column")
                                                        .isSpare
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
        </div>
    );
}

/** Draw one figure of a row, its value level with the row's name and its caption with the label. */
function Figure(properties: { cell: Cell; isSpare: boolean }) {
    return (
        <span {...style.attrs(styles.cell, properties.isSpare && styles.spare)}>
            {/* caption, kept as an empty line where there is none */}
            <span {...style.attrs(styles.captionLine)}>
                {properties.cell.caption !== undefined && (
                    <span {...style.attrs(styles.caption)}>
                        <Digits text={properties.cell.caption} />
                    </span>
                )}
            </span>

            {/* value */}
            <span {...style.attrs(styles.value, properties.cell.isQuiet && styles.valueQuiet)}>
                <Digits text={`${properties.cell.isFloor ? ">" : ""}${properties.cell.value}`} />
            </span>
        </span>
    );
}

/** Draw a row's mark, label and name, the name as a picker's when the row offers options. */
function Claim(properties: { row: LedgerRow; isPicked: boolean }) {
    return (
        <>
            <Marks marks={properties.row.marks} />
            <span {...style.attrs(styles.names)}>
                <span {...style.attrs(styles.label)}>{properties.row.label}</span>
                <span {...style.attrs(styles.nameLine)}>
                    <b {...style.attrs(styles.name)}>
                        {properties.isPicked ? (
                            <PickerName
                                options={properties.row.options}
                                shown={properties.row.name}
                            />
                        ) : (
                            properties.row.name
                        )}
                    </b>
                </span>
            </span>
        </>
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
function widthOf(
    characters: number,
    column: Column,
    edge: string,
    captionTracking: number,
): string {
    // measure in the ledger's own code face, its ch set at a main figure's size
    const size = column.isQuiet ? QUIET_SIZE / VALUE_SIZE : 1;
    const figure = characters * size;
    const letters = column.caption.length;
    const caption = `${(letters * (LABEL_SIZE / VALUE_SIZE)).toFixed(3)}ch + ${(letters * captionTracking * LABEL_SIZE).toFixed(3)}rem`;

    return `calc(max(${figure.toFixed(3)}ch, calc(${caption})) + ${edge})`;
}

/** Return a block's fill: its tint over the surface, stronger while lit. */
function fillOf(tint: string, isLit: boolean): string {
    return `color-mix(in srgb, ${tint} ${isLit ? "24%" : "12%"}, transparent)`;
}

/** The ledger styles. */
const styles = style.create({
    columns: (figures: string, narrowFigures: string, crampedFigures: string) => ({
        gridTemplateColumns: {
            default: `minmax(${NAME_WIDTH}, 1fr) ${figures}`,
            [NARROW_LEDGER]: `minmax(${NARROW_NAME_WIDTH}, 1fr) ${narrowFigures}`,
            [CRAMPED_LEDGER]: `minmax(${NARROW_NAME_WIDTH}, 1fr) ${crampedFigures}`,
        },
    }),
    tinted: (fill: string, tint: string) => ({
        [ledgerVariables.fill]: fill,
        [ledgerVariables.tint]: tint,
    }),
    ledger: {
        containerType: "inline-size",
        display: "grid",
        minHeight: { default: 0, [screen.belowDesktop]: "30rem", [media.maxMd]: 0 },
    },
    frame: {
        [ledgerVariables.surface]: color.background,
        display: "grid",
        gridTemplateRows: "minmax(0, 1fr)",
        minHeight: 0,
    },
    grid: {
        columnGap: { default: "0.75rem", [NARROW_LEDGER]: "0.375rem" },
        display: "grid",
        fontFamily: font.code,
        fontSize: typeScale.body,
        gridTemplateRows: "auto minmax(0, 1fr)",
        minHeight: 0,
        overflowX: "auto",
        overflowY: "hidden",
        overscrollBehaviorX: "contain",
    },
    window: {
        [ledgerVariables.surface]: color.card,
        backgroundColor: color.card,
        borderColor: color.border,
        borderRadius: chrome.radius,
        borderStyle: "solid",
        borderWidth: stroke.border,
        boxShadow: shadow.raised,
        gridTemplateRows: "minmax(0, 1fr)",
        overflow: "hidden",
    },
    row: {
        alignItems: "center",
        boxSizing: "border-box",
        display: "grid",
        gridColumn: "1 / -1",
        gridTemplateColumns: "subgrid",
        margin: 0,
        paddingInlineEnd: ROW_INSET,
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
        alignItems: "stretch",
        position: "relative",
        backgroundColor: ledgerVariables.fill,
        borderRadius: "4px",
        boxShadow: `inset 0 1.5px 0 ${color.background}, inset 0 -1.5px 0 ${color.background}`,
        transitionDuration: motion.durationShort,
        transitionProperty: "background-color, border-radius, box-shadow",
        transitionTimingFunction: motion.easingStandard,
    },
    pressable: {
        cursor: "pointer",
    },
    blockFlush: {
        borderRadius: 0,
        boxShadow: { default: `inset 0 -1px 0 ${color.border}`, ":last-child": "none" },
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
        backgroundColor: ledgerVariables.surface,
        borderEndStartRadius: "inherit",
        borderInlineStartColor: ledgerVariables.tint,
        borderInlineStartStyle: "solid",
        borderInlineStartWidth: "3px",
        borderStartStartRadius: "inherit",
        boxShadow: "inherit",
        display: "flex",
        fontFamily: font.text,
        gap: "0.625rem",
        insetInlineStart: 0,
        minWidth: 0,
        paddingBlock: { default: 0, [media.maxMd]: "0.5rem" },
        paddingInlineStart: ROW_INSET,
        position: "sticky",
        transitionDuration: motion.durationShort,
        transitionProperty: "border-color",
        transitionTimingFunction: motion.easingStandard,
        zIndex: 1,
        "::before": {
            backgroundColor: ledgerVariables.fill,
            borderRadius: "inherit",
            boxShadow: "inherit",
            content: '""',
            inset: 0,
            position: "absolute",
            transitionDuration: motion.durationShort,
            transitionProperty: "background-color",
            transitionTimingFunction: motion.easingStandard,
            zIndex: -1,
        },
    },
    marks: {
        display: "flex",
        flexShrink: 0,
        height: "1.75rem",
        width: "1.75rem",
        zoom: { default: 1.125, [NARROW_LEDGER]: 0.875 },
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
        fontSize: typeScale.label,
        letterSpacing: { default: tracking.label, [NARROW_LEDGER]: "0.02em" },
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
        fontSize: typeScale.body,
        fontWeight: 600,
        lineHeight: "1.25rem",
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    cell: {
        alignSelf: "center",
        display: "grid",
        justifyItems: "end",
    },
    spare: {
        display: { default: null, [CRAMPED_LEDGER]: "none" },
    },
    captionLine: {
        display: "flex",
        height: "0.875rem",
        marginBottom: "0.125rem",
    },
    caption: {
        color: color.mutedForeground,
        fontFamily: font.code,
        fontSize: typeScale.small,
        fontVariantNumeric: "tabular-nums",
        fontWeight: 400,
        lineHeight: "0.875rem",
    },
    value: {
        fontFamily: font.code,
        fontSize: typeScale.body,
        fontVariantNumeric: "tabular-nums",
        fontWeight: 600,
        lineHeight: "1.25rem",
        whiteSpace: "pre",
    },
    valueQuiet: {
        color: color.mutedForeground,
        fontSize: typeScale.small,
        fontWeight: 400,
    },
    head: {
        alignItems: "stretch",
        boxSizing: "border-box",
        height: chrome.strip,
    },
    corner: {
        backgroundColor: ledgerVariables.surface,
        insetInlineStart: 0,
        position: "sticky",
        zIndex: 1,
    },
    cornerWindow: {
        alignItems: "center",
        backgroundColor: color.muted,
        display: "flex",
        paddingInlineStart: "0.875rem",
    },
    headWindow: {
        backgroundColor: color.muted,
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        height: chrome.toolbar,
    },
});
