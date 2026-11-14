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
    parameters: {},
    when: [
        {
            step: { action: "focus", target: { role: "button", name: "Move", nth: 0 } },
            then: [
                {
                    observe: { kind: "texts", target: { css: "[data-slot=sortable-item]" } },
                    is: ["Pack", "Check in", "Board"],
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "" },
            ],
        },
        {
            step: { action: "press", key: " " },
            then: [
                {
                    observe: { kind: "texts", target: { css: "[data-slot=sortable-item]" } },
                    is: ["Pack", "Check in", "Board"],
                },
                {
                    observe: { kind: "text", target: { role: "status" } },
                    is: "Picked up \u2068Pack\u2069, position 1 of 3 in \u2068Steps\u2069.",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowDown" },
            then: [
                {
                    observe: { kind: "texts", target: { css: "[data-slot=sortable-item]" } },
                    is: ["Pack", "Check in", "Board"],
                },
                {
                    observe: { kind: "text", target: { role: "status" } },
                    is: "\u2068Pack\u2069 moved to position 2 of 3 in \u2068Steps\u2069.",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowDown" },
            then: [
                {
                    observe: { kind: "texts", target: { css: "[data-slot=sortable-item]" } },
                    is: ["Pack", "Check in", "Board"],
                },
                {
                    observe: { kind: "text", target: { role: "status" } },
                    is: "\u2068Pack\u2069 moved to position 3 of 3 in \u2068Steps\u2069.",
                },
            ],
        },
        {
            step: { action: "press", key: " " },
            then: [
                {
                    observe: { kind: "texts", target: { css: "[data-slot=sortable-item]" } },
                    is: ["Check in", "Board", "Pack"],
                },
                {
                    observe: { kind: "text", target: { role: "status" } },
                    is: "\u2068Pack\u2069 dropped at position 3 of 3 in \u2068Steps\u2069.",
                },
            ],
        },
    ],
});

/** Put a lifted item back with Escape. */
export const sortableCancelWithEscape = defineScenario({
    of: Sortable,
    interaction: viewInteraction,
    name: "cancel-with-escape",
    description: "lift an item with Enter, move it up and put it back where it was with Escape",
    given: { examples: [sortableChecklist] },
    parameters: {},
    when: [
        {
            step: { action: "focus", target: { role: "button", name: "Move", nth: 1 } },
            then: [
                {
                    observe: { kind: "texts", target: { css: "[data-slot=sortable-item]" } },
                    is: ["Pack", "Check in", "Board"],
                },
                {
                    observe: {
                        kind: "count",
                        target: { css: "[data-slot=sortable-item][data-state=dragging]" },
                    },
                    is: 0,
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "" },
            ],
        },
        {
            step: { action: "press", key: "Enter" },
            then: [
                {
                    observe: { kind: "texts", target: { css: "[data-slot=sortable-item]" } },
                    is: ["Pack", "Check in", "Board"],
                },
                {
                    observe: {
                        kind: "count",
                        target: { css: "[data-slot=sortable-item][data-state=dragging]" },
                    },
                    is: 1,
                },
                {
                    observe: { kind: "text", target: { role: "status" } },
                    is: "Picked up \u2068Check in\u2069, position 2 of 3 in \u2068Steps\u2069.",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowUp" },
            then: [
                {
                    observe: { kind: "texts", target: { css: "[data-slot=sortable-item]" } },
                    is: ["Pack", "Check in", "Board"],
                },
                {
                    observe: {
                        kind: "count",
                        target: { css: "[data-slot=sortable-item][data-state=dragging]" },
                    },
                    is: 1,
                },
                {
                    observe: { kind: "text", target: { role: "status" } },
                    is: "\u2068Check in\u2069 moved to position 1 of 3 in \u2068Steps\u2069.",
                },
            ],
        },
        {
            step: { action: "press", key: "Escape" },
            then: [
                {
                    observe: { kind: "texts", target: { css: "[data-slot=sortable-item]" } },
                    is: ["Pack", "Check in", "Board"],
                },
                {
                    observe: {
                        kind: "count",
                        target: { css: "[data-slot=sortable-item][data-state=dragging]" },
                    },
                    is: 0,
                },
                {
                    observe: { kind: "text", target: { role: "status" } },
                    is: "Moving \u2068Check in\u2069 was cancelled. It returned to position 2 of 3 in \u2068Steps\u2069.",
                },
            ],
        },
    ],
});

/** Move a card to the next column with the keyboard. */
export const sortableMoveAcrossColumns = defineScenario({
    of: Sortable,
    interaction: viewInteraction,
    name: "move-across-columns",
    description: "move a lifted card into the column beside its own with the cross arrow key",
    given: { examples: [sortableBoard] },
    parameters: {},
    when: [
        {
            step: { action: "focus", target: { role: "button", name: "Move", nth: 1 } },
            then: [{ observe: { kind: "text", target: { role: "status" } }, is: "" }],
        },
        {
            step: { action: "press", key: " " },
            then: [
                {
                    observe: { kind: "text", target: { role: "status" } },
                    is: "Picked up \u2068Review the post\u2069, position 2 of 2 in \u2068To do\u2069.",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [
                {
                    observe: { kind: "text", target: { role: "status" } },
                    is: "\u2068Review the post\u2069 moved to position 2 of 2 in \u2068Done\u2069.",
                },
            ],
        },
    ],
});
