import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { sortableBoard, sortableChecklist } from "./sortable.example.tsx";
import { Sortable } from "./sortable.tsx";

/** Reorder an item with the keyboard. */
export const sortableReorderWithKeys = defineScenario({
    of: Sortable,
    interaction: viewInteraction,
    name: "reorder-with-keys",
    description:
        "lift an item from its handle with Space, move it down with the arrow keys and drop it with Space, announcing each step",
    given: { examples: [sortableChecklist] },
    when: [
        { action: "focus", target: { role: "button", name: "Move", nth: 0 } },
        { action: "press", key: " " },
        { action: "press", key: "ArrowDown" },
        { action: "press", key: "ArrowDown" },
        { action: "press", key: " " },
    ],
    then: {
        observe: {
            steps: { kind: "texts", target: { css: "[data-slot=sortable-item]" } },
            announced: { kind: "text", target: { role: "status" } },
        },
        each: [
            { steps: ["Pack", "Check in", "Board"], announced: "" },
            {
                steps: ["Pack", "Check in", "Board"],
                announced: "Picked up \u2068Pack\u2069, position 1 of 3 in \u2068Steps\u2069.",
            },
            {
                steps: ["Pack", "Check in", "Board"],
                announced: "\u2068Pack\u2069 moved to position 2 of 3 in \u2068Steps\u2069.",
            },
            {
                steps: ["Pack", "Check in", "Board"],
                announced: "\u2068Pack\u2069 moved to position 3 of 3 in \u2068Steps\u2069.",
            },
            {
                steps: ["Check in", "Board", "Pack"],
                announced: "\u2068Pack\u2069 dropped at position 3 of 3 in \u2068Steps\u2069.",
            },
        ],
    },
});

/** Put a lifted item back with Escape. */
export const sortableCancelWithEscape = defineScenario({
    of: Sortable,
    interaction: viewInteraction,
    name: "cancel-with-escape",
    description: "lift an item with Enter, move it up and put it back where it was with Escape",
    given: { examples: [sortableChecklist] },
    when: [
        { action: "focus", target: { role: "button", name: "Move", nth: 1 } },
        { action: "press", key: "Enter" },
        { action: "press", key: "ArrowUp" },
        { action: "press", key: "Escape" },
    ],
    then: {
        observe: {
            steps: { kind: "texts", target: { css: "[data-slot=sortable-item]" } },
            lifted: {
                kind: "count",
                target: { css: "[data-slot=sortable-item][data-state=dragging]" },
            },
            announced: { kind: "text", target: { role: "status" } },
        },
        each: [
            { steps: ["Pack", "Check in", "Board"], lifted: 0, announced: "" },
            {
                steps: ["Pack", "Check in", "Board"],
                lifted: 1,
                announced: "Picked up \u2068Check in\u2069, position 2 of 3 in \u2068Steps\u2069.",
            },
            {
                steps: ["Pack", "Check in", "Board"],
                lifted: 1,
                announced: "\u2068Check in\u2069 moved to position 1 of 3 in \u2068Steps\u2069.",
            },
            {
                steps: ["Pack", "Check in", "Board"],
                lifted: 0,
                announced:
                    "Moving \u2068Check in\u2069 was cancelled. It returned to position 2 of 3 in \u2068Steps\u2069.",
            },
        ],
    },
});

/** Move a card to the next column with the keyboard. */
export const sortableMoveAcrossColumns = defineScenario({
    of: Sortable,
    interaction: viewInteraction,
    name: "move-across-columns",
    description: "move a lifted card into the column beside its own with the cross arrow key",
    given: { examples: [sortableBoard] },
    when: [
        { action: "focus", target: { role: "button", name: "Move", nth: 1 } },
        { action: "press", key: " " },
        { action: "press", key: "ArrowRight" },
    ],
    then: {
        observe: { announced: { kind: "text", target: { role: "status" } } },
        each: [
            { announced: "" },
            {
                announced:
                    "Picked up \u2068Review the post\u2069, position 2 of 2 in \u2068To do\u2069.",
            },
            {
                announced:
                    "\u2068Review the post\u2069 moved to position 2 of 2 in \u2068Done\u2069.",
            },
        ],
    },
});
