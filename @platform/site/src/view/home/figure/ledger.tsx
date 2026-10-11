import { color, font, stroke } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import type { JSX } from "@destack/view";

import type { Stagger } from "./stagger";
import { dollarsOf, type Price } from "./price";
import { plan } from "../../content/generated/plan";
import { palette } from "../../palette.stylex";
import { media } from "@destack/style/media.stylex";

/** What marks an item: a vendor's logo under `/logos`, an icon under `/diagram` on a tinted tile, or an icon's empty outline where nothing does the job yet. */
type ItemMark = { logo: string } | { icon: string; tint: string } | { placeholder: string };

/** One item of a ledger, set as a line of a bill: a numbered label, a claim, a line under it, and its price. */
export type Item = {
    /** The job the item does, set as the mono label. */
    label: string;
    /** The mark, or the marks side by side of every product that does the job. */
    mark: ItemMark | readonly ItemMark[];
    /** The claim: the product rented for the job, or what the job lets you do. */
    name: string;
    /** The line under the claim. */
    note: string;
    /** What the item costs a month. */
    price: Price;
    /** The item's line in the ledger's second column, such as the data it holds or the glue it needs. */
    detail: string;
};

/** How a ledger's items light together with the figure beside it: the lit number, how to light one while pointing, and how to choose one by pressing. */
export type Lighting = {
    lit: number | undefined;
    onLight: (number: number | undefined) => void;
    onChoose: (number: number | undefined) => void;
};

/** Return a price as its bill shows it: the amount, and how it adds up. */
function linesOf(price: Price): readonly [amount: string, basis: string] {
    if (price.kind === "fee") {
        return [price.monthly === 0 ? "—" : dollarsOf(price.monthly), price.basis];
    } else if (price.kind === "usage") {
        return ["by use", price.basis];
    } else if (price.kind === "bundled") {
        return ["bundled", price.basis];
    }

    return ["—", `included in ${plan.name}`];
}

/** Return an item's marks as a list, one mark or several. */
function marksOf(mark: ItemMark | readonly ItemMark[]): readonly ItemMark[] {
    return "logo" in mark || "icon" in mark || "placeholder" in mark ? [mark] : mark;
}

/** Report whether every mark of an item is an empty outline, so nothing does its job yet. */
function isEmptyOf(mark: ItemMark | readonly ItemMark[]) {
    return marksOf(mark).every((each) => "placeholder" in each);
}

/** Draw an item's mark: a vendor's logo, an icon on its tinted tile, or an icon's empty outline. */
function ItemBadge(properties: { mark: ItemMark }) {
    // choose the logo, the outline or the tile inside an expression, so the badge follows its mark as the stack switches
    return (
        <>
            {"logo" in properties.mark ? (
                <img alt="" src={`/logos/${properties.mark.logo}`} {...style.attrs(styles.logo)} />
            ) : "placeholder" in properties.mark ? (
                <span {...style.attrs(styles.tile, styles.placeholder)}>
                    <span
                        style={{ "mask-image": `url(/diagram/${properties.mark.placeholder}.svg)` }}
                        {...style.attrs(styles.glyph, styles.placeholderGlyph)}
                    />
                </span>
            ) : (
                <span
                    style={{ "background-color": properties.mark.tint }}
                    {...style.attrs(styles.tile)}
                >
                    <span
                        style={{ "mask-image": `url(/diagram/${properties.mark.icon}.svg)` }}
                        {...style.attrs(styles.glyph)}
                    />
                </span>
            )}
        </>
    );
}

/** One row of a ledger in both states of the stack. */
export type Entry = { stacked: Item; destacked: Item };

/**
 * Draw a numbered ledger of a figure's items as a bill, one ruled line per item, closed by a total.
 *
 * The lines share the ledger's height and one set of columns: mark, claim, price and the ledger's detail.
 * Each row keeps its number and label in place, and flips each cell from the stacked item to the destacked one on its own step.
 * A row's note shows as its title.
 * When the ledger takes presses, pressing a row destacks its item alone.
 * When the ledger takes a badge, each destacked row shows it beside its claim.
 */
