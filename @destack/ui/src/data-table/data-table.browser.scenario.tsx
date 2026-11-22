import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { dataTableTasks } from "./data-table.example.tsx";
import { DataTable } from "./data-table.tsx";

/** The table's body rows. */
const rows = { css: "[data-slot=data-table] tbody tr" };

/** Narrow the tasks to one status with its faceted filter, then rename a task in place. */
export const dataTableFilterAndRename = defineScenario({
    of: DataTable,
    interaction: viewInteraction,
    name: "filter-and-rename",
    description:
        "narrow the tasks to one status with its faceted filter, then rename a task in place",
    given: { examples: [dataTableTasks] },
    parameters: {},
    when: [
        {
            step: { action: "click", target: { role: "button", name: "Status" } },
            then: [{ observe: { kind: "count", target: rows }, is: 4 }],
        },
        {
            step: { action: "click", target: { role: "option", name: "Doing 2" } },
            then: [{ observe: { kind: "count", target: rows }, is: 2 }],
        },
        { step: { action: "press", key: "Escape" } },
        {
            step: { action: "click", target: { role: "button", name: "Record the demo" } },
            then: [{ observe: { kind: "focused" }, is: "Title" }],
        },
        {
            step: {
                action: "fill",
                target: { role: "textbox", name: "Title" },
                value: "Record the launch demo",
            },
        },
        {
            step: { action: "press", key: "Enter" },
            then: [
                {
                    observe: {
                        kind: "visible",
                        target: { role: "button", name: "Record the launch demo" },
                    },
                    is: true,
                },
            ],
        },
    ],
});
