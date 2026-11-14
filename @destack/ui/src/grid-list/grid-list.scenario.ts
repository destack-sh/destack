import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { gridListSymbols } from "./grid-list.example.tsx";
import { GridList } from "./grid-list.tsx";

/** Move through a grid list's rows and columns and pick with Enter. */
export const gridListMoveAndPick = defineScenario({
    of: GridList,
    interaction: viewInteraction,
    name: "move-and-pick",
    description:
        "move the focused cell along a row, down a column across a section's heading and to the row's end, and pick it with Enter",
    given: { examples: [gridListSymbols] },
    parameters: {},
    when: [
        {
            step: { action: "focus", target: { role: "grid", name: "Symbols" } },
            then: [
                {
                    observe: { kind: "name", target: { role: "gridcell", selected: true } },
                    is: "plus-minus",
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "" },
            ],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [
                {
                    observe: { kind: "name", target: { role: "gridcell", selected: true } },
                    is: "multiplication",
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "" },
            ],
        },
        {
            step: { action: "press", key: "ArrowDown" },
            then: [
                {
                    observe: { kind: "name", target: { role: "gridcell", selected: true } },
                    is: "almost equal",
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "" },
            ],
        },
        {
            step: { action: "press", key: "ArrowDown" },
            then: [
                {
                    observe: { kind: "name", target: { role: "gridcell", selected: true } },
                    is: "summation",
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "" },
            ],
        },
        {
            step: { action: "press", key: "End" },
            then: [
                {
                    observe: { kind: "name", target: { role: "gridcell", selected: true } },
                    is: "square root",
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "" },
            ],
        },
        {
            step: { action: "press", key: "Enter" },
            then: [
                {
                    observe: { kind: "name", target: { role: "gridcell", selected: true } },
                    is: "square root",
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "square root" },
            ],
        },
    ],
});
