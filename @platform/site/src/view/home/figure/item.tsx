import { color } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";

import type { Price, Workload } from "./price";
import { Tile } from "./tile";
import { palette } from "../../palette.stylex";

/** What marks an item: a vendor's logo or icon, an app of your space, or an outline. */
export type ItemMark =
    | { logo: string }
    | { icon: string; tint: string }
    | { app: string }
    | { placeholder: string };

/** One item of a bill: a label for its job, its mark and name, and its price. */
export type Item = {
    /** The job the item does, set as the mono label. */
    label: string;
    /** The mark, or the marks side by side of every product that does the job. */
    mark: ItemMark | readonly ItemMark[];
    /** The product rented for the job, or the app of your space that does it. */
    name: string;
    /** What the item costs a month. */
    price: Price;
};

/** One row of a bill in both states of the stack. */
export type Entry = { stacked: Item; destacked: Item };

/** One line of a bill. */
export type Line = { entry: Entry; workload: Workload; tint: string };

/** How a bill's lines light together with the figure beside it. */
export type Lighting = {
    lit: number | undefined;
    onLight: (number: number | undefined) => void;
    onChoose: (number: number | undefined) => void;
    chosen: number | undefined;
};

/** The colour a vendor's block wears in a bill, for vendors whose mark is black or a logo. */
const accents: Readonly<Record<string, string>> = {
    Notion: "#9f6b53",
    GitHub: "#2da44e",
    Framer: "#d9318f",
    Superhuman: "#7c3aed",
    WorkOS: "#0ea5e9",
    Doppler: "#ec4899",
    Supabase: "#3ecf8e",
    Resend: "#f97316",
    Temporal: "#14b8a6",
    PostHog: "#f54e00",
    Vercel: "#64748b",
    ChatGPT: "#10a37f",
    Grok: "#475569",
    "Cal.com": "#6366f1",
};

/** Return the colour a vendor's block wears: its accent where it has one, or its mark's tint. */
export function accentOf(vendor: string, tint: string): string {
    return accents[vendor] ?? tint;
}

/** Return an item's marks as a list, one mark or several. */
export function marksOf(mark: ItemMark | readonly ItemMark[]): readonly ItemMark[] {
    return "logo" in mark || "icon" in mark || "app" in mark || "placeholder" in mark
        ? [mark]
        : mark;
}

/** Report whether every mark of an item is an empty outline, so nothing does its job yet. */
export function isEmptyOf(mark: ItemMark | readonly ItemMark[]) {
    return marksOf(mark).every((each) => "placeholder" in each);
}

/** Draw an item's mark: a vendor's logo or icon, an app's brand tile, or an outline. */
export function ItemBadge(properties: { mark: ItemMark }) {
    // choose the logo, the outline or the tile inside an expression, so the badge follows its mark as the stack switches
    return (
        <>
            {"logo" in properties.mark ? (
                <img alt="" src={`/logos/${properties.mark.logo}`} {...style.attrs(styles.logo)} />
            ) : "app" in properties.mark ? (
                <Tile icon={properties.mark.app} size="large" />
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

/** The item mark styles. */
const styles = style.create({
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
});
