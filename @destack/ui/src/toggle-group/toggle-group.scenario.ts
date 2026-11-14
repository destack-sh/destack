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
    parameters: {},
    when: [
        {
            step: { action: "focus", target: { role: "radio", name: "Left" } },
            then: [
                { observe: { kind: "focused" }, is: "Left" },
                { observe: { kind: "name", target: { css: "[tabindex='0']" } }, is: "Left" },
            ],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [
                { observe: { kind: "focused" }, is: "Center" },
                { observe: { kind: "name", target: { css: "[tabindex='0']" } }, is: "Center" },
            ],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [
                { observe: { kind: "focused" }, is: "Justify" },
                { observe: { kind: "name", target: { css: "[tabindex='0']" } }, is: "Justify" },
            ],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [
                { observe: { kind: "focused" }, is: "Left" },
                { observe: { kind: "name", target: { css: "[tabindex='0']" } }, is: "Left" },
            ],
        },
        {
            step: { action: "press", key: "ArrowLeft" },
            then: [
                { observe: { kind: "focused" }, is: "Justify" },
                { observe: { kind: "name", target: { css: "[tabindex='0']" } }, is: "Justify" },
            ],
        },
        {
            step: { action: "press", key: "End" },
            then: [
                { observe: { kind: "focused" }, is: "Justify" },
                { observe: { kind: "name", target: { css: "[tabindex='0']" } }, is: "Justify" },
            ],
        },
        {
            step: { action: "press", key: "Home" },
            then: [
                { observe: { kind: "focused" }, is: "Left" },
                { observe: { kind: "name", target: { css: "[tabindex='0']" } }, is: "Left" },
            ],
        },
    ],
});
