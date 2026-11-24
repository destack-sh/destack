import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { sortTasks } from "./sort.example.tsx";
import { Sort } from "./sort.tsx";

/** The fields of an order. */
const items = { css: "[data-slot=sort-item]" };

/** Turn a field around, then remove each, which closes the panel. */
export const sortInPanel = defineScenario({
    of: Sort,
    interaction: viewInteraction,
    name: "panel",
    description: "turn a field around, then remove each, which closes the panel",
    given: { examples: [sortTasks] },
    parameters: {},
    when: [
        {
            step: { action: "click", target: { role: "button", name: "2 sorts" } },
            then: [{ observe: { kind: "count", target: items }, is: 2 }],
        },
        {
            step: { action: "click", target: { role: "button", name: "Newest first" } },
            then: [{ observe: { kind: "focused" }, is: "Oldest first" }],
        },
        {
            step: {
                action: "click",
                target: { css: "[data-slot=sort-item][aria-label=Due] [data-slot=sort-remove]" },
            },
            then: [{ observe: { kind: "focused" }, is: "Remove sort" }],
        },
        {
            step: { action: "press", key: "Enter" },
            then: [
                { observe: { kind: "focused" }, is: "Sort" },
                { observe: { kind: "count", target: items }, is: 0 },
            ],
        },
    ],
});
