import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import {
    dropdownMenuControlled,
    dropdownMenuNoteMenu,
    dropdownMenuUnavailableOptions,
} from "./dropdown-menu.example.tsx";
import { DropdownMenu } from "./dropdown-menu.tsx";

/** Move through an open dropdown menu with the keys. */
export const dropdownMenuMoveThroughItems = defineScenario({
    of: DropdownMenu,
    interaction: viewInteraction,
    name: "move-through-items",
    description: "open a dropdown menu from its trigger onto its first item and move through it",
    given: { examples: [dropdownMenuNoteMenu] },
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
    name: "close-on-escape",
    description: "close a dropdown menu on Escape and return the focus to its trigger",
    given: { examples: [dropdownMenuNoteMenu] },
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
    name: "choose-with-enter",
    description: "choose an item with Enter, running its action and closing the menu",
    given: { examples: [dropdownMenuNoteMenu] },
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
    name: "enter-and-leave-submenu",
    description: "open a submenu with the arrow key toward it and leave it with the arrow key back",
    given: { examples: [dropdownMenuNoteMenu] },
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

/** Leave unavailable options unchanged. */
export const dropdownMenuLeaveUnavailableOptions = defineScenario({
    of: DropdownMenu,
    interaction: viewInteraction,
    name: "leave-unavailable-options",
    description: "leave disabled checkbox and radio items unchanged, and a disabled submenu closed",
    given: { examples: [dropdownMenuUnavailableOptions] },
    parameters: {},
    when: [
        { action: "click", target: { role: "button", name: "View" } },
        { action: "click", target: { role: "menuitemcheckbox", name: "Show archived" } },
        { action: "click", target: { role: "menuitemradio", name: "Date" } },
        { action: "click", target: { role: "menuitem", name: "Move to" } },
        { action: "press", key: "Enter", target: { role: "menuitem", name: "Move to" } },
        { action: "press", key: "ArrowRight" },
    ],
    then: [
        {
            observe: {
                kind: "state",
                target: { role: "menuitemcheckbox", name: "Show archived" },
                state: "checked",
            },
            is: false,
        },
        {
            observe: {
                kind: "state",
                target: { role: "menuitemradio", name: "Date" },
                state: "checked",
            },
            is: false,
        },
        {
            observe: {
                kind: "state",
                target: { role: "menuitem", name: "Move to" },
                state: "expanded",
            },
            is: false,
        },
        { observe: { kind: "text", target: { role: "status" } }, is: "" },
    ],
});

/** Open a menu by its owner onto its first item. */
export const dropdownMenuOpenByOwner = defineScenario({
    of: DropdownMenu,
    interaction: viewInteraction,
    name: "open-by-owner",
    description: "open a controlled dropdown menu by its open property onto its first item",
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
    name: "stay-open-until-owner-closes",
    description:
        "ask to close a controlled dropdown menu on Escape, keeping the focus inside until its owner closes it",
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
                { observe: { kind: "text", target: { role: "status" } }, is: "" },
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
                { observe: { kind: "text", target: { role: "status" } }, is: "false" },
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
                { observe: { kind: "text", target: { role: "status" } }, is: "false" },
                { observe: { kind: "focused" }, is: "Note" },
            ],
        },
    ],
});
