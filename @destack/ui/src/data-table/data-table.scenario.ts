import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { DataTable } from "./data-table.tsx";
import { dataTableNotes } from "./data-table.example.tsx";

/** Move between a data table's cells with the arrow keys, Home and End. */
export const dataTableMoveBetweenCells = defineScenario({
    of: DataTable,
    interaction: viewInteraction,
    name: "move-between-cells",
    description:
        "move the focus between a data table's cells with the arrow keys, Home and End, onto a cell's single control",
    given: { examples: [dataTableNotes] },
    parameters: {},
    when: [
        {
            step: {
                action: "focus",
                target: { css: "tbody tr:first-child td[data-column=title]" },
            },
            then: [{ observe: { kind: "focused" }, is: "Groceries" }],
        },
        {
            step: { action: "press", key: "ArrowDown" },
            then: [{ observe: { kind: "focused" }, is: "Trip to Lisbon" }],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [{ observe: { kind: "focused" }, is: "940" }],
        },
        {
            step: { action: "press", key: "Home" },
            then: [{ observe: { kind: "focused" }, is: "Select row" }],
        },
    ],
});

/** Jump to a row's ends, a page down and the grid's ends. */
export const dataTableJumpToEnds = defineScenario({
    of: DataTable,
    interaction: viewInteraction,
    name: "jump-to-ends",
    description:
        "move the focus to a row's last cell with End, a page of rows down with Page Down, and to the grid's last and first cells with Control and End or Home",
    given: { examples: [dataTableNotes] },
    parameters: {},
    when: [
        {
            step: {
                action: "focus",
                target: { css: "tbody tr:first-child td[data-column=title]" },
            },
            then: [{ observe: { kind: "focused" }, is: "Groceries" }],
        },
        {
            step: { action: "press", key: "End" },
            then: [{ observe: { kind: "focused" }, is: "18" }],
        },
        {
            step: { action: "press", key: "PageDown" },
            then: [{ observe: { kind: "focused" }, is: "455" }],
        },
        {
            step: { action: "press", key: "PageUp" },
            then: [{ observe: { kind: "focused" }, is: "Words" }],
        },
        {
            step: { action: "press", key: "Control+End" },
            then: [{ observe: { kind: "focused" }, is: "455" }],
        },
        {
            step: { action: "press", key: "Control+Home" },
            then: [{ observe: { kind: "focused" }, is: "Select all" }],
        },
    ],
});