export function Ledger(properties: {
    entries: readonly Entry[];
    detail: string;
    total: readonly [label: string, amount: string, detail: string];
    isOpenAt: Stagger;
    onToggle?: (row: number) => void;
    lighting?: Lighting;
    badgeOf?: (row: number) => JSX.Element;
}) {
    // flip one cell between the stacked item's content and the destacked one's on the row's step
    const cell = (
        step: number,
        cellStyle: style.Styles,
        contentOf: (item: Item, isRented: boolean) => JSX.Element,
        entry: Entry,
    ) => (
        <span {...style.attrs(styles.cell, cellStyle)}>
            {[entry.stacked, entry.destacked].map((item, side) => (
                <span
                    aria-hidden={properties.isOpenAt(step) === (side === 0) ? "true" : undefined}
                    {...style.attrs(
                        styles.face,
                        isEmptyOf(item.mark) && styles.faceEmpty,
                        properties.isOpenAt(step) === (side === 0) && styles.faceHidden,
                    )}
                >
                    {contentOf(item, side === 0)}
                </span>
            ))}
        </span>
    );

    // draw one row: its number and label, then its claim, data and price, each flipping on its own
    const rowOf = (entry: Entry, index: number) => (
        <li
            title={properties.isOpenAt(index + 1) ? entry.destacked.note : entry.stacked.note}
            {...(properties.onToggle === undefined
                ? chooseOf(index + 1)
                : switchOf(index + 1, entry))}
            onPointerEnter={(event) => point(event, index + 1)}
            onPointerLeave={(event) => point(event, undefined)}
            {...style.attrs(
                styles.row,
                (properties.onToggle !== undefined || properties.lighting !== undefined) &&
                    styles.rowPressable,
                properties.lighting?.lit === index + 1 && styles.rowLit,
            )}
        >
            {cell(
                index + 1,
                styles.markCell,
                (item) => (
                    <span {...style.attrs(styles.marks)}>
                        {marksOf(item.mark).map((mark) => (
                            <ItemBadge mark={mark} />
                        ))}
                    </span>
                ),
                entry,
            )}
            <span {...style.attrs(styles.claimCell)}>
                <span
                    {...style.attrs(
                        styles.label,
                        (properties.isOpenAt(index + 1) ||
                            properties.lighting?.lit === index + 1) &&
                            styles.labelLit,
                    )}
                >
                    {entry.destacked.label}
                </span>
                {cell(
                    index + 1,
                    styles.nameCell,
                    (item, isRented) => (
                        <span {...style.attrs(styles.claim)}>
                            <b {...style.attrs(styles.name)}>{item.name}</b>
                            {isRented ? undefined : properties.badgeOf?.(index + 1)}
                        </span>
                    ),
                    entry,
                )}
            </span>
            {cell(
                index + 1,
                [styles.number, styles.amountColumn],
                (item) => (
                    <span
                        title={linesOf(item.price)[1]}
                        {...style.attrs(
                            (item.price.kind !== "fee" || item.price.monthly === 0) && styles.quiet,
                        )}
                    >
                        {linesOf(item.price)[0]}
                    </span>
                ),
                entry,
            )}
            {cell(
                index + 1,
                [styles.number, styles.dataColumn],
                (item) => (
                    <span {...style.attrs(item.detail === "—" && styles.quiet)}>{item.detail}</span>
                ),
                entry,
            )}
        </li>
    );

    // light a row while a mouse or pen points at it, since a touch lights it by pressing
    const point = (event: PointerEvent, row: number | undefined) => {
        if (event.pointerType !== "touch") {
            properties.lighting?.onLight(row);
        }
    };

    // make a row a button that chooses its page, or lets it go when pressed again
    const chooseOf = (row: number): JSX.LiHTMLAttributes<HTMLLIElement> => {
        // pass a ledger without lighting as it is
        const lighting = properties.lighting;
        if (lighting === undefined) {
            return {};
        }

        // choose the row, or let it go when it is the one lit
        const choose = () => lighting.onChoose(lighting.lit === row ? undefined : row);

        return {
            role: "button",
            tabindex: "0",
            "aria-pressed": lighting.lit === row ? "true" : "false",
            onClick: choose,
            onKeyDown: (event: KeyboardEvent) => {
                if (event.key === "Enter" || event.key === " ") {
                    event.preventDefault();
                    choose();
                }
            },
        };
    };

    // make a row a switch that destacks its item alone, when the ledger takes presses
    const switchOf = (row: number, entry: Entry): JSX.LiHTMLAttributes<HTMLLIElement> => ({
        role: "switch",
        tabindex: "0",
        "aria-checked": properties.isOpenAt(row) ? "true" : "false",
        title: properties.isOpenAt(row)
            ? `Rent ${entry.stacked.name} again`
            : `Destack ${entry.destacked.label} alone`,
        onClick: () => properties.onToggle?.(row),
        onKeyDown: (event: KeyboardEvent) => {
            if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                properties.onToggle?.(row);
            }
        },
    });

    return (
        <div {...style.attrs(styles.ledger)}>
            <ol {...style.attrs(styles.rows, styles.lines(properties.entries.length))}>
                {/* name the two quantities every line bills */}
                <li aria-hidden="true" {...style.attrs(styles.head)}>
                    <span {...style.attrs(styles.headCell, styles.amountColumn)}>Cost / mo</span>
                    <span {...style.attrs(styles.headCell, styles.dataColumn)}>
                        {properties.detail}
                    </span>
                </li>
                {properties.entries.map(rowOf)}
                {/* total the bill under its columns */}
                <li {...style.attrs(styles.total)}>
                    <span {...style.attrs(styles.totalLabel)}>{properties.total[0]}</span>
                    <span {...style.attrs(styles.totalNumber, styles.amountColumn)}>
                        {properties.total[1]}
                    </span>
                    <span {...style.attrs(styles.totalNumber, styles.dataColumn)}>
                        {properties.total[2]}
                    </span>
                </li>
            </ol>
        </div>
    );
}

