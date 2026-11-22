import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { sidebarNotebooks } from "./sidebar.example.tsx";
import { Sidebar } from "./sidebar.tsx";

/** The sidebar's rail along its page-facing edge. */
const rail = { css: "[data-slot=sidebar-rail]" };

/** The sidebar. */
const sidebar = { css: "[data-slot=sidebar]" };

/** Widen the sidebar by its rail, collapse it past its narrowest, and click it open again. */
export const sidebarResizeWithRail = defineScenario({
    of: Sidebar,
    interaction: viewInteraction,
    name: "resize-with-rail",
    description:
        "drag the rail to widen the sidebar, drag it past its narrowest to collapse it, and click it open",
    given: { examples: [sidebarNotebooks] },
    parameters: {},
    when: [
        { step: { action: "pointerMove", target: rail, position: { x: 8, y: 200 } } },
        { step: { action: "pointerDown" } },
        { step: { action: "pointerMove", target: rail, position: { x: 108, y: 200 } } },
        {
            step: { action: "pointerUp" },
            then: [
                {
                    observe: { kind: "attribute", target: sidebar, name: "data-state" },
                    is: "expanded",
                },
            ],
        },
        { step: { action: "pointerMove", target: rail, position: { x: 8, y: 200 } } },
        { step: { action: "pointerDown" } },
        { step: { action: "pointerMove", target: rail, position: { x: -400, y: 200 } } },
        {
            step: { action: "pointerUp" },
            then: [
                {
                    observe: { kind: "attribute", target: sidebar, name: "data-state" },
                    is: "collapsed",
                },
            ],
        },
        {
            step: { action: "click", target: rail },
            then: [
                {
                    observe: { kind: "attribute", target: sidebar, name: "data-state" },
                    is: "expanded",
                },
            ],
        },
    ],
});
