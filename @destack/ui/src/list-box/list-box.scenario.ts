import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { listBoxLabelGrid, listBoxNotebook } from "./list-box.example.tsx";
import { ListBox } from "./list-box.tsx";

/** Move through a list box with the keys and choose with Space. */
export const listBoxMoveAndChoose = defineScenario({
    of: ListBox,
    interaction: viewInteraction,
    name: "move-and-choose",
    description:
        "move the focus with the arrow keys past an unavailable option, stopping at the end, choose with Space, and jump with Home and a typed letter",
    given: { examples: [listBoxNotebook] },
    when: [
        { action: "focus", target: { role: "option", name: "Work" } },
        { action: "press", key: "ArrowDown" },
        { action: "press", key: "ArrowDown" },
        { action: "press", key: " " },
        { action: "press", key: "Home" },
        { action: "press", key: "r" },
    ],
    then: {
        observe: {
            focused: { kind: "focused" },
            selected: { kind: "name", target: { role: "option", selected: true } },
            chosen: { kind: "text", target: { role: "status" } },
        },
        each: [
            { focused: "Work", selected: "Work", chosen: "" },
            { focused: "Recipes", selected: "Work", chosen: "" },
            { focused: "Recipes", selected: "Work", chosen: "" },
            { focused: "Recipes", selected: "Recipes", chosen: "recipes" },
            { focused: "Trips", selected: "Recipes", chosen: "recipes" },
            { focused: "Recipes", selected: "Recipes", chosen: "recipes" },
        ],
    },
});

/** Move through a list box's grid by rows and columns, the focus selecting. */
export const listBoxMoveThroughGrid = defineScenario({
    of: ListBox,
    interaction: viewInteraction,
    name: "move-through-grid",
    description:
        "move the focus along a row and down a column of a grid layout, to the row's end and the grid's start, each option it lands on selected",
    given: { examples: [listBoxLabelGrid] },
    when: [
        { action: "focus", target: { role: "option", name: "Urgent" } },
        { action: "press", key: "ArrowRight" },
        { action: "press", key: "ArrowDown" },
        { action: "press", key: "End" },
        { action: "press", key: "Control+Home" },
    ],
    then: {
        observe: {
            focused: { kind: "focused" },
            selected: { kind: "name", target: { role: "option", selected: true } },
        },
        each: [
            { focused: "Urgent", selected: "Urgent" },
            { focused: "Later", selected: "Later" },
            { focused: "Done", selected: "Done" },
            { focused: "Someday", selected: "Someday" },
            { focused: "Urgent", selected: "Urgent" },
        ],
    },
});
