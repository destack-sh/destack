import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { comboboxMoveNote, comboboxTagNote } from "./combobox.example.tsx";
import { Combobox } from "./combobox.tsx";

/** Type into a combobox and choose an option with Enter. */
export const comboboxTypeAndChooseOption = defineScenario({
    of: Combobox,
    interaction: viewInteraction,
    name: "type-and-choose-option",
    description: "open a combobox's list as the person types, choose an option and close it",
    given: { examples: [comboboxMoveNote] },
    when: [
        { action: "fill", target: { role: "combobox", name: "Notebook" }, value: "wo" },
        { action: "press", key: "Enter" },
    ],
    then: {
        observe: {
            expanded: {
                kind: "state",
                target: { role: "combobox", name: "Notebook" },
                state: "expanded",
            },
            anchor: {
                kind: "attribute",
                target: { css: "[role=listbox]" },
                name: "data-popover-open",
            },
            highlighted: { kind: "texts", target: { role: "option", selected: true } },
            value: { kind: "value", target: { role: "combobox", name: "Notebook" } },
        },
        each: [
            { expanded: true, anchor: "combobox-input", highlighted: ["Work"], value: "wo" },
            { expanded: false, anchor: null, highlighted: [], value: "Work" },
        ],
    },
});

/** Close a combobox's list with Escape and clear it with a second Escape. */
export const comboboxCloseAndClearOnEscape = defineScenario({
    of: Combobox,
    interaction: viewInteraction,
    name: "close-and-clear-on-escape",
    description:
        "open a combobox's list with the down arrow key and close it with Escape, then clear the input",
    given: { examples: [comboboxMoveNote] },
    when: [
        { action: "fill", target: { role: "combobox", name: "Notebook" }, value: "tr" },
        { action: "press", key: "Escape" },
        { action: "press", key: "ArrowDown" },
        { action: "press", key: "Escape" },
        { action: "press", key: "Escape" },
    ],
    then: {
        observe: {
            expanded: {
                kind: "state",
                target: { role: "combobox", name: "Notebook" },
                state: "expanded",
            },
            value: { kind: "value", target: { role: "combobox", name: "Notebook" } },
        },
        each: [
            { expanded: true, value: "tr" },
            { expanded: false, value: "tr" },
            { expanded: true, value: "tr" },
            { expanded: false, value: "tr" },
            { expanded: false, value: "" },
        ],
    },
});

/** Choose several tags, keeping the list open, and drop the last with Backspace. */
export const comboboxChooseSeveralAndDropLast = defineScenario({
    of: Combobox,
    interaction: viewInteraction,
    name: "choose-several-and-drop-last",
    description:
        "add tags to a multiple combobox with Enter, keeping its list open, and drop the last chip with Backspace from the empty input",
    given: { examples: [comboboxTagNote] },
    when: [
        { action: "fill", target: { role: "combobox", name: "Tags" }, value: "fo" },
        { action: "press", key: "Enter" },
        { action: "press", key: "Backspace" },
    ],
    then: {
        observe: {
            chips: { kind: "texts", target: { css: "[data-slot=combobox-chip]" } },
            expanded: {
                kind: "state",
                target: { role: "combobox", name: "Tags" },
                state: "expanded",
            },
        },
        each: [
            { chips: ["travel"], expanded: true },
            { chips: ["travel", "food"], expanded: true },
            { chips: ["travel"], expanded: true },
        ],
    },
});

/** Create a tag from text no option has. */
export const comboboxCreateFromText = defineScenario({
    of: Combobox,
    interaction: viewInteraction,
    name: "create-from-text",
    description: "offer to create an option from text no option has, and choose it with Enter",
    given: { examples: [comboboxTagNote] },
    when: [
        { action: "fill", target: { role: "combobox", name: "Tags" }, value: "summer" },
        { action: "press", key: "Enter" },
    ],
    then: {
        observe: {
            options: { kind: "texts", target: { role: "option" } },
            chips: { kind: "texts", target: { css: "[data-slot=combobox-chip]" } },
        },
        each: [
            { options: ['Create "\u2068summer\u2069"'], chips: ["travel"] },
            { options: ["travel", "food", "family"], chips: ["travel", "summer"] },
        ],
    },
});
