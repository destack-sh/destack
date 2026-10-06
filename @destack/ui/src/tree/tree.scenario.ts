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
    when: [
        { action: "focus", target: { role: "treeitem", name: "Trips" } },
        { action: "press", key: "ArrowDown" },
        { action: "press", key: "ArrowDown" },
        { action: "press", key: "ArrowUp" },
        { action: "press", key: "ArrowUp" },
        { action: "press", key: "ArrowRight" },
        { action: "press", key: "ArrowRight" },
        { action: "press", key: "ArrowDown" },
        { action: "press", key: "ArrowLeft" },
        { action: "press", key: "ArrowLeft" },
        { action: "press", key: "End" },
        { action: "press", key: "Home" },
        { action: "press", key: "r" },
    ],
    then: {
        observe: {
            focused: { kind: "focused" },
            trips: {
                kind: "state",
                target: { role: "treeitem", name: "Trips" },
                state: "expanded",
            },
        },
        each: [
            { focused: "Trips", trips: false },
            { focused: "Work", trips: false },
            { focused: "Plans", trips: false },
            { focused: "Work", trips: false },
            { focused: "Trips", trips: false },
            { focused: "Trips", trips: true },
            { focused: "Lisbon", trips: true },
            { focused: "Porto", trips: true },
            { focused: "Trips", trips: true },
            { focused: "Trips", trips: false },
            { focused: "Recipes", trips: false },
            { focused: "Trips", trips: false },
            { focused: "Recipes", trips: false },
        ],
    },
});

/** Select tree items with Enter and a click. */
export const treeSelectWithEnterAndClick = defineScenario({
    of: Tree,
    interaction: viewInteraction,
    name: "select-with-enter-and-click",
    description: "select the focused item with Enter and a clicked item, reporting each",
    given: { examples: [treeWorkExpanded] },
    when: [
        { action: "press", key: "Enter", target: { role: "treeitem", name: "Recipes" } },
        { action: "click", target: { role: "treeitem", name: "Plans" } },
    ],
    then: {
        observe: {
            selected: { kind: "texts", target: { role: "treeitem", selected: true } },
            reported: { kind: "text", target: { role: "status" } },
        },
        each: [
            { selected: ["Recipes"], reported: "recipes" },
            { selected: ["Plans"], reported: "plans" },
        ],
    },
});

/** Keep the focus on a letter typed with a shortcut modifier. */
export const treeIgnoreShortcutLetters = defineScenario({
    of: Tree,
    interaction: viewInteraction,
    name: "ignore-shortcut-letters",
    description: "leave the focus in place on a letter typed with a shortcut modifier",
    given: { examples: [treeWorkExpanded] },
    when: [
        { action: "focus", target: { role: "treeitem", name: "Trips" } },
        { action: "press", key: "Control+r" },
        { action: "press", key: "Meta+r" },
        { action: "press", key: "Alt+r" },
    ],
    then: { observe: { focused: { kind: "focused" } }, end: { focused: "Trips" } },
});
