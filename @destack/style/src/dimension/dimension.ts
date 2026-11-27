import { schema } from "@destack/schema";

/** The units a dimension takes, as CSS writes lengths and percentages. */
export const DIMENSION_UNITS = ["px", "rem", "em", "%", "vw", "vh", "ch"] as const;

/** A unit a dimension takes. */
export type DimensionUnit = (typeof DIMENSION_UNITS)[number];

/** A CSS length or percentage: a number and its unit, or a bare zero. */
const DIMENSION = new RegExp(`^(?:0|-?(?:\\d+|\\d*\\.\\d+)(?:${DIMENSION_UNITS.join("|")}))$`, "u");

/** A length or percentage as CSS writes it, such as `12px`, `1.5rem` or `50%`. */
export const Dimension = Object.assign(
    schema
        .string()
        .regex(DIMENSION, { message: "a dimension is a number and a unit, such as 12px" })
        .meta({ id: "dimension", title: "Dimension" }),
    {
        /** Read a dimension's number and unit, a bare zero in pixels, refusing other text. */
        split(text: string): { readonly value: number; readonly unit: DimensionUnit } {
            // refuse text that is no dimension, then take the unit it ends in
            const valid = Dimension.parse(text);
            const unit = DIMENSION_UNITS.find((each) => valid.endsWith(each)) ?? "px";

            return { value: Number.parseFloat(valid), unit };
        },

        /** Write a number in a unit as a dimension. */
        of(value: number, unit: DimensionUnit): string {
            return `${String(value)}${unit}`;
        },
    },
);
