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
    parameters: {},
    when: [
        {
            step: { action: "click", target: { role: "button", name: "Products" } },
            then: [
                {
                    observe: {
                        kind: "state",
                        target: { role: "button", name: "Products" },
                        state: "expanded",
                    },
                    is: true,
                },
                {
                    observe: {
                        kind: "visible",
                        target: { role: "link", name: "Tasks Plan the work with your team" },
                    },
                    is: true,
                },
                { observe: { kind: "focused" }, is: "Products" },
            ],
        },
        {
            step: {
                action: "focus",
                target: { role: "link", name: "Tasks Plan the work with your team" },
            },
            then: [
                {
                    observe: {
                        kind: "state",
                        target: { role: "button", name: "Products" },
                        state: "expanded",
                    },
                    is: true,
                },
                {
                    observe: {
                        kind: "visible",
                        target: { role: "link", name: "Tasks Plan the work with your team" },
                    },
                    is: true,
                },
                { observe: { kind: "focused" }, is: "Tasks Plan the work with your team" },
            ],
        },
        {
            step: { action: "press", key: "Escape" },
            then: [
                {
                    observe: {
                        kind: "state",
                        target: { role: "button", name: "Products" },
                        state: "expanded",
                    },
                    is: false,
                },
                {
                    observe: {
                        kind: "visible",
                        target: { role: "link", name: "Tasks Plan the work with your team" },
                    },
                    is: false,
                },
                { observe: { kind: "focused" }, is: "Products" },
            ],
        },
    ],
});

/** Move between a navigation menu's top-level entries. */
export const navigationMenuMoveBetweenEntries = defineScenario({
    of: NavigationMenu,
    interaction: viewInteraction,
    name: "move-between-entries",
    description:
        "move between a navigation menu's top-level entries with left and right, leaving the panel's links out",
    given: { examples: [navigationMenuSiteNavigation] },
    parameters: {},
    when: [
        {
            step: { action: "focus", target: { role: "button", name: "Products" } },
            then: [
                { observe: { kind: "focused" }, is: "Products" },
                {
                    observe: { kind: "name", target: { css: "[aria-current=page]" } },
                    is: "Pricing",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [
                { observe: { kind: "focused" }, is: "Pricing" },
                {
                    observe: { kind: "name", target: { css: "[aria-current=page]" } },
                    is: "Pricing",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [
                { observe: { kind: "focused" }, is: "Products" },
                {
                    observe: { kind: "name", target: { css: "[aria-current=page]" } },
                    is: "Pricing",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowLeft" },
            then: [
                { observe: { kind: "focused" }, is: "Pricing" },
                {
                    observe: { kind: "name", target: { css: "[aria-current=page]" } },
                    is: "Pricing",
                },
            ],
        },
    ],
});

/** Carry Tab through an open navigation panel. */
export const navigationMenuTabThroughPanel = defineScenario({
    of: NavigationMenu,
    interaction: viewInteraction,
    name: "tab-through-panel",
    description:
        "move Tab from an open trigger into its panel and from the panel's last link on to the next trigger",
    given: { examples: [navigationMenuTwoPanels] },
    parameters: {},
    when: [
        {
            step: { action: "click", target: { role: "button", name: "Products" } },
            then: [{ observe: { kind: "focused" }, is: "Products" }],
        },
        {
            step: { action: "press", key: "Tab" },
            then: [
                { observe: { kind: "focused" }, is: "Notes Write things down and find them again" },
            ],
        },
        {
            step: {
                action: "focus",
                target: { role: "link", name: "Tasks Plan the work with your team" },
            },
            then: [{ observe: { kind: "focused" }, is: "Tasks Plan the work with your team" }],
        },
        {
            step: { action: "press", key: "Tab" },
            then: [{ observe: { kind: "focused" }, is: "Company" }],
        },
    ],
});
