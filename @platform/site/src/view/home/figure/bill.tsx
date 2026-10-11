import { color, font, stroke } from "@destack/theme/tokens.stylex";
import { media } from "@destack/style/media.stylex";
import * as style from "@destack/style";
import type { JSX } from "@destack/view";

import { isEmptyOf, type Item, ItemBadge, type Lighting, type Line, marksOf } from "./item";
import { usageCostOf } from "./price";
import type { Stagger } from "./stagger";
import { palette } from "../../palette.stylex";
import { screen } from "../../layout/screen.stylex";
import { rates } from "../../content/generated/rates";

/** The bytes in a gigabyte, the unit a bill counts storage in. */
const GB = 1_000_000_000;

/**
 * Draw a figure's bill as a stack of equal blocks, one to each line, with the storage and compute its job takes and what it costs.
 *
 * While rented, each block wears its vendor's colour with a gap to the next, and costs what the vendor charges.
 * Once destacked, each block takes the one shared tint and costs its work at the plan's unit prices, and the gaps close once all are.
 * When the bill takes lighting, pointing at a block lights its line and pressing it chooses the line.
 * When the bill takes presses, pressing a block destacks its line alone.
 */
export function Bill(properties: {
    lines: readonly Line[];
    label: string;
    isOpenAt: Stagger;
    lighting?: Lighting;
    onToggle?: (row: number) => void;
    badgeOf?: (row: number) => JSX.Element;
}) {
    // light a block while a mouse or pen points at it, since a touch lights it by pressing
    const point = (event: PointerEvent, row: number | undefined) => {
        if (event.pointerType !== "touch") {
            properties.lighting?.onLight(row);
        }
    };

    // press a block: destack its line alone, or choose it, or let it go when it is the one lit
    const press = (row: number) => {
        if (properties.onToggle !== undefined) {
            properties.onToggle(row);
        } else if (properties.lighting !== undefined) {
            properties.lighting.onChoose(properties.lighting.lit === row ? undefined : row);
        }
    };

    // read which lines run in your space, and join the blocks into one once all of them do
    const isOwned = (index: number) => properties.isOpenAt(index + 1);
    const isWhole = () => properties.lines.every((_, index) => isOwned(index));
    const isAny = () => properties.lines.some((_, index) => isOwned(index));

    // total the work the lines take, which stays the same whoever runs it
    const storage = () => properties.lines.reduce((sum, line) => sum + line.workload.storage, 0);
    const compute = () => properties.lines.reduce((sum, line) => sum + line.workload.compute, 0);
    const isFloor = () => properties.lines.some((line) => line.workload.isAtLeast);

    // bill the lines still rented at their vendors' prices, and the lines in your space by the work they use
    const cost = () =>
        properties.lines.reduce(
            (sum, line, index) =>
                sum +
                (isOwned(index)
                    ? usageCostOf(line.workload, rates)
                    : monthlyOf(line.entry.stacked)),
            0,
        );

    return (
        <div data-component="Bill" {...style.attrs(styles.bill)}>
            <ol {...style.attrs(styles.blocks, isWhole() && styles.blocksWhole)}>
                {properties.lines.map((line, index) => (
                    <li
                        title={isOwned(index) ? line.entry.destacked.note : line.entry.stacked.note}
                        onPointerEnter={(event) => point(event, index + 1)}
                        onPointerLeave={(event) => point(event, undefined)}
                        onClick={() => press(index + 1)}
                        {...style.attributes(
                            [
                                styles.row,
                                styles.block,
                                (properties.lighting !== undefined ||
                                    properties.onToggle !== undefined) &&
                                    styles.pressable,
                                isWhole() && styles.blockWhole,
                                properties.lighting?.lit === index + 1 && styles.blockLit,
                                isEmptyOf(line.entry.stacked.mark) &&
                                    !isOwned(index) &&
                                    styles.blockEmpty,
                            ],
                            inlineOf(isOwned(index) ? palette.signal : line.tint),
                        )}
                    >
                        <span {...style.attrs(styles.claim)}>
                            <span {...style.attrs(styles.marks)}>
                                {isOwned(index)
                                    ? marksOf(line.entry.destacked.mark).map((mark) => (
                                          <ItemBadge mark={mark} />
                                      ))
                                    : marksOf(line.entry.stacked.mark)
                                          .slice(0, 3)
                                          .map((mark) => <ItemBadge mark={mark} />)}
                            </span>
                            <span {...style.attrs(styles.names)}>
                                <span {...style.attrs(styles.label)}>
                                    {line.entry.destacked.label}
                                </span>
                                <span {...style.attrs(styles.nameLine)}>
                                    <b {...style.attrs(styles.name)}>
                                        {isOwned(index)
                                            ? line.entry.destacked.name
                                            : line.entry.stacked.name}
                                    </b>
                                    {isOwned(index) ? properties.badgeOf?.(index + 1) : undefined}
                                </span>
                            </span>
                        </span>
                        <span {...style.attrs(styles.cell, styles.wide)}>
                            <span {...style.attrs(styles.label)}>GB</span>
                            <span {...style.attrs(styles.value, styles.valueQuiet)}>
                                {amountOf(line.workload.storage / GB)}
                            </span>
                        </span>
                        <span {...style.attrs(styles.cell, styles.wide)}>
                            <span {...style.attrs(styles.label)}>GB·h</span>
                            <span {...style.attrs(styles.value, styles.valueQuiet)}>
                                <span {...style.attrs(styles.floor)}>
                                    {line.workload.isAtLeast ? ">" : ""}
                                </span>
                                {amountOf(line.workload.compute)}
                            </span>
                        </span>
                        <span {...style.attrs(styles.cell)}>
                            <span {...style.attrs(styles.label)}>$/mo</span>
                            <span {...style.attrs(styles.value)}>
                                {isOwned(index)
                                    ? dollarsOf(usageCostOf(line.workload, rates))
                                    : costOf(line.entry.stacked)}
                            </span>
                        </span>
                    </li>
                ))}
            </ol>

            {/* total the bill under the stack: the work, and what it costs rented or in one space */}
            <p
                title={isAny() ? "Usage at cloud prices, free on your own machines" : undefined}
                {...style.attrs(styles.row, styles.total)}
            >
                <span {...style.attrs(styles.names)}>
                    <span {...style.attrs(styles.label)}>Total</span>
                    <b {...style.attrs(styles.name)}>{properties.label}</b>
                </span>
                <span {...style.attrs(styles.cell, styles.wide)}>
                    <span {...style.attrs(styles.label)}>GB</span>
                    <span {...style.attrs(styles.value, styles.valueQuiet)}>
                        {amountOf(storage() / GB)}
                    </span>
                </span>
                <span {...style.attrs(styles.cell, styles.wide)}>
                    <span {...style.attrs(styles.label)}>GB·h</span>
                    <span {...style.attrs(styles.value, styles.valueQuiet)}>
                        <span {...style.attrs(styles.floor)}>{isFloor() ? ">" : ""}</span>
                        {compute().toFixed(2)}
                    </span>
                </span>
                <span {...style.attrs(styles.cell)}>
                    <span {...style.attrs(styles.label)}>$/mo</span>
                    <span {...style.attrs(styles.value)}>{dollarsOf(cost())}</span>
                </span>
            </p>
        </div>
    );
}

