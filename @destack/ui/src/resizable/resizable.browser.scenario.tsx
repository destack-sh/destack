import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { resizablePanelGroupInboxSplit } from "./resizable.example.tsx";
import { ResizablePanelGroup } from "./resizable.tsx";

/** The splitter after the inbox. */
const splitter = { role: "separator", name: "Resize the inbox" } as const;

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
