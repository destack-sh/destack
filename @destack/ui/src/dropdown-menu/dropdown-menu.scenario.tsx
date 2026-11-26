import { t } from "@destack/locale";
import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { dropdownMenuControlled, dropdownMenuDisabled } from "./dropdown-menu.example.tsx";
import { DropdownMenu } from "./dropdown-menu.tsx";

/** Move through an open dropdown menu with the keys. */
export const dropdownMenuMoveThroughItems = defineScenario({
    of: DropdownMenu,
    interaction: viewInteraction,
    name: "moveThroughItems",
    description: t`Open a dropdown menu from its trigger onto its first item and move through it.`,
    given: { examples: [dropdownMenuDisabled] },
    parameters: {},
    when: [
        {
            step: { action: "click", target: { role: "button", name: "Note" } },
            then: [
                { observe: { kind: "focused" }, is: "Rename" },
                {
                    observe: {
                        kind: "state",
                        target: { role: "button", name: "Note" },
                        state: "expanded",
                    },
                    is: true,
                },
                {
                    observe: {
                        kind: "attribute",
                        target: { css: "[data-slot=dropdown-menu-content]" },
                        name: "data-popover-open",
                    },
                    is: "dropdown-menu-trigger",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowDown" },
            then: [
                { observe: { kind: "focused" }, is: "Move to" },
                {
                    observe: {
                        kind: "state",
                        target: { role: "button", name: "Note" },
                        state: "expanded",
                    },
                    is: true,
                },
                {
                    observe: {
                        kind: "attribute",
                        target: { css: "[data-slot=dropdown-menu-content]" },
                        name: "data-popover-open",
                    },
                    is: "dropdown-menu-trigger",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowDown" },
            then: [
                { observe: { kind: "focused" }, is: "Delete" },
                {
                    observe: {
                        kind: "state",
                        target: { role: "button", name: "Note" },
                        state: "expanded",
                    },
                    is: true,
                },
                {
                    observe: {
                        kind: "attribute",
                        target: { css: "[data-slot=dropdown-menu-content]" },
                        name: "data-popover-open",
                    },
                    is: "dropdown-menu-trigger",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowDown" },
            then: [
                { observe: { kind: "focused" }, is: "Rename" },
                {
                    observe: {
                        kind: "state",
                        target: { role: "button", name: "Note" },
                        state: "expanded",
                    },
                    is: true,
                },
                {
                    observe: {
                        kind: "attribute",
                        target: { css: "[data-slot=dropdown-menu-content]" },
                        name: "data-popover-open",
                    },
                    is: "dropdown-menu-trigger",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowUp" },
            then: [
                { observe: { kind: "focused" }, is: "Delete" },
                {
                    observe: {
                        kind: "state",
                        target: { role: "button", name: "Note" },
                        state: "expanded",
                    },
                    is: true,
                },
                {
                    observe: {
                        kind: "attribute",
                        target: { css: "[data-slot=dropdown-menu-content]" },
                        name: "data-popover-open",
                    },
                    is: "dropdown-menu-trigger",
                },
            ],
        },
        {
            step: { action: "press", key: "End" },
            then: [
                { observe: { kind: "focused" }, is: "Delete" },
                {
                    observe: {
                        kind: "state",
                        target: { role: "button", name: "Note" },
                        state: "expanded",
                    },
                    is: true,
                },
                {
                    observe: {
                        kind: "attribute",
                        target: { css: "[data-slot=dropdown-menu-content]" },
                        name: "data-popover-open",
                    },
                    is: "dropdown-menu-trigger",
                },
            ],
        },
        {
            step: { action: "press", key: "Home" },
            then: [
                { observe: { kind: "focused" }, is: "Rename" },
                {
                    observe: {
                        kind: "state",
                        target: { role: "button", name: "Note" },
                        state: "expanded",
                    },
                    is: true,
                },
                {
                    observe: {
                        kind: "attribute",
                        target: { css: "[data-slot=dropdown-menu-content]" },
                        name: "data-popover-open",
                    },
                    is: "dropdown-menu-trigger",
                },
            ],
        },
        {
            step: { action: "press", key: "d" },
            then: [
                { observe: { kind: "focused" }, is: "Delete" },
                {
                    observe: {
                        kind: "state",
                        target: { role: "button", name: "Note" },
                        state: "expanded",
                    },
                    is: true,
                },
                {
                    observe: {
                        kind: "attribute",
                        target: { css: "[data-slot=dropdown-menu-content]" },
                        name: "data-popover-open",
                    },
                    is: "dropdown-menu-trigger",
                },
            ],
        },
        {
            step: { action: "press", key: "m" },
            then: [
                { observe: { kind: "focused" }, is: "Move to" },
                {
                    observe: {
                        kind: "state",
                        target: { role: "button", name: "Note" },
                        state: "expanded",
                    },
                    is: true,
                },
                {
                    observe: {
                        kind: "attribute",
                        target: { css: "[data-slot=dropdown-menu-content]" },
                        name: "data-popover-open",
                    },
                    is: "dropdown-menu-trigger",
                },
            ],
        },
    ],
});

/** Close a dropdown menu on Escape. */
export const dropdownMenuCloseOnEscape = defineScenario({
    of: DropdownMenu,
    interaction: viewInteraction,
    name: "closeOnEscape",
    description: t`Close a dropdown menu on Escape and return the focus to its trigger.`,
    given: { examples: [dropdownMenuDisabled] },
    parameters: {},
    when: [
        {
            step: { action: "focus", target: { role: "button", name: "Note" } },
            then: [
                { observe: { kind: "focused" }, is: "Note" },
                {
                    observe: {
                        kind: "state",
                        target: { role: "button", name: "Note" },
                        state: "expanded",
                    },
                    is: false,
                },
                {
                    observe: {
                        kind: "attribute",
                        target: { css: "[data-slot=dropdown-menu-content]" },
                        name: "data-popover-open",
                    },
                    is: null,
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowUp" },
            then: [
                { observe: { kind: "focused" }, is: "Delete" },
                {
                    observe: {
                        kind: "state",
                        target: { role: "button", name: "Note" },
                        state: "expanded",
                    },
                    is: true,
                },
                {
                    observe: {
                        kind: "attribute",
                        target: { css: "[data-slot=dropdown-menu-content]" },
                        name: "data-popover-open",
                    },
                    is: "dropdown-menu-trigger",
                },
            ],
        },
        {
            step: { action: "press", key: "Escape" },
            then: [
                { observe: { kind: "focused" }, is: "Note" },
                {
                    observe: {
                        kind: "state",
                        target: { role: "button", name: "Note" },
                        state: "expanded",
                    },
                    is: false,
                },
                {
                    observe: {
                        kind: "attribute",
                        target: { css: "[data-slot=dropdown-menu-content]" },
                        name: "data-popover-open",
                    },
                    is: null,
                },
            ],
        },
    ],
});

/** Choose a dropdown menu's item with Enter. */
export const dropdownMenuChooseWithEnter = defineScenario({
    of: DropdownMenu,
    interaction: viewInteraction,
    name: "chooseWithEnter",
    description: t`Choose an item with Enter, running its action and closing the menu.`,
    given: { examples: [dropdownMenuDisabled] },
    parameters: {},
    when: [
        {
            step: { action: "click", target: { role: "button", name: "Note" } },
            then: [
                { observe: { kind: "focused" }, is: "Rename" },
                {
                    observe: {
                        kind: "state",
                        target: { role: "button", name: "Note" },
                        state: "expanded",
                    },
                    is: true,
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "" },
            ],
        },
        {
            step: { action: "press", key: "Enter" },
            then: [
                { observe: { kind: "focused" }, is: "Note" },
                {
                    observe: {
                        kind: "state",
                        target: { role: "button", name: "Note" },
                        state: "expanded",
                    },
                    is: false,
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "rename" },
            ],
        },
    ],
});

/** Enter and leave a submenu with the arrow keys. */
export const dropdownMenuEnterAndLeaveSubmenu = defineScenario({
    of: DropdownMenu,
    interaction: viewInteraction,
    name: "enterAndLeaveSubmenu",
    description: t`Open a submenu with the arrow key toward it and leave it with the arrow key back.`,
    given: { examples: [dropdownMenuDisabled] },
    parameters: {},
    when: [
        {
            step: { action: "click", target: { role: "button", name: "Note" } },
            then: [
                { observe: { kind: "focused" }, is: "Rename" },
                {
                    observe: {
                        kind: "state",
                        target: { css: "[data-slot=dropdown-menu-sub-trigger]" },
                        state: "expanded",
                    },
                    is: false,
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "" },
            ],
        },
        {
            step: { action: "press", key: "ArrowDown" },
            then: [
                { observe: { kind: "focused" }, is: "Move to" },
                {
                    observe: {
                        kind: "state",
                        target: { css: "[data-slot=dropdown-menu-sub-trigger]" },
                        state: "expanded",
                    },
                    is: false,
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "" },
            ],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [
                { observe: { kind: "focused" }, is: "Trips" },
                {
                    observe: {
                        kind: "state",
                        target: { css: "[data-slot=dropdown-menu-sub-trigger]" },
                        state: "expanded",
                    },
                    is: true,
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "" },
            ],
        },
        {
            step: { action: "press", key: "ArrowLeft" },
            then: [
                { observe: { kind: "focused" }, is: "Move to" },
                {
                    observe: {
                        kind: "state",
                        target: { css: "[data-slot=dropdown-menu-sub-trigger]" },
                        state: "expanded",
                    },
                    is: false,
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "" },
            ],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [
                { observe: { kind: "focused" }, is: "Trips" },
                {
                    observe: {
                        kind: "state",
                        target: { css: "[data-slot=dropdown-menu-sub-trigger]" },
                        state: "expanded",
                    },
                    is: true,
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "" },
            ],
        },
        {
            step: { action: "press", key: " " },
            then: [
                { observe: { kind: "focused" }, is: "Note" },
                {
                    observe: {
                        kind: "state",
                        target: { css: "[data-slot=dropdown-menu-sub-trigger]" },
                        state: "expanded",
                    },
                    is: false,
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "trips" },
            ],
        },
    ],
});

/** Leave an unavailable item without running it. */
export const dropdownMenuLeaveUnavailableOptions = defineScenario({
    of: DropdownMenu,
    interaction: viewInteraction,
    name: "leaveUnavailableOptions",
    description: t`Leave an unavailable item without running it, keeping the menu open.`,
    given: { examples: [dropdownMenuDisabled] },
    parameters: {},
    when: [
        { action: "click", target: { role: "button", name: "Note" } },
        { action: "click", target: { role: "menuitem", name: "Duplicate" } },
    ],
    then: [
        {
            observe: {
                kind: "state",
                target: { role: "button", name: "Note" },
                state: "expanded",
            },
            is: true,
        },
        { observe: { kind: "text", target: { role: "status", name: "Action" } }, is: "" },
    ],
});

/** Open a menu by its owner onto its first item. */
export const dropdownMenuOpenByOwner = defineScenario({
    of: DropdownMenu,
    interaction: viewInteraction,
    name: "openByOwner",
    description: t`Open a controlled dropdown menu by its open property onto its first item.`,
    given: { examples: [dropdownMenuControlled] },
    parameters: {},
    when: [{ action: "set", properties: { open: true } }],
    then: [
        {
            observe: {
                kind: "attribute",
                target: { css: "[data-slot=dropdown-menu-content]" },
                name: "data-popover-open",
            },
            is: "dropdown-menu-trigger",
        },
        { observe: { kind: "focused" }, is: "Rename" },
    ],
});

/** Keep a controlled menu open until its owner closes it. */
export const dropdownMenuStayOpenUntilOwnerCloses = defineScenario({
    of: DropdownMenu,
    interaction: viewInteraction,
    name: "stayOpenUntilOwnerCloses",
    description: t`Ask to close a controlled dropdown menu on Escape, with the focus inside until its owner closes it.`,
    given: { examples: [dropdownMenuControlled] },
    parameters: {},
    when: [
        {
            step: { action: "set", properties: { open: true } },
            then: [
                {
                    observe: {
                        kind: "attribute",
                        target: { css: "[data-slot=dropdown-menu-content]" },
                        name: "data-popover-open",
                    },
                    is: "dropdown-menu-trigger",
                },
                {
                    observe: { kind: "text", target: { role: "status", name: "Requested" } },
                    is: "",
                },
                { observe: { kind: "focused" }, is: "Rename" },
            ],
        },
        {
            step: { action: "press", key: "Escape" },
            then: [
                {
                    observe: {
                        kind: "attribute",
                        target: { css: "[data-slot=dropdown-menu-content]" },
                        name: "data-popover-open",
                    },
                    is: "dropdown-menu-trigger",
                },
                {
                    observe: { kind: "text", target: { role: "status", name: "Requested" } },
                    is: "false",
                },
                { observe: { kind: "focused" }, is: "Rename" },
            ],
        },
        {
            step: { action: "set", properties: { open: false } },
            then: [
                {
                    observe: {
                        kind: "attribute",
                        target: { css: "[data-slot=dropdown-menu-content]" },
                        name: "data-popover-open",
                    },
                    is: null,
                },
                {
                    observe: { kind: "text", target: { role: "status", name: "Requested" } },
                    is: "false",
                },
                { observe: { kind: "focused" }, is: "Note" },
            ],
        },
    ],
});
