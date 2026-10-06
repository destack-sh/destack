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
    when: [
        { action: "focus", target: { css: "tbody tr:first-child td[data-column=title]" } },
        { action: "press", key: "ArrowDown" },
        { action: "press", key: "ArrowRight" },
        { action: "press", key: "Home" },
    ],
    then: {
        observe: { focused: { kind: "focused" } },
        each: [
            { focused: "Groceries" },
            { focused: "Trip to Lisbon" },
            { focused: "940" },
            { focused: "Select row" },
        ],
    },
});