/** The ledger styles. */
const styles = style.create({
    ledger: {
        borderBlockColor: color.border,
        borderBlockStyle: "solid",
        borderBlockWidth: stroke.border,
        boxSizing: "border-box",
        display: "grid",
        gridTemplateRows: "minmax(0, 1fr)",
        minHeight: 0,
    },
    rows: {
        display: "grid",
        gridAutoRows: { default: "minmax(2.25rem, 1fr)", [media.maxMd]: "auto" },
        gridTemplateColumns: {
            default: "max-content minmax(0, 1fr) max-content max-content",
            [media.maxMd]: "max-content minmax(0, 1fr) max-content",
        },
        listStyle: "none",
        margin: 0,
        minHeight: 0,
        padding: 0,
    },
    lines: (count: number) => ({
        gridTemplateRows: `auto repeat(${count}, minmax(2.25rem, 1fr)) auto`,
    }),
    row: {
        alignItems: "center",
        backgroundColor: "transparent",
        cursor: "default",
        display: "grid",
        gridColumn: "1 / -1",
        gridTemplateColumns: "subgrid",
        minWidth: 0,
        paddingBlock: { default: "0", [media.maxMd]: "0.375rem" },
        transition: "background-color 250ms ease",
    },
    rowPressable: {
        cursor: "pointer",
    },
    rowLit: {
        backgroundColor: `color-mix(in srgb, ${palette.signal} 10%, transparent)`,
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
        transition: "color 250ms ease",
        whiteSpace: "nowrap",
    },
    labelLit: {
        color: color.foreground,
    },
    cell: {
        display: "grid",
        minWidth: 0,
    },
    markCell: {
        gridColumn: "1",
        paddingInlineEnd: "0.75rem",
        gridRow: { default: "1", [media.maxMd]: "1 / 3" },
    },
    claimCell: {
        display: "grid",
        gridColumn: "2",
        minWidth: 0,
        gridRow: { default: "1", [media.maxMd]: "1 / 3" },
    },
    nameCell: {
        minWidth: 0,
    },
    quiet: {
        color: color.mutedForeground,
        fontWeight: 400,
    },
    number: {
        fontFamily: font.code,
        fontSize: "0.875rem",
        fontWeight: 700,
        justifyItems: "end",
        paddingInlineStart: "1.25rem",
        whiteSpace: "nowrap",
    },
    amountColumn: {
        gridColumn: "3",
        gridRow: "1",
    },
    dataColumn: {
        gridColumn: { default: "4", [media.maxMd]: "3" },
        gridRow: { default: "1", [media.maxMd]: "2" },
    },
    head: {
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        display: "grid",
        gridColumn: "1 / -1",
        gridTemplateColumns: "subgrid",
        paddingBlock: "0.375rem",
    },
    headCell: {
        color: color.mutedForeground,
        fontFamily: font.code,
        fontSize: "0.6875rem",
        letterSpacing: "0.1em",
        paddingInlineStart: "1.25rem",
        textAlign: "end",
        textTransform: "uppercase",
    },
    claim: {
        alignItems: "center",
        columnGap: "0.5rem",
        display: "flex",
        minWidth: 0,
    },
    name: {
        fontSize: { default: "1rem", [media.maxMd]: "0.875rem" },
        fontWeight: 600,
        lineHeight: "1.25rem",
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    placeholder: {
        borderColor: color.border,
        borderStyle: "dashed",
        borderWidth: "1.5px",
        boxShadow: "none",
        boxSizing: "border-box",
    },
    placeholderGlyph: {
        backgroundColor: color.mutedForeground,
        opacity: 0.5,
    },
    faceEmpty: {
        color: color.mutedForeground,
        opacity: 0.45,
    },
    marks: {
        display: "flex",
        flexShrink: 0,
        gap: "0.1875rem",
    },
    logo: {
        backgroundColor: palette.logoGround,
        borderRadius: "5px",
        flexShrink: 0,
        height: "1.75rem",
        width: "1.75rem",
    },
    tile: {
        alignItems: "center",
        borderRadius: "5px",
        boxShadow: `inset 0 0 0 1.5px color-mix(in srgb, ${palette.ink} 22%, transparent)`,
        display: "inline-flex",
        flexShrink: 0,
        height: "1.75rem",
        justifyContent: "center",
        width: "1.75rem",
    },
    glyph: {
        backgroundColor: palette.cream,
        height: "1.0625rem",
        maskPosition: "center",
        maskRepeat: "no-repeat",
        maskSize: "contain",
        width: "1.0625rem",
    },
    face: {
        alignItems: "center",
        display: "flex",
        gridArea: "1 / 1",
        minWidth: 0,
        transitionDuration: "240ms",
        transitionProperty: "opacity, transform",
        transitionTimingFunction: "cubic-bezier(0.23, 1, 0.32, 1)",
    },
    faceHidden: {
        opacity: 0,
        pointerEvents: "none",
        transform: "perspective(40rem) rotateX(70deg)",
    },
    total: {
        alignItems: "center",
        borderTopColor: color.foreground,
        borderTopStyle: "solid",
        borderTopWidth: "1.5px",
        display: "grid",
        fontFamily: font.code,
        fontSize: { default: "0.875rem", [media.maxMd]: "1rem" },
        fontWeight: 700,
        gridColumn: "1 / -1",
        gridTemplateColumns: "subgrid",
        paddingBlock: "0.5rem",
        whiteSpace: "nowrap",
    },
    totalLabel: {
        gridColumn: "1 / 3",
        overflow: "hidden",
        textOverflow: "ellipsis",
        gridRow: { default: "1", [media.maxMd]: "1 / 3" },
    },
    totalNumber: {
        paddingInlineStart: "1.25rem",
        textAlign: "end",
    },
});
