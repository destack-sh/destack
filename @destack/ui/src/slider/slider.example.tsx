import { defineExample } from "@destack/package/declare";
import { Slider } from "./slider.tsx";

/** A slider that sets the font size of a note. */
export const sliderFontSize = defineExample({
    of: Slider,
    name: "font-size",
    description: "a slider that sets the font size of a note",
    render: () => (
        <Slider name="size" min={12} max={24} step={1} value={16} aria-label="Font size" />
    ),
});

/** The font size slider unavailable. */
export const sliderFontSizeDisabled = defineExample({
    of: Slider,
    name: "font-size-disabled",
    description: "the font size slider unavailable",
    render: () => (
        <Slider name="size" min={12} max={24} step={1} value={16} aria-label="Font size" disabled />
    ),
});
