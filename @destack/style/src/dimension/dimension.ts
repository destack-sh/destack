import { schema } from "@destack/schema";

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

    /** Write a dimension as CSS writes a length, such as `12px`. */
    css(dimension: Dimension): string {
        return `${String(dimension.value)}${dimension.unit}`;
    },
});
