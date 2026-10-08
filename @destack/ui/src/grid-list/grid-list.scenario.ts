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
    when: [
        { action: "focus", target: { role: "grid", name: "Symbols" } },
        { action: "press", key: "ArrowRight" },
        { action: "press", key: "ArrowDown" },
        { action: "press", key: "ArrowDown" },
        { action: "press", key: "End" },
        { action: "press", key: "Enter" },
    ],
    then: {
        observe: {
            focused: { kind: "name", target: { role: "gridcell", selected: true } },
            picked: { kind: "text", target: { role: "status" } },
        },
        each: [
            { focused: "plus-minus", picked: "" },
            { focused: "multiplication", picked: "" },
            { focused: "almost equal", picked: "" },
            { focused: "summation", picked: "" },
            { focused: "square root", picked: "" },
            { focused: "square root", picked: "square root" },
        ],
    },
});
