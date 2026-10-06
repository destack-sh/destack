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
    when: [
        { action: "click", target: { role: "button", name: "Note" } },
        { action: "press", key: "ArrowDown" },
        { action: "press", key: "ArrowDown" },
        { action: "press", key: "ArrowDown" },
        { action: "press", key: "ArrowUp" },
        { action: "press", key: "End" },
        { action: "press", key: "Home" },
        { action: "press", key: "d" },
        { action: "press", key: "m" },
    ],
    then: {
        observe: {
            focused: { kind: "focused" },
            expanded: {
                kind: "state",
                target: { role: "button", name: "Note" },
                state: "expanded",
            },
            anchor: {
                kind: "attribute",
                target: { css: "[data-slot=dropdown-menu-content]" },
                name: "data-popover-open",
            },
        },
        each: [
            { focused: "Rename", expanded: true, anchor: "dropdown-menu-trigger" },
            { focused: "Move to", expanded: true, anchor: "dropdown-menu-trigger" },
            { focused: "Delete", expanded: true, anchor: "dropdown-menu-trigger" },
            { focused: "Rename", expanded: true, anchor: "dropdown-menu-trigger" },
            { focused: "Delete", expanded: true, anchor: "dropdown-menu-trigger" },
            { focused: "Delete", expanded: true, anchor: "dropdown-menu-trigger" },
            { focused: "Rename", expanded: true, anchor: "dropdown-menu-trigger" },
            { focused: "Delete", expanded: true, anchor: "dropdown-menu-trigger" },
            { focused: "Move to", expanded: true, anchor: "dropdown-menu-trigger" },
        ],
    },
});

/** Close a dropdown menu on Escape. */
export const dropdownMenuCloseOnEscape = defineScenario({
    of: DropdownMenu,
    interaction: viewInteraction,
    name: "close-on-escape",
    description: "close a dropdown menu on Escape and return the focus to its trigger",
    given: { examples: [dropdownMenuNoteMenu] },
    when: [
        { action: "focus", target: { role: "button", name: "Note" } },
        { action: "press", key: "ArrowUp" },
        { action: "press", key: "Escape" },
    ],
    then: {
        observe: {
            focused: { kind: "focused" },
            expanded: {
                kind: "state",
                target: { role: "button", name: "Note" },
                state: "expanded",
            },
            anchor: {
                kind: "attribute",
                target: { css: "[data-slot=dropdown-menu-content]" },
                name: "data-popover-open",
            },
        },
        each: [
            { focused: "Note", expanded: false, anchor: null },
            { focused: "Delete", expanded: true, anchor: "dropdown-menu-trigger" },
            { focused: "Note", expanded: false, anchor: null },
        ],
    },
});

/** Choose a dropdown menu's item with Enter. */
export const dropdownMenuChooseWithEnter = defineScenario({
    of: DropdownMenu,
    interaction: viewInteraction,
    name: "choose-with-enter",
    description: "choose an item with Enter, running its action and closing the menu",
    given: { examples: [dropdownMenuNoteMenu] },
    when: [
        { action: "click", target: { role: "button", name: "Note" } },
        { action: "press", key: "Enter" },
    ],
    then: {
        observe: {
            focused: { kind: "focused" },
            expanded: {
                kind: "state",
                target: { role: "button", name: "Note" },
                state: "expanded",
            },
            action: { kind: "text", target: { role: "status" } },
        },
        each: [
            { focused: "Rename", expanded: true, action: "" },
            { focused: "Note", expanded: false, action: "rename" },
        ],
    },
});

