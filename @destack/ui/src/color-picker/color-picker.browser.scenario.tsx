import { t } from "@destack/locale";
import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { colorPickerAccent } from "./color-picker.example.tsx";
import { ColorPicker } from "./color-picker.tsx";

/** The text of the color. */
const field = { css: "[data-slot=color-field]" };

/** Press the middle of the area, step by the keys, and type a color. */
export const colorPickerPick = defineScenario({
    of: ColorPicker,
    interaction: viewInteraction,
    name: "pick",
    description: t`Press the middle of the area, step by the keys, and type a color.`,
    given: { examples: [colorPickerAccent] },
    parameters: {},
    when: [
        {
            step: { action: "click", target: { css: "[data-slot=color-area-paint]" } },
            then: [{ observe: { kind: "value", target: field }, is: "#7f5840" }],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [{ observe: { kind: "value", target: field }, is: "#7f573e" }],
        },
        { step: { action: "fill", target: field, value: "30a46c" } },
        {
            step: { action: "press", key: "Enter" },
            then: [{ observe: { kind: "value", target: field }, is: "#30a46c" }],
        },
    ],
});
