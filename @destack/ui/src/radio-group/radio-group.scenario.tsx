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
    parameters: {},
    when: [
        {
            step: { action: "focus", target: { role: "radio", name: "Regular" } },
            then: [
                { observe: { kind: "focused" }, is: "Regular" },
                {
                    observe: { kind: "name", target: { role: "radio", checked: true } },
                    is: "Regular",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowDown" },
            then: [
                { observe: { kind: "focused" }, is: "Compact" },
                {
                    observe: { kind: "name", target: { role: "radio", checked: true } },
                    is: "Compact",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowUp" },
            then: [
                { observe: { kind: "focused" }, is: "Regular" },
                {
                    observe: { kind: "name", target: { role: "radio", checked: true } },
                    is: "Regular",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [
                { observe: { kind: "focused" }, is: "Compact" },
                {
                    observe: { kind: "name", target: { role: "radio", checked: true } },
                    is: "Compact",
                },
            ],
        },
    ],
});