/** Enter and leave a submenu with the arrow keys. */
export const dropdownMenuEnterAndLeaveSubmenu = defineScenario({
    of: DropdownMenu,
    interaction: viewInteraction,
    name: "enter-and-leave-submenu",
    description: "open a submenu with the arrow key toward it and leave it with the arrow key back",
    given: { examples: [dropdownMenuNoteMenu] },
    when: [
        { action: "click", target: { role: "button", name: "Note" } },
        { action: "press", key: "ArrowDown" },
        { action: "press", key: "ArrowRight" },
        { action: "press", key: "ArrowLeft" },
        { action: "press", key: "ArrowRight" },
        { action: "press", key: " " },
    ],
    then: {
        observe: {
            focused: { kind: "focused" },
            submenu: {
                kind: "state",
                target: { css: "[data-slot=dropdown-menu-sub-trigger]" },
                state: "expanded",
            },
            action: { kind: "text", target: { role: "status" } },
        },
        each: [
            { focused: "Rename", submenu: false, action: "" },
            { focused: "Move to", submenu: false, action: "" },
            { focused: "Trips", submenu: true, action: "" },
            { focused: "Move to", submenu: false, action: "" },
            { focused: "Trips", submenu: true, action: "" },
            { focused: "Note", submenu: false, action: "trips" },
        ],
    },
});

/** Leave unavailable options unchanged. */
export const dropdownMenuLeaveUnavailableOptions = defineScenario({
    of: DropdownMenu,
    interaction: viewInteraction,
    name: "leave-unavailable-options",
    description: "leave disabled checkbox and radio items unchanged, and a disabled submenu closed",
    given: { examples: [dropdownMenuUnavailableOptions] },
    when: [
        { action: "click", target: { role: "button", name: "View" } },
        { action: "click", target: { role: "menuitemcheckbox", name: "Show archived" } },
        { action: "click", target: { role: "menuitemradio", name: "Date" } },
        { action: "click", target: { role: "menuitem", name: "Move to" } },
        { action: "press", key: "Enter", target: { role: "menuitem", name: "Move to" } },
        { action: "press", key: "ArrowRight" },
    ],
    then: {
        observe: {
            archived: {
                kind: "state",
                target: { role: "menuitemcheckbox", name: "Show archived" },
                state: "checked",
            },
            date: {
                kind: "state",
                target: { role: "menuitemradio", name: "Date" },
                state: "checked",
            },
            submenu: {
                kind: "state",
                target: { role: "menuitem", name: "Move to" },
                state: "expanded",
            },
            changed: { kind: "text", target: { role: "status" } },
        },
        end: { archived: false, date: false, submenu: false, changed: "" },
    },
});

/** Open a menu by its owner onto its first item. */
export const dropdownMenuOpenByOwner = defineScenario({
    of: DropdownMenu,
    interaction: viewInteraction,
    name: "open-by-owner",
    description: "open a controlled dropdown menu by its open property onto its first item",
    given: { examples: [dropdownMenuControlled] },
    when: [{ action: "set", properties: { open: true } }],
    then: {
        observe: {
            shown: {
                kind: "attribute",
                target: { css: "[data-slot=dropdown-menu-content]" },
                name: "data-popover-open",
            },
            focused: { kind: "focused" },
        },
        end: { shown: "dropdown-menu-trigger", focused: "Rename" },
    },
});

/** Keep a controlled menu open until its owner closes it. */
export const dropdownMenuStayOpenUntilOwnerCloses = defineScenario({
    of: DropdownMenu,
    interaction: viewInteraction,
    name: "stay-open-until-owner-closes",
    description:
        "ask to close a controlled dropdown menu on Escape, keeping the focus inside until its owner closes it",
    given: { examples: [dropdownMenuControlled] },
    when: [
        { action: "set", properties: { open: true } },
        { action: "press", key: "Escape" },
        { action: "set", properties: { open: false } },
    ],
    then: {
        observe: {
            shown: {
                kind: "attribute",
                target: { css: "[data-slot=dropdown-menu-content]" },
                name: "data-popover-open",
            },
            requested: { kind: "text", target: { role: "status" } },
            focused: { kind: "focused" },
        },
        each: [
            { shown: "dropdown-menu-trigger", requested: "", focused: "Rename" },
            { shown: "dropdown-menu-trigger", requested: "false", focused: "Rename" },
            { shown: null, requested: "false", focused: "Note" },
        ],
    },
});
