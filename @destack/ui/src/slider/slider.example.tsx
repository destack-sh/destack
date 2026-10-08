import { defineExample } from "@destack/package/declare";
import { Slider, SliderRange, SliderThumb, SliderTrack } from "./slider.tsx";

/** A slider that sets the font size of a note. */
export const sliderFontSize = defineExample({
    of: Slider,
    name: "font-size",
    description: "a slider that sets the font size of a note",
    render: () => (
        <Slider name="size" min={12} max={24} defaultValue={[16]} aria-label="Font size" />
    ),
});

/** The font size slider unavailable. */
export const sliderFontSizeDisabled = defineExample({
    of: Slider,
    name: "font-size-disabled",
    description: "the font size slider unavailable",
    render: () => (
        <Slider name="size" min={12} max={24} defaultValue={[16]} aria-label="Font size" disabled />
    ),
});

/** A price range filter with marks at every hundred. */
export const sliderPriceRange = defineExample({
    of: Slider,
    name: "price-range",
    description: "a price range filter with marks at every hundred",
    render: () => (
        <Slider
            name="price"
            min={0}
            max={500}
            step={10}
            defaultValue={[100, 300]}
            marks={[0, 100, 200, 300, 400, 500]}
            aria-label="Price"
        />
    ),
});

/** A volume slider standing upright. */
export const sliderVolumeVertical = defineExample({
    of: Slider,
    name: "volume-vertical",
    description: "a volume slider standing upright",
    render: () => <Slider orientation="vertical" defaultValue={[60]} aria-label="Volume" />,
});

/** A working-hours range composed from the slider's track, range and thumbs. */
export const sliderWorkingHours = defineExample({
    of: Slider,
    name: "working-hours",
    description: "a working-hours range composed from the slider's track, range and thumbs",
    render: () => (
        <Slider name="hours" min={0} max={24} defaultValue={[9, 17]} aria-label="Working hours">
            <SliderTrack>
                <SliderRange />
            </SliderTrack>
            <SliderThumb />
            <SliderThumb />
        </Slider>
    ),
});
