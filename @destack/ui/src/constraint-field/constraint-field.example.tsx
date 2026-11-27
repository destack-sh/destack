import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { ConstraintField } from "./constraint-field.tsx";

/** A layer pinned to its container's top left, as design tools start one. */
export const constraintFieldDefault = defineExample({
    of: ConstraintField,
    name: "default",
    description: t`A layer pinned to its container's top left, as design tools start one.`,
    render: (properties) => <ConstraintField {...properties} aria-label="Constraints" />,
});

/** A layer pinned to both sides, growing with its container, kept at its middle down. */
export const constraintFieldStretched = defineExample({
    of: ConstraintField,
    name: "stretched",
    description: t`A layer pinned to both sides, growing with its container, kept at its middle down.`,
    render: (properties) => (
        <ConstraintField
            {...properties}
            aria-label="Constraints"
            defaultValue={{ x: "both", y: "center" }}
        />
    ),
});

/** Constraints that take no change now. */
export const constraintFieldDisabled = defineExample({
    of: ConstraintField,
    name: "disabled",
    description: t`Constraints that take no change now.`,
    properties: { disabled: true },
    render: (properties) => <ConstraintField {...properties} aria-label="Constraints" />,
});
