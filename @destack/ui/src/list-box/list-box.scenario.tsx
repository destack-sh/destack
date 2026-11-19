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
    parameters: {},
    when: [
        {
            step: { action: "focus", target: { role: "option", name: "Work" } },
            then: [
                { observe: { kind: "focused" }, is: "Work" },
                {
                    observe: { kind: "name", target: { role: "option", selected: true } },
                    is: "Work",
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "" },
            ],
        },
        {
            step: { action: "press", key: "ArrowDown" },
            then: [
                { observe: { kind: "focused" }, is: "Recipes" },
                {
                    observe: { kind: "name", target: { role: "option", selected: true } },
                    is: "Work",
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "" },
            ],
        },
        {
            step: { action: "press", key: "ArrowDown" },
            then: [
                { observe: { kind: "focused" }, is: "Recipes" },
                {
                    observe: { kind: "name", target: { role: "option", selected: true } },
                    is: "Work",
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "" },
            ],
        },
        {
            step: { action: "press", key: " " },
            then: [
                { observe: { kind: "focused" }, is: "Recipes" },
                {
                    observe: { kind: "name", target: { role: "option", selected: true } },
                    is: "Recipes",
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "recipes" },
            ],
        },
        {
            step: { action: "press", key: "Home" },
            then: [
                { observe: { kind: "focused" }, is: "Trips" },
                {
                    observe: { kind: "name", target: { role: "option", selected: true } },
                    is: "Recipes",
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "recipes" },
            ],
        },
        {
            step: { action: "press", key: "r" },
            then: [
                { observe: { kind: "focused" }, is: "Recipes" },
                {
                    observe: { kind: "name", target: { role: "option", selected: true } },
                    is: "Recipes",
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "recipes" },
            ],
        },
    ],
});

/** Move through a list box's grid by rows and columns, the focus selecting. */
export const listBoxMoveThroughGrid = defineScenario({
    of: ListBox,
    interaction: viewInteraction,
    name: "move-through-grid",
    description:
        "move the focus along a row and down a column of a grid layout, to the row's end and the grid's start, each option it lands on selected",
    given: { examples: [listBoxLabelGrid] },
    parameters: {},
    when: [
        {
            step: { action: "focus", target: { role: "option", name: "Urgent" } },
            then: [
                { observe: { kind: "focused" }, is: "Urgent" },
                {
                    observe: { kind: "name", target: { role: "option", selected: true } },
                    is: "Urgent",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [
                { observe: { kind: "focused" }, is: "Later" },
                {
                    observe: { kind: "name", target: { role: "option", selected: true } },
                    is: "Later",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowDown" },
            then: [
                { observe: { kind: "focused" }, is: "Done" },
                {
                    observe: { kind: "name", target: { role: "option", selected: true } },
                    is: "Done",
                },
            ],
        },
        {
            step: { action: "press", key: "End" },
            then: [
                { observe: { kind: "focused" }, is: "Someday" },
                {
                    observe: { kind: "name", target: { role: "option", selected: true } },
                    is: "Someday",
                },
            ],
        },
        {
            step: { action: "press", key: "Control+Home" },
            then: [
                { observe: { kind: "focused" }, is: "Urgent" },
                {
                    observe: { kind: "name", target: { role: "option", selected: true } },
                    is: "Urgent",
                },
            ],
        },
    ],
});
