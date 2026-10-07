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
    when: [
        { action: "focus", target: { role: "img", name: "Notes per notebook" } },
        { action: "press", key: "ArrowRight" },
        { action: "press", key: "ArrowLeft" },
    ],
    then: {
        observe: { announced: { kind: "name", target: { role: "status" } } },
        each: [
            { announced: "Notebook: Work\nNotes: 93" },
            { announced: "Notebook: Trips\nNotes: 36" },
            { announced: "Notebook: Work\nNotes: 93" },
        ],
    },
});
