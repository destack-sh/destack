import { type Message, t } from "@destack/locale";
import { schema } from "@destack/schema";
import { Dimension } from "../dimension/index.ts";

/** A CSS filter that scales a color quality by an amount, 1 leaving it as it is. */
export const FilterAmountKind = schema.enum([
    "brightness",
    "contrast",
    "saturate",
    "grayscale",
    "sepia",
    "invert",
    "opacity",
]);
/** A CSS filter that scales a color quality by an amount. */
export type FilterAmountKind = schema.Infer<typeof FilterAmountKind>;

/** The kinds of CSS filters: a blur, a turn of hue, or an amount of a color quality. */
export const FilterKind = schema.enum(["blur", "hue-rotate", ...FilterAmountKind.options]);
/** The kinds of CSS filters. */
export type FilterKind = schema.Infer<typeof FilterKind>;

/** The kinds of CSS filters, titled. */
export const FILTER_KINDS = {
    blur: { title: t.context("option")`Blur` },
    "hue-rotate": { title: t.context("option")`Hue rotation` },
    brightness: { title: t.context("option")`Brightness` },
    contrast: { title: t.context("option")`Contrast` },
    saturate: { title: t.context("option")`Saturation` },
    grayscale: { title: t.context("option")`Grayscale` },
    sepia: { title: t.context("option")`Sepia` },
    invert: { title: t.context("option")`Invert` },
    opacity: { title: t.context("option")`Opacity` },
} satisfies Record<FilterKind, { readonly title: Message }>;

/** One of CSS's filter functions: a blur, a turn of hue, or an amount of a color quality. */
const filterJson = schema
    .discriminatedUnion("kind", [
        schema.object({
            /** A blur. */
            kind: schema.literal("blur"),
            /** How far it softens. */
            radius: Dimension,
        }),
        schema.object({
            /** A turn of every hue. */
            kind: schema.literal("hue-rotate"),
            /** How far it turns, in degrees. */
            angle: schema.number(),
        }),
        schema.object({
            /** A color quality it scales. */
            kind: FilterAmountKind,
            /** How much, 1 for as it is. */
            amount: schema.number().nonnegative(),
        }),
    ])
    .meta({ id: "style.filter", title: "Filter" });
/** One of CSS's filter functions. */
export type Filter = schema.Infer<typeof filterJson>;

/** One of CSS's filter functions: a blur, a turn of hue, or an amount of a color quality. */
export const Filter = Object.assign(filterJson, {
    /** Start a filter of a kind at the amount that changes the least, a slight blur. */
    of(kind: FilterKind): Filter {
        const amount = FilterAmountKind.safeParse(kind).data;

        return amount !== undefined
            ? { kind: amount, amount: 1 }
            : kind === "blur"
              ? { kind, radius: { value: 4, unit: "px" } }
              : { kind: "hue-rotate", angle: 0 };
    },

    /** Write a filter as its CSS function, such as `blur(4px)`. */
    css(filter: Filter): string {
        return filter.kind === "blur"
            ? `blur(${Dimension.css(filter.radius)})`
            : filter.kind === "hue-rotate"
              ? `hue-rotate(${String(filter.angle)}deg)`
              : `${filter.kind}(${String(filter.amount)})`;
    },
});

/** The filters of a box or of what lies behind it, applied in order, as CSS lists them. */
export const FilterStack = Object.assign(
    schema.array(Filter).meta({ id: "style.filterStack", title: "Filters" }),
    {
        /** Write the filters as CSS's `filter` or `backdrop-filter`, none for no filter. */
        css(stack: readonly Filter[]): string {
            return stack.length === 0
                ? "none"
                : stack.map((filter) => Filter.css(filter)).join(" ");
        },
    },
);
