import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { studioTasks } from "./studio.example.tsx";
import { Tasks } from "./tasks.tsx";

/** The selected tasks. */
const selected = { css: "[role=gridcell][aria-selected=true]" };

/** The names of the groups. */
const groups = { css: "[data-slot=grid-list-section-header] [role=columnheader]" };

/** The number of tasks in the third group, To do. */
const todo = { css: "[data-slot=grid-list-section-header]", nth: 2 };

/** The view the person looks at. */
const view = { css: "main [aria-pressed=true]", nth: 0 };

/** The actions on a task, as its menu lists them. */
const MENU = [
    "Open↵",
    "Change statusS",
    "Set priorityP",
    "Add to project⇧P",
    "Set due date⇧D",
    "Assign toA",
    "Change labelsL",
    "PeekSpace",
    "Assign to meI",
    "Move",
    "Move up⇧⌥↑",
    "Move down⇧⌥↓",
    "Delete⌘⌫",
];

/** The search of the open picker. */
const picker = { css: "dialog[open] [data-slot=command-input]" };

/** Create a task, set it by keys, look at it, change several at once, and take it back. */
export const studioWorkByKeys = defineScenario({
    of: Tasks,
    interaction: viewInteraction,
    name: "work-by-keys",
    description:
        "create a task, set its status by a digit, peek, select two and assign them, and undo",
    given: { examples: [studioTasks] },
    parameters: {},
    when: [
        // create a task
        {
            step: { action: "press", key: "c" },
            then: [
                {
                    observe: { kind: "texts", target: groups },
                    is: ["In review", "In progress", "To do"],
                },
            ],
        },
        {
            step: {
                action: "fill",
                target: { css: "dialog[open] input[aria-label=Title]" },
                value: "Order the paper",
            },
        },
        {
            step: { action: "press", key: "Enter" },
            then: [{ observe: { kind: "text", target: todo }, is: "To do9New taskC" }],
        },

        // set the status of the task under the pointer by a digit
        { step: { action: "pointerMove", target: { css: "[data-item=HAR-6]" } } },
        {
            step: { action: "press", key: "s" },
            then: [{ observe: { kind: "focused" }, is: "Change status" }],
        },
        {
            step: { action: "press", key: "3", target: picker },
            then: [{ observe: { kind: "text", target: todo }, is: "To do10New taskC" }],
        },

        // look at a task, and at the next
        { step: { action: "pointerMove", target: { css: "[data-item=HAR-1]" } } },
        {
            step: { action: "press", key: "Space" },
            then: [{ observe: { kind: "count", target: { css: "[data-slot=card]" } }, is: 1 }],
        },
        {
            step: { action: "press", key: "Space" },
            then: [{ observe: { kind: "count", target: { css: "[data-slot=card]" } }, is: 0 }],
        },

        // select two, and assign both
        { step: { action: "press", key: "x" } },
        {
            step: { action: "press", key: "Shift+ArrowDown" },
            then: [
                { observe: { kind: "count", target: selected }, is: 2 },
                {
                    observe: { kind: "text", target: { css: "[data-slot=action-bar-count]" } },
                    is: "⁨2⁩ selected",
                },
            ],
        },
        { step: { action: "press", key: "s" } },
        {
            step: { action: "press", key: "3", target: picker },
            then: [{ observe: { kind: "text", target: todo }, is: "To do12New taskC" }],
        },

        // take it back
        {
            step: { action: "press", key: "ControlOrMeta+z" },
            then: [{ observe: { kind: "text", target: todo }, is: "To do10New taskC" }],
        },
        {
            step: { action: "press", key: "Escape" },
            then: [{ observe: { kind: "count", target: selected }, is: 0 }],
        },

        // go to another view and back by two keys in a row
        { step: { action: "press", key: "g" } },
        {
            step: { action: "press", key: "b" },
            then: [{ observe: { kind: "text", target: view }, is: "Backlog" }],
        },
        { step: { action: "press", key: "g" } },
        {
            step: { action: "press", key: "a" },
            then: [{ observe: { kind: "text", target: view }, is: "Active" }],
        },

        // open the menu of a task by keys
        { step: { action: "pointerMove", target: { css: "[data-item=HAR-1]" } } },
        {
            step: { action: "press", key: "Shift+F10" },
            then: [
                {
                    observe: { kind: "texts", target: { css: ":popover-open [role=menuitem]" } },
                    is: MENU,
                },
            ],
        },
    ],
});
