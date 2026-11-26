import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { ColorSwatchPicker, ColorSwatchPickerItem } from "./color-swatch-picker.tsx";

/** A project's color chosen from every swatch, cyan checked. */
export const colorSwatchPickerProjectColor = defineExample({
    of: ColorSwatchPicker,
    name: "projectColor",
    description: t`A project's color chosen from every swatch, cyan checked.`,
    properties: {
        defaultValue: "cyan",
        "aria-label": "Project color",
    },
    render: (properties) => <ColorSwatchPicker {...properties} />,
});

/** A label's color chosen from three swatches the caller lays out. */
export const colorSwatchPickerLabelColor = defineExample({
    of: ColorSwatchPicker,
    name: "labelColor",
    description: t`A label's color chosen from three swatches the caller lays out.`,
    properties: {
        name: "color",
        defaultValue: "purple",
        "aria-label": "Label color",
    },
    render: (properties) => (
        <ColorSwatchPicker {...properties}>
            <ColorSwatchPickerItem value="cyan" />
            <ColorSwatchPickerItem value="orange" />
            <ColorSwatchPickerItem value="purple" />
        </ColorSwatchPicker>
    ),
});

/** A project's color before one is chosen. */
export const colorSwatchPickerProjectColorUnset = defineExample({
    of: ColorSwatchPicker,
    name: "projectColorUnset",
    description: t`A project's color before one is chosen.`,
    properties: {
        "aria-label": "Project color",
    },
    render: (properties) => <ColorSwatchPicker {...properties} />,
});
