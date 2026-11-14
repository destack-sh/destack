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
    parameters: {},
    when: [
        {
            step: {
                action: "focus",
                target: { role: "separator", name: "Resize the notebook list" },
            },
            then: [
                {
                    observe: {
                        kind: "attribute",
                        target: { role: "separator", name: "Resize the notebook list" },
                        name: "aria-valuenow",
                    },
                    is: "30",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [
                {
                    observe: {
                        kind: "attribute",
                        target: { role: "separator", name: "Resize the notebook list" },
                        name: "aria-valuenow",
                    },
                    is: "40",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [
                {
                    observe: {
                        kind: "attribute",
                        target: { role: "separator", name: "Resize the notebook list" },
                        name: "aria-valuenow",
                    },
                    is: "50",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [
                {
                    observe: {
                        kind: "attribute",
                        target: { role: "separator", name: "Resize the notebook list" },
                        name: "aria-valuenow",
                    },
                    is: "50",
                },
            ],
        },
        {
            step: { action: "press", key: "Home" },
            then: [
                {
                    observe: {
                        kind: "attribute",
                        target: { role: "separator", name: "Resize the notebook list" },
                        name: "aria-valuenow",
                    },
                    is: "20",
                },
            ],
        },
        {
            step: { action: "press", key: "End" },
            then: [
                {
                    observe: {
                        kind: "attribute",
                        target: { role: "separator", name: "Resize the notebook list" },
                        name: "aria-valuenow",
                    },
                    is: "50",
                },
            ],
        },
    ],
});

/** Collapse a collapsible panel with Enter and restore it. */
export const resizablePanelGroupCollapseWithEnter = defineScenario({
    of: ResizablePanelGroup,
    interaction: viewInteraction,
    name: "collapse-with-enter",
    description:
        "collapse a collapsible panel with Enter on its splitter and restore it to its smallest share",
    given: { examples: [resizablePanelGroupNotebookCollapsed] },
    parameters: {},
    when: [
        {
            step: {
                action: "focus",
                target: { role: "separator", name: "Resize the notebook list" },
            },
            then: [
                {
                    observe: {
                        kind: "attribute",
                        target: { role: "separator", name: "Resize the notebook list" },
                        name: "aria-valuenow",
                    },
                    is: "0",
                },
            ],
        },
        {
            step: { action: "press", key: "Enter" },
            then: [
                {
                    observe: {
                        kind: "attribute",
                        target: { role: "separator", name: "Resize the notebook list" },
                        name: "aria-valuenow",
                    },
                    is: "20",
                },
            ],
        },
        {
            step: { action: "press", key: "Enter" },
            then: [
                {
                    observe: {
                        kind: "attribute",
                        target: { role: "separator", name: "Resize the notebook list" },
                        name: "aria-valuenow",
                    },
                    is: "0",
                },
            ],
        },
    ],
});
