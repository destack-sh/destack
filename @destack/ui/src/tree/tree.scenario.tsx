import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { treeWorkExpanded } from "./tree.example.tsx";
import { Tree } from "./tree.tsx";

/** Move through a tree, expanding and entering parents. */
export const treeMoveThroughItems = defineScenario({
    of: Tree,
    interaction: viewInteraction,
    name: "move-through-items",
    description: "move through visible items, expand and enter parents, and return to them",
    given: { examples: [treeWorkExpanded] },
    parameters: {},
    when: [
        {
            step: { action: "focus", target: { role: "treeitem", name: "Trips" } },
            then: [
                { observe: { kind: "focused" }, is: "Trips" },
                {
                    observe: {
                        kind: "state",
                        target: { role: "treeitem", name: "Trips" },
                        state: "expanded",
                    },
                    is: false,
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowDown" },
            then: [
                { observe: { kind: "focused" }, is: "Work" },
                {
                    observe: {
                        kind: "state",
                        target: { role: "treeitem", name: "Trips" },
                        state: "expanded",
                    },
                    is: false,
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowDown" },
            then: [
                { observe: { kind: "focused" }, is: "Plans" },
                {
                    observe: {
                        kind: "state",
                        target: { role: "treeitem", name: "Trips" },
                        state: "expanded",
                    },
                    is: false,
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowUp" },
            then: [
                { observe: { kind: "focused" }, is: "Work" },
                {
                    observe: {
                        kind: "state",
                        target: { role: "treeitem", name: "Trips" },
                        state: "expanded",
                    },
                    is: false,
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowUp" },
            then: [
                { observe: { kind: "focused" }, is: "Trips" },
                {
                    observe: {
                        kind: "state",
                        target: { role: "treeitem", name: "Trips" },
                        state: "expanded",
                    },
                    is: false,
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [
                { observe: { kind: "focused" }, is: "Trips" },
                {
                    observe: {
                        kind: "state",
                        target: { role: "treeitem", name: "Trips" },
                        state: "expanded",
                    },
                    is: true,
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [
                { observe: { kind: "focused" }, is: "Lisbon" },
                {
                    observe: {
                        kind: "state",
                        target: { role: "treeitem", name: "Trips" },
                        state: "expanded",
                    },
                    is: true,
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowDown" },
            then: [
                { observe: { kind: "focused" }, is: "Porto" },
                {
                    observe: {
                        kind: "state",
                        target: { role: "treeitem", name: "Trips" },
                        state: "expanded",
                    },
                    is: true,
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowLeft" },
            then: [
                { observe: { kind: "focused" }, is: "Trips" },
                {
                    observe: {
                        kind: "state",
                        target: { role: "treeitem", name: "Trips" },
                        state: "expanded",
                    },
                    is: true,
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowLeft" },
            then: [
                { observe: { kind: "focused" }, is: "Trips" },
                {
                    observe: {
                        kind: "state",
                        target: { role: "treeitem", name: "Trips" },
                        state: "expanded",
                    },
                    is: false,
                },
            ],
        },
        {
            step: { action: "press", key: "End" },
            then: [
                { observe: { kind: "focused" }, is: "Recipes" },
                {
                    observe: {
                        kind: "state",
                        target: { role: "treeitem", name: "Trips" },
                        state: "expanded",
                    },
                    is: false,
                },
            ],
        },
        {
            step: { action: "press", key: "Home" },
            then: [
                { observe: { kind: "focused" }, is: "Trips" },
                {
                    observe: {
                        kind: "state",
                        target: { role: "treeitem", name: "Trips" },
                        state: "expanded",
                    },
                    is: false,
                },
            ],
        },
        {
            step: { action: "press", key: "r" },
            then: [
                { observe: { kind: "focused" }, is: "Recipes" },
                {
                    observe: {
                        kind: "state",
                        target: { role: "treeitem", name: "Trips" },
                        state: "expanded",
                    },
                    is: false,
                },
            ],
        },
    ],
});

/** Select tree items with Enter and a click. */
export const treeSelectWithEnterAndClick = defineScenario({
    of: Tree,
    interaction: viewInteraction,
    name: "select-with-enter-and-click",
    description: "select the focused item with Enter and a clicked item, reporting each",
    given: { examples: [treeWorkExpanded] },
    parameters: {},
    when: [
        {
            step: { action: "press", key: "Enter", target: { role: "treeitem", name: "Recipes" } },
            then: [
                {
                    observe: { kind: "texts", target: { role: "treeitem", selected: true } },
                    is: ["Recipes"],
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "recipes" },
            ],
        },
        {
            step: { action: "click", target: { role: "treeitem", name: "Plans" } },
            then: [
                {
                    observe: { kind: "texts", target: { role: "treeitem", selected: true } },
                    is: ["Plans"],
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "plans" },
            ],
        },
    ],
});

/** Keep the focus on a letter typed with a shortcut modifier. */
export const treeIgnoreShortcutLetters = defineScenario({
    of: Tree,
    interaction: viewInteraction,
    name: "ignore-shortcut-letters",
    description: "leave the focus in place on a letter typed with a shortcut modifier",
    given: { examples: [treeWorkExpanded] },
    parameters: {},
    when: [
        { action: "focus", target: { role: "treeitem", name: "Trips" } },
        { action: "press", key: "Control+r" },
        { action: "press", key: "Meta+r" },
        { action: "press", key: "Alt+r" },
    ],
    then: [{ observe: { kind: "focused" }, is: "Trips" }],
});
