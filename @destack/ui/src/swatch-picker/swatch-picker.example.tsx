import { defineExample } from "@destack/package/declare";
import { SwatchPicker, SwatchPickerItem } from "./swatch-picker.tsx";

/** A project's color chosen from every swatch, teal checked. */
export const swatchPickerProjectColor = defineExample({
    of: SwatchPicker,
    name: "project-color",
    description: "a project's color chosen from every swatch, teal checked",
    render: () => <SwatchPicker defaultValue="teal" aria-label="Project color" />,
});

/** A label's color chosen from three swatches the caller lays out. */
export const swatchPickerLabelColor = defineExample({
    of: SwatchPicker,
    name: "label-color",
    description: "a label's color chosen from three swatches the caller lays out",
    render: () => (
        <SwatchPicker name="color" defaultValue="plum" aria-label="Label color">
            <SwatchPickerItem value="teal" />
            <SwatchPickerItem value="orange" />
            <SwatchPickerItem value="plum" />
        </SwatchPicker>
    ),
});

/** A project's color before one is chosen. */
export const swatchPickerProjectColorUnset = defineExample({
    of: SwatchPicker,
    name: "project-color-unset",
    description: "a project's color before one is chosen",
    render: () => <SwatchPicker aria-label="Project color" />,
});
