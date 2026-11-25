import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { resizablePanelGroupInboxSplit } from "./resizable.example.tsx";
import { ResizablePanelGroup } from "./resizable.tsx";

/** The splitter after the inbox. */
const splitter = { role: "separator", name: "Resize the inbox" } as const;

/** The group, which stays while its splitter moves. */
const group = { css: "[data-slot=resizable-panel-group]" };

/** Move a splitter to pixel limits: 240 and 480 pixels of an 800 pixel group. */
export const resizablePanelGroupMoveToPixelLimits = defineScenario({
    of: ResizablePanelGroup,
    interaction: viewInteraction,
    name: "move-to-pixel-limits",
    description: "move a splitter with Home and End to its panel's limits in pixels",
    given: { examples: [resizablePanelGroupInboxSplit] },
    parameters: {},
    when: [
        {
            step: { action: "focus", target: splitter },
            then: [
                {
                    observe: { kind: "attribute", target: splitter, name: "aria-valuemin" },
                    is: "30",
                },
            ],
        },
        {
            step: { action: "press", key: "Home" },
            then: [
                {
                    observe: { kind: "attribute", target: splitter, name: "aria-valuenow" },
                    is: "30",
                },
            ],
        },
        {
            step: { action: "press", key: "End" },
            then: [
                {
                    observe: { kind: "attribute", target: splitter, name: "aria-valuenow" },
                    is: "60",
                },
            ],
        },
    ],
});

/** Drag a splitter from beside its line. */
export const resizablePanelGroupDragBeside = defineScenario({
    of: ResizablePanelGroup,
    interaction: viewInteraction,
    name: "drag-beside",
    description: "drag a splitter from a few pixels beside its line",
    given: { examples: [resizablePanelGroupInboxSplit], globals: { locale: "en-US" } },
    parameters: {},
    when: [
        { step: { action: "pointerMove", target: group, position: { x: 324, y: 100 } } },
        { step: { action: "pointerDown" } },
        { step: { action: "pointerMove", target: group, position: { x: 404, y: 100 } } },
        {
            step: { action: "pointerUp" },
            then: [
                {
                    observe: { kind: "attribute", target: splitter, name: "aria-valuenow" },
                    is: "50",
                },
            ],
        },
    ],
});

/** Drag a splitter right to left. */
export const resizablePanelGroupDragRightToLeft = defineScenario({
    of: ResizablePanelGroup,
    interaction: viewInteraction,
    name: "drag-right-to-left",
    description: "drag a splitter to the left to widen the panel at the right",
    given: { examples: [resizablePanelGroupInboxSplit], globals: { locale: "ar-EG" } },
    parameters: {},
    when: [
        { step: { action: "pointerMove", target: group, position: { x: 476, y: 100 } } },
        { step: { action: "pointerDown" } },
        { step: { action: "pointerMove", target: group, position: { x: 396, y: 100 } } },
        {
            step: { action: "pointerUp" },
            then: [
                {
                    observe: { kind: "attribute", target: splitter, name: "aria-valuenow" },
                    is: "50",
                },
            ],
        },
    ],
});
