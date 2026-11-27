import { schema } from "@destack/schema";
import { type Bindable, Reference } from "../reference/index.ts";
import { Quantity } from "../quantity/index.ts";

/** The units of a dimension: CSS's lengths and percentages, past the token format's px and rem. */
export const DIMENSION_UNITS = ["px", "rem", "em", "%", "vw", "vh", "ch"] as const;

/** A unit a dimension takes. */
export const DimensionUnit = schema.enum(DIMENSION_UNITS);
/** A unit a dimension takes. */
export type DimensionUnit = schema.Infer<typeof DimensionUnit>;

/** A length or percentage, as the design token format's dimension: a number and its unit. */
const dimensionJson = schema
    .object({
        /** The number. */
        value: schema.number(),
        /** The unit. */
        unit: DimensionUnit,
    })
    .meta({ id: "style.dimension", title: "Dimension" });
/** A length or percentage. */
export type Dimension = schema.Infer<typeof dimensionJson>;

/** A length or percentage, as the design token format's dimension: a number and its unit. */
export const Dimension = Object.assign(dimensionJson, {
    /** No length, in pixels. */
    zero: { value: 0, unit: "px" } satisfies Dimension,

    /** Read a length a person types or CSS writes, such as `12px` or `(8 + 4) * 2`, in a unit. */
    read(text: string, unit?: DimensionUnit): Dimension | undefined {
        // read the expression, a plain number in the given unit
        const quantity = Quantity.parse(text);
        const read = DimensionUnit.safeParse(quantity?.unit ?? unit).data;

        return quantity === undefined || read === undefined
            ? undefined
            : { value: quantity.value, unit: read };
    },

    /** A dimension, or a reference to a dimension token in its place. */
    bindable: Reference.or(dimensionJson, "dimension"),

    /** Write a dimension as CSS writes a length, such as `12px`, a reference as its variable. */
    css(dimension: Bindable<Dimension>): string {
        return Reference.write(dimension, (length) => `${String(length.value)}${length.unit}`);
    },
});
