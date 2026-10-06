import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import {
    navigationMenuSiteNavigation,
    navigationMenuTwoPanels,
} from "./navigation-menu.example.tsx";
import { NavigationMenu } from "./navigation-menu.tsx";

/** Close a navigation menu's panel on Escape. */
export const navigationMenuClosePanelOnEscape = defineScenario({
    of: NavigationMenu,
    interaction: viewInteraction,
    name: "close-panel-on-escape",
    description: "disclose a navigation menu's panel from its trigger and close it on Escape",
    given: { examples: [navigationMenuSiteNavigation] },
    when: [
        { action: "click", target: { role: "button", name: "Products" } },
        { action: "focus", target: { role: "link", name: "Tasks" } },
        { action: "press", key: "Escape" },
    ],
    then: {
        observe: {
            expanded: {
                kind: "state",
                target: { role: "button", name: "Products" },
                state: "expanded",
            },
            panel: { kind: "visible", target: { role: "link", name: "Tasks" } },
            focused: { kind: "focused" },
        },
        each: [
            { expanded: true, panel: true, focused: "Products" },
            { expanded: true, panel: true, focused: "Tasks" },
            { expanded: false, panel: false, focused: "Products" },
        ],
    },
});

/** Move between a navigation menu's top-level entries. */
export const navigationMenuMoveBetweenEntries = defineScenario({
    of: NavigationMenu,
    interaction: viewInteraction,
    name: "move-between-entries",
    description:
        "move between a navigation menu's top-level entries with left and right, leaving the panel's links out",
    given: { examples: [navigationMenuSiteNavigation] },
    when: [
        { action: "focus", target: { role: "button", name: "Products" } },
        { action: "press", key: "ArrowRight" },
        { action: "press", key: "ArrowRight" },
        { action: "press", key: "ArrowLeft" },
    ],
    then: {
        observe: {
            focused: { kind: "focused" },
            current: { kind: "name", target: { css: "[aria-current=page]" } },
        },
        each: [
            { focused: "Products", current: "Pricing" },
            { focused: "Pricing", current: "Pricing" },
            { focused: "Products", current: "Pricing" },
            { focused: "Pricing", current: "Pricing" },
        ],
    },
});

/** Carry Tab through an open navigation panel. */
export const navigationMenuTabThroughPanel = defineScenario({
    of: NavigationMenu,
    interaction: viewInteraction,
    name: "tab-through-panel",
    description:
        "carry Tab from an open trigger into its panel and from the panel's last link on to the next trigger",
    given: { examples: [navigationMenuTwoPanels] },
    when: [
        { action: "click", target: { role: "button", name: "Products" } },
        { action: "press", key: "Tab" },
        { action: "focus", target: { role: "link", name: "Tasks" } },
        { action: "press", key: "Tab" },
    ],
    then: {
        observe: { focused: { kind: "focused" } },
        each: [
            { focused: "Products" },
            { focused: "Notes" },
            { focused: "Tasks" },
            { focused: "Company" },
        ],
    },
});
