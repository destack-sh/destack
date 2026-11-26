import { t } from "@destack/locale";
import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { timelinePlan } from "./timeline.example.tsx";
import { Timeline } from "./index.ts";

/** The marks of the axis. */
const marks = { css: "[data-slot=timeline-axis-mark]" };

/** The days marked while the window shows every task, from 2 to 16 November. */
const FIT = ["2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16"];

/** The first task's row. */
const row = { css: "[data-slot=timeline-row][data-key=design]" };

/** Zoom a plan in with a key and the wheel, pan it, and fit it again. */
export const timelineZoomAndPan = defineScenario({
    of: Timeline,
    interaction: viewInteraction,
    name: "zoomAndPan",
    description: t`Zoom a plan in with the plus key, pan it with a horizontal wheel, and fit every task again with zero.`,
    given: { examples: [timelinePlan] },
    parameters: {},
    when: [
        {
            step: { action: "focus", target: row },
            then: [{ observe: { kind: "texts", target: marks }, is: FIT }],
        },
        {
            step: { action: "press", key: "+" },
            then: [
                {
                    observe: { kind: "texts", target: marks },
                    is: ["5", "6", "7", "8", "9", "10", "11", "12", "13"],
                },
            ],
        },
        { step: { action: "pointerMove", target: { css: "[data-slot=timeline-body]" } } },
        {
            step: { action: "wheel", deltaX: 300 },
            then: [
                {
                    observe: { kind: "texts", target: marks },
                    is: ["8", "9", "10", "11", "12", "13", "14", "15", "16", "17"],
                },
            ],
        },
        {
            step: { action: "press", key: "0", target: row },
            then: [{ observe: { kind: "texts", target: marks }, is: FIT }],
        },
    ],
});