/** Return what a rented item's fee comes to a month, or nothing for what it charges by use or bundles. */
function monthlyOf(item: Item): number {
    return item.price.kind === "fee" ? item.price.monthly : 0;
}

/** Return an amount to the hundredth, a floor for what rounds to nothing, or a dash where there is none. */
function amountOf(amount: number): string {
    if (amount === 0) {
        return "—";
    } else if (amount < 0.01) {
        return "<0.01";
    }

    return amount.toFixed(2);
}

/** Return dollars to the cent, so every amount takes the same width. */
function dollarsOf(amount: number): string {
    return `$${amount.toFixed(2)}`;
}

/** Return what a rented item costs a month, or a dash for what it charges by use or its vendor bills apart. */
function costOf(item: Item): string {
    return item.price.kind === "fee" && item.price.monthly > 0
        ? dollarsOf(item.price.monthly)
        : "—";
}

/** Return a block's inline colours: a faint fill and a solid edge in its tint. */
function inlineOf(tint: string) {
    return {
        "background-color": `color-mix(in srgb, ${tint} 12%, transparent)`,
        "border-inline-start-color": tint,
    };
}

/** The bill styles. */
const styles = style.create({
    bill: {
        display: "grid",
        gridTemplateRows: "minmax(0, 1fr) auto",
        minHeight: { default: 0, [screen.belowDesktop]: "30rem" },
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
    blocksWhole: {
        gap: 0,
    },
    block: {
        borderRadius: "4px",
        boxShadow: `inset 0 1.5px 0 ${color.background}, inset 0 -1.5px 0 ${color.background}`,
        flexBasis: 0,
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
    blockLit: {
        outlineColor: color.foreground,
        outlineOffset: "-1px",
        outlineStyle: "solid",
        outlineWidth: stroke.border,
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
        gap: "0.1875rem",
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
        textTransform: "uppercase",
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
    total: {
        borderTopColor: color.foreground,
        borderTopStyle: "solid",
        borderTopWidth: "1.5px",
        marginTop: "0.5rem",
        paddingBlock: "0.625rem",
    },
});
