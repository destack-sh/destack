import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { chartBar } from "./chart.example.tsx";
import { Chart } from "./chart.tsx";

/** Focus a chart's first point and move between its points with the arrow keys, each announced. */
export const chartMoveBetweenPoints = defineScenario({
    of: Chart,
    interaction: viewInteraction,
    name: "move-between-points",
    description:
        "focus a chart to focus its first point, then move to the next point and back with the arrow keys, each announced in a status",
    given: { examples: [chartBar] },
    parameters: {},
    when: [
        {
            step: { action: "focus", target: { role: "img", name: "Notes per notebook" } },
            then: [
                {
                    observe: { kind: "name", target: { role: "status" } },
                    is: "Notebook: Work\nNotes: 93",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [
                {
                    observe: { kind: "name", target: { role: "status" } },
                    is: "Notebook: Trips\nNotes: 36",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowLeft" },
            then: [
                {
                    observe: { kind: "name", target: { role: "status" } },
                    is: "Notebook: Work\nNotes: 93",
                },
            ],
        },
    ],
});
