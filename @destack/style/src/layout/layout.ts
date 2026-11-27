import { type Message, t } from "@destack/locale";
import { schema } from "@destack/schema";
import { BoxSides } from "../box/index.ts";
import { Dimension } from "../dimension/index.ts";

/** A way a layout lines its children up, as flexbox's directions do. */
export const LayoutDirection = schema.enum(["row", "column"]);
/** A way a layout lines its children up. */
export type LayoutDirection = schema.Infer<typeof LayoutDirection>;

/** Where a layout places its children across its direction, as flexbox aligns items. */
export const LayoutAlign = schema.enum(["start", "center", "end", "stretch"]);
/** Where a layout places its children across its direction. */
export type LayoutAlign = schema.Infer<typeof LayoutAlign>;

/** How a layout spreads its children along its direction, as flexbox justifies content. */
export const LayoutJustify = schema.enum(["start", "center", "end", "space-between"]);
/** How a layout spreads its children along its direction. */
export type LayoutJustify = schema.Infer<typeof LayoutJustify>;

/** The ways a layout lines its children up, titled and described. */
export const LAYOUT_DIRECTIONS = {
    row: {
        title: t.context("option")`Row`,
        description: t`Side by side, in the text's direction.`,
    },
    column: { title: t.context("option")`Column`, description: t`One below the other.` },
} satisfies Record<LayoutDirection, { readonly title: Message; readonly description: Message }>;

/** Where a layout places its children across its direction, titled. */
export const LAYOUT_ALIGNS = {
    start: { title: t.context("option")`Start` },
    center: { title: t.context("option")`Center` },
    end: { title: t.context("option")`End` },
    stretch: { title: t.context("option")`Stretch` },
} satisfies Record<LayoutAlign, { readonly title: Message }>;

/** How a layout spreads its children along its direction, titled. */
export const LAYOUT_JUSTIFIES = {
    start: { title: t.context("option")`Start` },
    center: { title: t.context("option")`Center` },
    end: { title: t.context("option")`End` },
    "space-between": { title: t.context("option")`Space between` },
} satisfies Record<LayoutJustify, { readonly title: Message }>;

/** How a box lays its children out in a line, as flexbox and design tools' auto layout do. */
const layoutJson = schema
    .object({
        /** The way it lines its children up. */
        direction: LayoutDirection,
        /** The room between children. */
        gap: Dimension.bindable,
        /** Where it places them across its direction. */
        align: LayoutAlign,
        /** How it spreads them along its direction. */
        justify: LayoutJustify,
        /** Whether children that do not fit go on the next line. */
        isWrapping: schema.boolean(),
        /** The room inside its edges. */
        padding: BoxSides,
    })
    .meta({ id: "style.layout", title: "Layout" });
/** How a box lays its children out in a line. */
export type Layout = schema.Infer<typeof layoutJson>;

/** How a box lays its children out in a line, as flexbox and design tools' auto layout do. */
export const Layout = Object.assign(layoutJson, {
    /** Lay a box out as CSS's flexbox properties. */
    css(layout: Layout): LayoutStyle {
        return {
            display: "flex",
            flexDirection: layout.direction,
            gap: Dimension.css(layout.gap),
            alignItems: layout.align,
            justifyContent: layout.justify,
            flexWrap: layout.isWrapping ? "wrap" : "nowrap",
            padding: BoxSides.css(layout.padding),
        };
    },
});

/** A layout as CSS properties. */
export interface LayoutStyle {
    /** The display, always flex. */
    readonly display: "flex";
    /** The direction. */
    readonly flexDirection: string;
    /** The gap. */
    readonly gap: string;
    /** The alignment across. */
    readonly alignItems: string;
    /** The spread along. */
    readonly justifyContent: string;
    /** The wrapping. */
    readonly flexWrap: string;
    /** The padding. */
    readonly padding: string;
}
