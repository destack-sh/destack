import { t } from "@destack/locale";
import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { colorSwatchPickerLabelColor } from "./color-swatch-picker.example.tsx";
import { ColorSwatchPicker } from "./color-swatch-picker.tsx";

/** Choose swatches with the arrow keys. */
export const colorSwatchPickerChooseWithArrowKeys = defineScenario({
    of: ColorSwatchPicker,
    interaction: viewInteraction,
    name: "chooseWithArrowKeys",
    description: t`Move the focus between swatches with the arrow keys, choosing each, and stop at the ends.`,
    given: { examples: [colorSwatchPickerLabelColor] },
    parameters: {},
    when: [
        {
            step: { action: "focus", target: { role: "option", name: "purple" } },
            then: [
                { observe: { kind: "focused" }, is: "purple" },
                {
                    observe: { kind: "name", target: { role: "option", selected: true } },
                    is: "purple",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowLeft" },
            then: [
                { observe: { kind: "focused" }, is: "orange" },
                {
                    observe: { kind: "name", target: { role: "option", selected: true } },
                    is: "orange",
                },
            ],
        },
        {
            step: { action: "press", key: "Home" },
            then: [
                { observe: { kind: "focused" }, is: "cyan" },
                {
                    observe: { kind: "name", target: { role: "option", selected: true } },
                    is: "cyan",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowLeft" },
            then: [
                { observe: { kind: "focused" }, is: "cyan" },
                {
                    observe: { kind: "name", target: { role: "option", selected: true } },
                    is: "cyan",
                },
            ],
        },
    ],
});
