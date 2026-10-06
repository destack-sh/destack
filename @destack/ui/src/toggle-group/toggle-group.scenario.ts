import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { toggleGroupUnaligned } from "./toggle-group.example.tsx";
import { ToggleGroup } from "./toggle-group.tsx";

/** Move through a toggle group with the arrow keys. */
export const toggleGroupMoveWithArrowKeys = defineScenario({
    of: ToggleGroup,
    interaction: viewInteraction,
    name: "move-with-arrow-keys",
    description:
        "move the focus between a toggle group's items with arrow keys, Home and End, wrapping",
    given: { examples: [toggleGroupUnaligned] },
    when: [
        { action: "focus", target: { role: "radio", name: "Left" } },
        { action: "press", key: "ArrowRight" },
        { action: "press", key: "ArrowRight" },
        { action: "press", key: "ArrowRight" },
        { action: "press", key: "ArrowLeft" },
        { action: "press", key: "End" },
        { action: "press", key: "Home" },
    ],
    then: {
        observe: {
            focused: { kind: "focused" },
            tabStop: { kind: "name", target: { css: "[tabindex='0']" } },
        },
        each: [
            { focused: "Left", tabStop: "Left" },
            { focused: "Center", tabStop: "Center" },
            { focused: "Justify", tabStop: "Justify" },
            { focused: "Left", tabStop: "Left" },
            { focused: "Justify", tabStop: "Justify" },
            { focused: "Justify", tabStop: "Justify" },
            { focused: "Left", tabStop: "Left" },
        ],
    },
});
