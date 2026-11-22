import { defineExample } from "@destack/package/declare";
import { ColorSwatchPicker, ColorSwatchPickerItem } from "./color-swatch-picker.tsx";

/** A project's color chosen from every swatch, teal checked. */
export const colorSwatchPickerProjectColor = defineExample({
    of: ColorSwatchPicker,
    name: "project-color",
    description: "a project's color chosen from every swatch, teal checked",
    properties: {
        defaultValue: "teal",
        "aria-label": "Project color",
    },
    render: (properties) => <ColorSwatchPicker {...properties} />,
});

/** A label's color chosen from three swatches the caller lays out. */
export const colorSwatchPickerLabelColor = defineExample({
    of: ColorSwatchPicker,
    name: "label-color",
    description: "a label's color chosen from three swatches the caller lays out",
    properties: {
        name: "color",
        defaultValue: "plum",
        "aria-label": "Label color",
    },
    render: (properties) => (
        <ColorSwatchPicker {...properties}>
            <ColorSwatchPickerItem value="teal" />
            <ColorSwatchPickerItem value="orange" />
            <ColorSwatchPickerItem value="plum" />
        </ColorSwatchPicker>
    ),
});

/** A project's color before one is chosen. */
export const colorSwatchPickerProjectColorUnset = defineExample({
    of: ColorSwatchPicker,
    name: "project-color-unset",
    description: "a project's color before one is chosen",
    properties: {
        "aria-label": "Project color",
    },
    render: (properties) => <ColorSwatchPicker {...properties} />,
});
