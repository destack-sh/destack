import { color } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";

import { plate } from "../style/plate.stylex";
import { tokens } from "../style/tokens.stylex";

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

/** Return a callout number with two digits, as the stack figure numbers its rows. */
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

/** Draw a numbered ledger of a figure's items in the stack figure's legend style, in two columns or one, closed by a total row. */
export function Ledger(properties: {
    items: readonly Item[];
    total: readonly [label: string, value: string];
    isOpen: boolean;
    lighting?: Lighting;
    isSingle?: boolean;
    isNoted?: boolean;
}) {
    return (
        <div {...stylex.attrs(styles.ledger, properties.isNoted === true && styles.packed)}>
            <ol
                {...stylex.attrs(
                    styles.rows,
                    properties.isSingle === true && styles.single,
                    properties.isNoted === true && styles.packedRows,
                )}
            >
                {properties.items.map((item, index) => (
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
                                (properties.isOpen || properties.lighting?.lit === index + 1) &&
                                    styles.labelLit,
                            )}
                        >
                            {numberOf(index + 1)} {item.label}
                        </span>
                        <span {...stylex.attrs(styles.claim)}>
                            <ItemBadge mark={item.mark} />
                            <b {...stylex.attrs(styles.name)}>{item.name}</b>
                            <span {...stylex.attrs(styles.plates)}>
                                {item.chips.map((chip) => (
                                    <span
                                        {...stylex.attrs(
                                            plate.plate,
                                            !properties.isOpen && plate.closed,
                                        )}
                                    >
                                        {chip}
                                    </span>
                                ))}
                            </span>
                        </span>
                        <span
                            {...stylex.attrs(
                                styles.note,
                                properties.isNoted === true && styles.noted,
                            )}
                        >
                            {item.note}
                        </span>
                    </li>
                ))}
            </ol>
            <p {...stylex.attrs(styles.total)}>
                <span>{properties.total[0]}</span>
                <span>{properties.total[1]}</span>
            </p>
        </div>
    );
}

/** The ledger styles. */
const styles = stylex.create({
    ledger: {
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
    packed: {
        alignContent: "start",
        gridTemplateRows: "auto auto",
        rowGap: "0.75rem",
    },
    packedRows: {
        gridAutoRows: "auto",
        rowGap: "0.75rem",
    },
    single: {
        gridTemplateColumns: "minmax(0, 1fr)",
    },
    row: {
        alignContent: "center",
        borderRadius: "6px",
        cursor: "default",
        display: "grid",
        minWidth: 0,
        paddingBlock: "0.625rem",
        paddingInline: "0.75rem",
        rowGap: "0.3125rem",
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
        whiteSpace: "nowrap",
    },
    labelLit: {
        color: color.foreground,
    },
    claim: {
        alignItems: "center",
        display: "flex",
        gap: "0.5rem",
        minWidth: 0,
    },
    name: {
        fontSize: "1rem",
        fontWeight: 600,
        lineHeight: "1.375rem",
        overflow: "hidden",
        textOverflow: "ellipsis",
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
    noted: {
        display: "block",
        whiteSpace: "normal",
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
    total: {
        borderTopColor: tokens.rule,
        borderTopStyle: "solid",
        borderTopWidth: tokens.hairline,
        display: "flex",
        fontFamily: tokens.monoFont,
        fontSize: "0.875rem",
        fontWeight: 700,
        justifyContent: "space-between",
        margin: 0,
        paddingTop: "0.625rem",
    },
});
