import { color, font, stroke } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import type { JSX } from "@destack/view";

import { plate } from "./plate.stylex";
import type { Stagger } from "./stagger";
import { palette } from "../../palette.stylex";

/** What marks an item: a vendor's logo under `/logos`, or an icon under `/diagram` on a tinted tile. */
type ItemMark = { logo: string } | { icon: string; tint: string };

/** One item of a ledger, set as the stack figure sets its legend: a numbered label, a claim with its plates, and a line under it. */
export type Item = {
    /** The job the item does, set as the mono label. */
    label: string;
    mark: ItemMark;
    /** The claim: the product rented for the job, or what the job lets you do. */
    name: string;
    /** The plates beside the claim while rented, which owning drops. */
    chips: readonly string[];
    /** The line under the claim. */
    note: string;
};

/** How a ledger's items light together with the figure beside it: the lit number, and how to light one or none. */
export type Lighting = { lit: number | undefined; onLight: (number: number | undefined) => void };

/** Return a row number with two digits, as the stack figure numbers its rows. */
export function numberOf(number: number) {
    return String(number).padStart(2, "0");
}

/** Draw an item's mark: a vendor's logo, or an icon on its tinted tile. */
function ItemBadge(properties: { mark: ItemMark }) {
    // choose the logo or the tile inside an expression, so the badge follows its mark as the stack switches
    return (
        <>
            {"logo" in properties.mark ? (
                <img alt="" src={`/logos/${properties.mark.logo}`} {...style.attrs(styles.logo)} />
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
 * Draw a numbered ledger of a figure's items in the stack figure's legend style, in one or two columns, closed by a total.
 *
 * Each row keeps its number and label in place, and flips its claim from the stacked item to the destacked one on its own step.
 * When the ledger takes presses, pressing a row destacks its item alone.
 * When the ledger takes a badge, each destacked row shows it in place of the rented plates.
 */
export function Ledger(properties: {
    entries: readonly Entry[];
    total: readonly [label: string, value: string];
    isOpenAt: Stagger;
    onToggle?: (row: number) => void;
    columns?: 1 | 2;
    lighting?: Lighting;
    badgeOf?: (row: number) => JSX.Element;
}) {
    // draw one face of a row shown in its state and hidden in the other
    const face = (item: Item, isRented: boolean, step: number) => (
        <span
            aria-hidden={properties.isOpenAt(step) === isRented ? "true" : undefined}
            {...style.attrs(
                styles.face,
                properties.isOpenAt(step) === isRented && styles.faceHidden,
            )}
        >
            <span {...style.attrs(styles.claim)}>
                <ItemBadge mark={item.mark} />
                <b {...style.attrs(styles.name)}>{item.name}</b>
                {isRented ? (
                    <span {...style.attrs(styles.plates)}>
                        {item.chips.map((chip) => (
                            <span {...style.attrs(plate.plate, plate.closed)}>{chip}</span>
                        ))}
                    </span>
                ) : (
                    <span {...style.attrs(styles.plates)}>{properties.badgeOf?.(step)}</span>
                )}
            </span>
            <span {...style.attrs(styles.note)}>{item.note}</span>
        </span>
    );

    // draw one row: its number and label, and both faces of its claim
    const rowOf = (entry: Entry, index: number) => (
        <li
            {...(properties.onToggle === undefined ? {} : switchOf(index + 1, entry))}
            onPointerEnter={() => properties.lighting?.onLight(index + 1)}
            onPointerLeave={() => properties.lighting?.onLight(undefined)}
            {...style.attrs(
                styles.row,
                properties.columns === 1 && index > 0 && styles.rowRuled,
                properties.onToggle !== undefined && styles.rowPressable,
                properties.lighting?.lit === index + 1 && styles.rowLit,
            )}
        >
            <span
                {...style.attrs(
                    styles.label,
                    (properties.isOpenAt(index + 1) || properties.lighting?.lit === index + 1) &&
                        styles.labelLit,
                )}
            >
                {numberOf(index + 1)} {entry.destacked.label}
            </span>
            <span {...style.attrs(styles.faces)}>
                {face(entry.stacked, true, index + 1)}
                {face(entry.destacked, false, index + 1)}
            </span>
        </li>
    );

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
            <ol {...style.attrs(styles.rows, properties.columns === 1 && styles.rowsSingle)}>
                {properties.entries.map(rowOf)}
            </ol>
            <p {...style.attrs(styles.total)}>
                <span>{properties.total[0]}</span>
                <span>{properties.total[1]}</span>
            </p>
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
        gridTemplateRows: "minmax(0, 1fr) auto",
        minHeight: 0,
    },
    rows: {
        columnGap: 0,
        display: "grid",
        marginInline: "-0.75rem",
        gridAutoRows: "minmax(0, 1fr)",
        gridTemplateColumns: "repeat(2, minmax(0, 1fr))",
        listStyle: "none",
        margin: 0,
        padding: 0,
        "@media (max-width: 767px)": { gridTemplateColumns: "minmax(0, 1fr)" },
    },
    rowsSingle: {
        gridTemplateColumns: "minmax(0, 1fr)",
    },
    rowRuled: {
        "::before": {
            backgroundColor: color.border,
            content: "''",
            height: stroke.border,
            insetInline: "0.75rem",
            position: "absolute",
            top: 0,
        },
    },
    row: {
        alignContent: "center",
        backgroundColor: "transparent",
        borderRadius: "6px",
        cursor: "default",
        display: "grid",
        minWidth: 0,
        paddingBlock: "0.625rem",
        paddingInline: "0.75rem",
        position: "relative",
        rowGap: "0.3125rem",
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
        fontSize: "0.6875rem",
        letterSpacing: "0.1em",
        lineHeight: "1rem",
        textTransform: "uppercase",
        transition: "color 250ms ease",
        whiteSpace: "nowrap",
    },
    labelLit: {
        color: color.foreground,
    },
    claim: {
        alignItems: "center",
        display: "flex",
        flexWrap: "wrap",
        rowGap: "0.25rem",
        columnGap: "0.5rem",
        minWidth: 0,
    },
    name: {
        fontSize: "1rem",
        fontWeight: 600,
        lineHeight: "1.375rem",
        whiteSpace: "nowrap",
    },
    plates: {
        display: "flex",
        flexShrink: 0,
        gap: "0.25rem",
        marginLeft: "auto",
    },
    logo: {
        borderRadius: "5px",
        flexShrink: 0,
        height: "1.375rem",
        width: "1.375rem",
    },
    tile: {
        alignItems: "center",
        borderRadius: "5px",
        boxShadow: `inset 0 0 0 1.5px color-mix(in srgb, ${palette.ink} 22%, transparent)`,
        display: "inline-flex",
        flexShrink: 0,
        height: "1.375rem",
        justifyContent: "center",
        width: "1.375rem",
    },
    glyph: {
        backgroundColor: palette.cream,
        height: "0.875rem",
        maskPosition: "center",
        maskRepeat: "no-repeat",
        maskSize: "contain",
        width: "0.875rem",
    },
    faces: {
        display: "grid",
        minWidth: 0,
    },
    face: {
        display: "grid",
        gridArea: "1 / 1",
        minWidth: 0,
        rowGap: "0.3125rem",
        transitionDuration: "240ms",
        transitionProperty: "opacity, transform",
        transitionTimingFunction: "cubic-bezier(0.23, 1, 0.32, 1)",
    },
    faceHidden: {
        opacity: 0,
        pointerEvents: "none",
        transform: "perspective(40rem) rotateX(70deg)",
    },
    note: {
        color: color.mutedForeground,
        fontSize: "0.8125rem",
        lineHeight: "1.125rem",
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    total: {
        alignItems: "center",
        borderTopColor: color.border,
        borderTopStyle: "solid",
        borderTopWidth: stroke.border,
        boxSizing: "border-box",
        display: "flex",
        justifyContent: "space-between",
        fontFamily: font.code,
        fontSize: "0.875rem",
        fontWeight: 700,
        height: "2.25rem",
        margin: 0,
    },
});
