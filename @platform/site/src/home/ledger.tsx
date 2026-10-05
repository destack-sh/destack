import { color } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";

import { plate } from "../style/plate.stylex";
import { tokens } from "../style/tokens.stylex";
import type { Stagger } from "./stagger";

/** What marks an item: a vendor's logo under `/logos`, or an icon under `/diagram` on a tinted tile. */
type ItemMark = { logo: string } | { icon: string; tint: string };

/** One item of a ledger, set as the stack figure sets its legend: a numbered label, a claim with its plates, and a line under it. */
export type Item = {
    /** The job the item does, set as the mono label. */
    label: string;
    mark: ItemMark;
    /** The claim: the product rented for the job, or what the job lets you do. */
    name: string;
    /** The plates beside the claim: dashed while rented, solid once owned. */
    chips: readonly string[];
    /** The mono line under the claim. */
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
                <img alt="" src={`/logos/${properties.mark.logo}`} {...stylex.attrs(styles.logo)} />
            ) : (
                <span
                    style={{ "background-color": properties.mark.tint }}
                    {...stylex.attrs(styles.tile)}
                >
                    <span
                        style={{ "mask-image": `url(/diagram/${properties.mark.icon}.svg)` }}
                        {...stylex.attrs(styles.glyph)}
                    />
                </span>
            )}
        </>
    );
}

/** One row of a ledger in both states of the stack. */
export type Entry = { stacked: Item; destacked: Item };

/** A ledger's closing line in both states of the stack. */
export type Total = Record<"stacked" | "destacked", readonly [label: string, value: string]>;

/**
 * Draw a numbered ledger of a figure's items in the stack figure's legend style, in two columns, closed by a total.
 *
 * Each row keeps its number and label in place, and flips its claim from the stacked item to the destacked one on its own step.
 */
export function Ledger(properties: {
    entries: readonly Entry[];
    total: Total;
    isOpenAt: Stagger;
    lighting?: Lighting;
}) {
    // draw one face of a row shown in its state and hidden in the other
    const face = (item: Item, isRented: boolean, step: number) => (
        <span
            aria-hidden={properties.isOpenAt(step) === isRented ? "true" : undefined}
            {...stylex.attrs(
                styles.face,
                properties.isOpenAt(step) === isRented && styles.faceHidden,
            )}
        >
            <span {...stylex.attrs(styles.claim)}>
                <ItemBadge mark={item.mark} />
                <b {...stylex.attrs(styles.name)}>{item.name}</b>
                <span {...stylex.attrs(styles.plates)}>
                    {item.chips.map((chip) => (
                        <span {...stylex.attrs(plate.plate, isRented && plate.closed)}>{chip}</span>
                    ))}
                </span>
            </span>
            <span {...stylex.attrs(styles.note)}>{item.note}</span>
        </span>
    );

    return (
        <div {...stylex.attrs(styles.ledger)}>
            <ol {...stylex.attrs(styles.rows)}>
                {properties.entries.map((entry, index) => (
                    <li
                        onPointerEnter={() => properties.lighting?.onLight(index + 1)}
                        onPointerLeave={() => properties.lighting?.onLight(undefined)}
                        {...stylex.attrs(
                            styles.row,
                            properties.lighting?.lit === index + 1 && styles.rowLit,
                        )}
                    >
                        <span
                            {...stylex.attrs(
                                styles.label,
                                (properties.isOpenAt(index + 1) ||
                                    properties.lighting?.lit === index + 1) &&
                                    styles.labelLit,
                            )}
                        >
                            {numberOf(index + 1)} {entry.destacked.label}
                        </span>
                        <span {...stylex.attrs(styles.faces)}>
                            {face(entry.stacked, true, index + 1)}
                            {face(entry.destacked, false, index + 1)}
                        </span>
                    </li>
                ))}
            </ol>
            <p {...stylex.attrs(styles.total)}>
                {(["stacked", "destacked"] as const).map((state) => (
                    <span
                        {...stylex.attrs(
                            styles.face,
                            styles.totalFace,
                            properties.isOpenAt(properties.entries.length + 1) ===
                                (state === "stacked") && styles.faceHidden,
                        )}
                    >
                        <span>{properties.total[state][0]}</span>
                        <span>{properties.total[state][1]}</span>
                    </span>
                ))}
            </p>
        </div>
    );
}

/** The ledger styles. */
const styles = stylex.create({
    ledger: {
        borderBlockColor: tokens.rule,
        borderBlockStyle: "solid",
        borderBlockWidth: tokens.hairline,
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
    row: {
        alignContent: "center",
        backgroundColor: "rgb(255 121 46 / 0%)",
        borderRadius: "6px",
        cursor: "default",
        display: "grid",
        minWidth: 0,
        paddingBlock: "0.625rem",
        paddingInline: "0.75rem",
        rowGap: "0.3125rem",
        transition: "background-color 250ms ease",
    },
    rowLit: {
        backgroundColor: "rgb(255 121 46 / 10%)",
    },
    label: {
        color: color.mutedForeground,
        fontFamily: tokens.monoFont,
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
        gap: "0.25rem 0.5rem",
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
    note: {
        display: { default: "block", "@media (max-height: 999px)": "none" },
        fontFamily: tokens.monoFont,
        fontSize: "0.75rem",
        letterSpacing: "0.02em",
        lineHeight: "1.125rem",
        opacity: 0.8,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
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
        boxShadow: `inset 0 0 0 1.5px color-mix(in srgb, ${tokens.signalInk} 22%, transparent)`,
        display: "inline-flex",
        flexShrink: 0,
        height: "1.375rem",
        justifyContent: "center",
        width: "1.375rem",
    },
    glyph: {
        backgroundColor: tokens.cream,
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
        transitionDuration: "450ms",
        transitionProperty: "opacity, transform",
        transitionTimingFunction: "cubic-bezier(0.6, 0, 0.2, 1)",
    },
    faceHidden: {
        opacity: 0,
        pointerEvents: "none",
        transform: "perspective(40rem) rotateX(90deg)",
    },
    totalFace: {
        display: "flex",
        justifyContent: "space-between",
    },
    total: {
        alignItems: "center",
        borderTopColor: tokens.rule,
        borderTopStyle: "solid",
        borderTopWidth: tokens.hairline,
        boxSizing: "border-box",
        display: "grid",
        fontFamily: tokens.monoFont,
        fontSize: "0.875rem",
        fontWeight: 700,
        height: "2.25rem",
        margin: 0,
    },
});
