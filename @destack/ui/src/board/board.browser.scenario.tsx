import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { boardLaunch, boardLongBacklog } from "./board.example.tsx";
import { Board } from "./board.tsx";

/** The card the scenarios drag. */
const card = { text: "Write the launch post" };

/** The second column, Doing. */
const doing = { css: "[data-slot=board-column]", nth: 1 };

/** The cards of the second column. */
const doingCards = { css: "[data-slot=board-item]", within: doing };

/** The copy of the lifted card that follows the pointer. */
const preview = { css: "[data-slot=drag-preview]" };

/** The live region announcing each step of a drag. */
const status = { role: "status" };

/** Drag a card into another column with a mouse. */
export const boardDragAcrossColumns = defineScenario({
    of: Board,
    interaction: viewInteraction,
    name: "drag-across-columns",
    description:
        "press a card, drag it into another column with its copy in sight, and drop it there",
    given: { examples: [boardLaunch] },
    parameters: {},
    when: [
        { step: { action: "pointerMove", target: card } },
        {
            step: { action: "pointerDown" },
            then: [{ observe: { kind: "count", target: preview }, is: 0 }],
        },
        {
            step: { action: "pointerMove", target: doing },
            then: [
                { observe: { kind: "visible", target: preview }, is: true },
                {
                    observe: { kind: "text", target: status },
                    is: "⁨Write the launch post⁩ moved to position 2 of 2 in ⁨Doing⁩.",
                },
            ],
        },
        {
            step: { action: "pointerUp" },
            then: [
                {
                    observe: { kind: "texts", target: doingCards },
                    is: ["Price the plans", "Write the launch post"],
                },
                { observe: { kind: "count", target: preview }, is: 0 },
            ],
        },
    ],
});

/** Drag a card into another column with a pen. */
export const boardDragAcrossColumnsByPen = defineScenario({
    of: Board,
    interaction: viewInteraction,
    name: "drag-across-columns-by-pen",
    description:
        "press a card with a pen, drag it into another column with its copy in sight, and drop it there",
    given: { examples: [boardLaunch], globals: { pointer: "pen" } },
    parameters: {},
    when: [
        { step: { action: "pointerMove", target: card } },
        {
            step: { action: "pointerDown" },
            then: [{ observe: { kind: "count", target: preview }, is: 0 }],
        },
        {
            step: { action: "pointerMove", target: doing },
            then: [
                { observe: { kind: "visible", target: preview }, is: true },
                {
                    observe: { kind: "text", target: status },
                    is: "⁨Write the launch post⁩ moved to position 2 of 2 in ⁨Doing⁩.",
                },
            ],
        },
        {
            step: { action: "pointerUp" },
            then: [
                {
                    observe: { kind: "texts", target: doingCards },
                    is: ["Price the plans", "Write the launch post"],
                },
                { observe: { kind: "count", target: preview }, is: 0 },
            ],
        },
    ],
});

/** Drag a card into another column with a finger, after a long press on it. */
export const boardLongPressAndDragAcrossColumns = defineScenario({
    of: Board,
    interaction: viewInteraction,
    name: "long-press-and-drag-across-columns",
    description: "long-press a card until it lifts, drag it into another column and drop it there",
    given: { examples: [boardLaunch], globals: { pointer: "touch" } },
    parameters: {},
    when: [
        { step: { action: "pointerMove", target: card } },
        {
            step: { action: "pointerDown" },
            then: [{ observe: { kind: "count", target: preview }, is: 0 }],
        },
        {
            step: { action: "pause", milliseconds: 500 },
            then: [{ observe: { kind: "visible", target: preview }, is: true }],
        },
        { step: { action: "pointerMove", target: doing } },
        {
            step: { action: "pointerUp" },
            then: [
                {
                    observe: { kind: "texts", target: doingCards },
                    is: ["Price the plans", "Write the launch post"],
                },
            ],
        },
    ],
});

/** The cards of the long board's second column, Next. */
const nextCards = {
    css: "[data-slot=board-item]",
    within: { css: "[data-slot=board-column]", nth: 1 },
};

/** Jump to the end of a long column by keys, past the cards in view, and move its last card. */
export const boardWalkLongColumn = defineScenario({
    of: Board,
    interaction: viewInteraction,
    name: "walk-long-column",
    description:
        "jump to the end of a backlog that mounts only the cards in view, then move its last card across",
    given: { examples: [boardLongBacklog] },
    parameters: {},
    when: [
        { step: { action: "focus", target: { role: "option", name: "Ticket 1" } } },
        { step: { action: "press", key: "End" } },
        {
            step: { action: "pause", milliseconds: 50 },
            then: [{ observe: { kind: "focused" }, is: "Ticket 500" }],
        },
        {
            step: { action: "press", key: "Alt+Shift+ArrowRight" },
            then: [{ observe: { kind: "texts", target: nextCards }, is: ["Ticket 500"] }],
        },
    ],
});
