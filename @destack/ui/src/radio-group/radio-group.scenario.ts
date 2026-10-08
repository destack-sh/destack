import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { radioGroupListDensity } from "./radio-group.example.tsx";
import { RadioGroup } from "./radio-group.tsx";

/** Check radios with the arrow keys. */
export const radioGroupCheckWithArrowKeys = defineScenario({
    of: RadioGroup,
    interaction: viewInteraction,
    name: "check-with-arrow-keys",
    description:
        "move the focus to the next or previous radio with the arrow keys, checking it and wrapping at the ends",
    given: { examples: [radioGroupListDensity] },
    when: [
        { action: "focus", target: { role: "radio", name: "Regular" } },
        { action: "press", key: "ArrowDown" },
        { action: "press", key: "ArrowUp" },
        { action: "press", key: "ArrowRight" },
    ],
    then: {
        observe: {
            focused: { kind: "focused" },
            checked: { kind: "name", target: { role: "radio", checked: true } },
        },
        each: [
            { focused: "Regular", checked: "Regular" },
            { focused: "Compact", checked: "Compact" },
            { focused: "Regular", checked: "Regular" },
            { focused: "Compact", checked: "Compact" },
        ],
    },
});
