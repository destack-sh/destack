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
        "focus a cell and move along its row, down a column across a section's heading and to the row's end, and pick it with Enter",
    given: { examples: [gridListSymbols] },
    parameters: {},
    when: [
        {
            step: { action: "focus", target: { role: "gridcell", name: "plus-minus" } },
            then: [
                {
                    observe: { kind: "focused" },
                    is: "plus-minus",
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "" },
            ],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [
                {
                    observe: { kind: "focused" },
                    is: "multiplication",
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "" },
            ],
        },
        {
            step: { action: "press", key: "ArrowDown" },
            then: [
                {
                    observe: { kind: "focused" },
                    is: "almost equal",
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "" },
            ],
        },
        {
            step: { action: "press", key: "ArrowDown" },
            then: [
                {
                    observe: { kind: "focused" },
                    is: "summation",
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "" },
            ],
        },
        {
            step: { action: "press", key: "End" },
            then: [
                {
                    observe: { kind: "focused" },
                    is: "square root",
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "" },
            ],
        },
        {
            step: { action: "press", key: "Enter" },
            then: [
                {
                    observe: { kind: "focused" },
                    is: "square root",
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "square root" },
            ],
        },
    ],
});
