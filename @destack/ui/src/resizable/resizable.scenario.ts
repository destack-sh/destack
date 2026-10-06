import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import {
    resizablePanelGroupNotebookCollapsed,
    resizablePanelGroupNotebookSplit,
} from "./resizable.example.tsx";
import { ResizablePanelGroup } from "./resizable.tsx";

/** Move a splitter with the arrow keys, Home and End. */
export const resizablePanelGroupMoveWithKeys = defineScenario({
    of: ResizablePanelGroup,
    interaction: viewInteraction,
    name: "move-with-keys",
    description:
        "move a splitter by a step with the arrow keys and to its limits with Home and End",
    given: { examples: [resizablePanelGroupNotebookSplit] },
    when: [
        { action: "focus", target: { role: "separator", name: "Resize the notebook list" } },
        { action: "press", key: "ArrowRight" },
        { action: "press", key: "ArrowRight" },
        { action: "press", key: "ArrowRight" },
        { action: "press", key: "Home" },
        { action: "press", key: "End" },
    ],
    then: {
        observe: {
            share: {
                kind: "attribute",
                target: { role: "separator", name: "Resize the notebook list" },
                name: "aria-valuenow",
            },
        },
        each: [
            { share: "30" },
            { share: "40" },
            { share: "50" },
            { share: "50" },
            { share: "20" },
            { share: "50" },
        ],
    },
});

/** Collapse a collapsible panel with Enter and restore it. */
export const resizablePanelGroupCollapseWithEnter = defineScenario({
    of: ResizablePanelGroup,
    interaction: viewInteraction,
    name: "collapse-with-enter",
    description:
        "collapse a collapsible panel with Enter on its splitter and restore it to its smallest share",
    given: { examples: [resizablePanelGroupNotebookCollapsed] },
    when: [
        { action: "focus", target: { role: "separator", name: "Resize the notebook list" } },
        { action: "press", key: "Enter" },
        { action: "press", key: "Enter" },
    ],
    then: {
        observe: {
            share: {
                kind: "attribute",
                target: { role: "separator", name: "Resize the notebook list" },
                name: "aria-valuenow",
            },
        },
        each: [{ share: "0" }, { share: "20" }, { share: "0" }],
    },
});
