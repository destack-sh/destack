import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { createSignal } from "@destack/view";
import { Field, FieldLabel } from "../field/index.ts";
import {
    Slider,
    SliderIndicator,
    SliderMarks,
    SliderThumb,
    SliderTrack,
    SliderValue,
} from "./slider.tsx";

/** The styles of the examples. */
const styles = style.create({
    stack: {
        display: "grid",
        gap: space[6],
        width: "18rem",
        maxWidth: "100%",
    },
    below: {
        position: "absolute",
        insetBlockStart: "100%",
        marginBlockStart: space[2],
    },
    labelled: {
        marginBlockEnd: space[6],
    },
});

/** A slider that sets the font size of a note. */
export const sliderFontSize = defineExample({
    of: Slider,
    name: "fontSize",
    description: t`A slider that sets the font size of a note.`,
    properties: {
        name: "size",
        min: 12,
        max: 24,
        step: 1,
        defaultValue: [16],
        "aria-label": "Font size",
    },
    render: (properties) => <Slider {...properties} />,
});

/** The font size slider unavailable. */
export const sliderFontSizeDisabled = defineExample({
    of: Slider,
    name: "fontSizeDisabled",
    description: t`The font size slider unavailable.`,
    render: () => (
        <Slider name="size" min={12} max={24} defaultValue={[16]} aria-label="Font size" disabled />
    ),
});

/** A price range filter with marks at every hundred. */
export const sliderPriceRange = defineExample({
    of: Slider,
    name: "priceRange",
    description: t`A price range filter with marks at every hundred.`,
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
    name: "volumeVertical",
    description: t`A volume slider standing upright.`,
    render: () => <Slider orientation="vertical" defaultValue={[60]} aria-label="Volume" />,
});

/** A working-hours range composed from the slider's track, range and thumbs. */
export const sliderWorkingHours = defineExample({
    of: Slider,
    name: "workingHours",
    description: t`A working-hours range composed from the slider's track, range and thumbs.`,
    render: () => (
        <Slider name="hours" min={0} max={24} defaultValue={[9, 17]} aria-label="Working hours">
            <SliderTrack>
                <SliderIndicator />
            </SliderTrack>
            <SliderThumb />
            <SliderThumb />
        </Slider>
    ),
});

/** A budget with its value written beside its label, and one a person only reads. */
export const sliderBudget = defineExample({
    of: SliderValue,
    name: "budget",
    description: t`A budget with its values written out, and one a person only reads.`,
    render: () => {
        const [budget, setBudget] = createSignal<readonly number[]>([1200, 3400]);
        const format = { style: "currency", currency: "EUR", maximumFractionDigits: 0 } as const;

        return (
            <div {...style.attrs(styles.stack)}>
                <Slider
                    aria-label="Budget"
                    min={0}
                    max={5000}
                    step={100}
                    minStepsBetweenValues={5}
                    format={format}
                    value={budget()}
                    onValueChange={setBudget}
                    xstyle={styles.labelled}
                >
                    <SliderTrack>
                        <SliderIndicator />
                    </SliderTrack>
                    <SliderThumb />
                    <SliderThumb />
                    <SliderValue xstyle={styles.below} />
                </Slider>
                <Slider
                    aria-label="Spent"
                    max={5000}
                    defaultValue={[2100]}
                    format={format}
                    readOnly
                />
            </div>
        );
    },
});

/** A rating on labelled marks, in halves. */
export const sliderRating = defineExample({
    of: SliderMarks,
    name: "rating",
    description: t`A rating on labelled marks, in halves.`,
    render: () => (
        <div {...style.attrs(styles.stack)}>
            <Field>
                <FieldLabel>Effort</FieldLabel>
                <Slider
                    min={0}
                    max={5}
                    step={0.5}
                    largeStep={2}
                    defaultValue={[2.5]}
                    marks={[0, 1, 2, 3, 4, 5]}
                    xstyle={styles.labelled}
                >
                    <SliderTrack>
                        <SliderIndicator />
                    </SliderTrack>
                    <SliderMarks>{(mark) => String(mark)}</SliderMarks>
                    <SliderThumb />
                </Slider>
            </Field>
        </div>
    ),
});
