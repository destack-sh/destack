import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { DimensionField } from "./dimension-field.tsx";

/** A sidebar's width, in the root's font size. */
export const dimensionFieldDefault = defineExample({
    of: DimensionField,
    name: "default",
    description: t`A sidebar's width, its number and its unit.`,
    properties: { defaultValue: { value: 18, unit: "rem" }, "aria-label": "Width" },
    render: (properties) => <DimensionField {...properties} />,
});
