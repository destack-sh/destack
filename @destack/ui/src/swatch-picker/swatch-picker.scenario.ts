import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { swatchPickerLabelColor } from "./swatch-picker.example.tsx";
import { SwatchPicker } from "./swatch-picker.tsx";

/** Choose swatches with the arrow keys. */
export const swatchPickerChooseWithArrowKeys = defineScenario({
    of: SwatchPicker,
    interaction: viewInteraction,
    name: "choose-with-arrow-keys",
    description:
        "move the focus between swatches with the arrow keys, choosing each, and stop at the ends",
    given: { examples: [swatchPickerLabelColor] },
    when: [
        { action: "focus", target: { role: "option", name: "plum" } },
        { action: "press", key: "ArrowLeft" },
        { action: "press", key: "Home" },
        { action: "press", key: "ArrowLeft" },
    ],
    then: {
        observe: {
            focused: { kind: "focused" },
            chosen: { kind: "name", target: { role: "option", selected: true } },
        },
        each: [
            { focused: "plum", chosen: "plum" },
            { focused: "orange", chosen: "orange" },
            { focused: "teal", chosen: "teal" },
            { focused: "teal", chosen: "teal" },
        ],
    },
});
