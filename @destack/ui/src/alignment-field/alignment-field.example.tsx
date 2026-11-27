import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { AlignmentField } from "./alignment-field.tsx";

/** A grid that places a box's children at its top start. */
export const alignmentFieldDefault = defineExample({
    of: AlignmentField,
    name: "default",
    description: t`A grid that places a box's children at its top start.`,
    render: (properties) => (
        <AlignmentField
            {...properties}
            aria-label="Alignment"
            defaultValue={{ x: "start", y: "start" }}
        />
    ),
});

/** A grid with none of its places chosen, as for children spaced apart. */
export const alignmentFieldNone = defineExample({
    of: AlignmentField,
    name: "none",
    description: t`A grid with none of its places chosen, as for children spaced apart.`,
    render: (properties) => <AlignmentField {...properties} aria-label="Alignment" />,
});

/** A grid that takes no choice now, its center chosen. */
export const alignmentFieldDisabled = defineExample({
    of: AlignmentField,
    name: "disabled",
    description: t`A grid that takes no choice now, its center chosen.`,
    properties: { disabled: true },
    render: (properties) => (
        <AlignmentField
            {...properties}
            aria-label="Alignment"
            defaultValue={{ x: "center", y: "center" }}
        />
    ),
});
