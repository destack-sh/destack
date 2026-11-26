import { t } from "@destack/locale";
import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { filterTasks } from "./filter.example.tsx";
import { Filter } from "./filter.tsx";

/** The buttons of what conditions compare with. */
const values = { css: "[data-slot=filter-value]" };

/** Add a value by its name, then a text by its field, and remove both by the keyboard. */
export const filterByKeyboard = defineScenario({
    of: Filter,
    interaction: viewInteraction,
    name: "keyboard",
    description: t`Add a value by its name, then a text by its field, and remove both by the keyboard.`,
    given: { examples: [filterTasks] },
    parameters: {},
    when: [
        {
            step: { action: "press", key: "f" },
            then: [{ observe: { kind: "focused" }, is: "Filter by…" }],
        },
        {
            step: {
                action: "fill",
                target: { role: "combobox", name: "Filter by…" },
                value: "grace",
            },
        },
        {
            step: { action: "press", key: "Enter" },
            then: [
                { observe: { kind: "focused" }, is: "Grace" },
                { observe: { kind: "count", target: values }, is: 2 },
                { observe: { kind: "visible", target: { text: "Book the train" } }, is: false },
            ],
        },
        { step: { action: "press", key: "f" } },
        {
            step: {
                action: "fill",
                target: { role: "combobox", name: "Filter by…" },
                value: "title",
            },
        },
        {
            step: { action: "press", key: "Enter" },
            then: [{ observe: { kind: "focused" }, is: "Title" }],
        },
        { step: { action: "fill", target: { role: "textbox", name: "Title" }, value: "pa" } },
        {
            step: { action: "press", key: "Enter" },
            then: [{ observe: { kind: "focused" }, is: "pa" }],
        },
        { step: { action: "press", key: "Tab" } },
        {
            step: { action: "press", key: "Enter" },
            then: [
                { observe: { kind: "focused" }, is: "Remove filter" },
                { observe: { kind: "count", target: values }, is: 2 },
            ],
        },
        {
            step: { action: "press", key: "Enter" },
            then: [{ observe: { kind: "count", target: values }, is: 1 }],
        },
    ],
});
