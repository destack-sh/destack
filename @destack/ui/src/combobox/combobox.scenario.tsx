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
    parameters: {},
    when: [
        {
            step: { action: "fill", target: { role: "combobox", name: "Notebook" }, value: "wo" },
            then: [
                {
                    observe: {
                        kind: "state",
                        target: { role: "combobox", name: "Notebook" },
                        state: "expanded",
                    },
                    is: true,
                },
                {
                    observe: {
                        kind: "attribute",
                        target: { css: "[role=listbox]" },
                        name: "data-popover-open",
                    },
                    is: "combobox-input",
                },
                {
                    observe: { kind: "texts", target: { role: "option", selected: true } },
                    is: ["Work"],
                },
                {
                    observe: { kind: "value", target: { role: "combobox", name: "Notebook" } },
                    is: "wo",
                },
            ],
        },
        {
            step: { action: "press", key: "Enter" },
            then: [
                {
                    observe: {
                        kind: "state",
                        target: { role: "combobox", name: "Notebook" },
                        state: "expanded",
                    },
                    is: false,
                },
                {
                    observe: {
                        kind: "attribute",
                        target: { css: "[role=listbox]" },
                        name: "data-popover-open",
                    },
                    is: null,
                },
                { observe: { kind: "texts", target: { role: "option", selected: true } }, is: [] },
                {
                    observe: { kind: "value", target: { role: "combobox", name: "Notebook" } },
                    is: "Work",
                },
            ],
        },
    ],
});

/** Close a combobox's list with Escape and clear it with a second Escape. */
export const comboboxCloseAndClearOnEscape = defineScenario({
    of: Combobox,
    interaction: viewInteraction,
    name: "close-and-clear-on-escape",
    description:
        "open a combobox's list with the down arrow key and close it with Escape, then clear the input",
    given: { examples: [comboboxMoveNote] },
    parameters: {},
    when: [
        {
            step: { action: "fill", target: { role: "combobox", name: "Notebook" }, value: "tr" },
            then: [
                {
                    observe: {
                        kind: "state",
                        target: { role: "combobox", name: "Notebook" },
                        state: "expanded",
                    },
                    is: true,
                },
                {
                    observe: { kind: "value", target: { role: "combobox", name: "Notebook" } },
                    is: "tr",
                },
            ],
        },
        {
            step: { action: "press", key: "Escape" },
            then: [
                {
                    observe: {
                        kind: "state",
                        target: { role: "combobox", name: "Notebook" },
                        state: "expanded",
                    },
                    is: false,
                },
                {
                    observe: { kind: "value", target: { role: "combobox", name: "Notebook" } },
                    is: "tr",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowDown" },
            then: [
                {
                    observe: {
                        kind: "state",
                        target: { role: "combobox", name: "Notebook" },
                        state: "expanded",
                    },
                    is: true,
                },
                {
                    observe: { kind: "value", target: { role: "combobox", name: "Notebook" } },
                    is: "tr",
                },
            ],
        },
        {
            step: { action: "press", key: "Escape" },
            then: [
                {
                    observe: {
                        kind: "state",
                        target: { role: "combobox", name: "Notebook" },
                        state: "expanded",
                    },
                    is: false,
                },
                {
                    observe: { kind: "value", target: { role: "combobox", name: "Notebook" } },
                    is: "tr",
                },
            ],
        },
        {
            step: { action: "press", key: "Escape" },
            then: [
                {
                    observe: {
                        kind: "state",
                        target: { role: "combobox", name: "Notebook" },
                        state: "expanded",
                    },
                    is: false,
                },
                {
                    observe: { kind: "value", target: { role: "combobox", name: "Notebook" } },
                    is: "",
                },
            ],
        },
    ],
});

/** Choose several tags, keeping the list open, and drop the last with Backspace. */
export const comboboxChooseSeveralAndDropLast = defineScenario({
    of: Combobox,
    interaction: viewInteraction,
    name: "choose-several-and-drop-last",
    description:
        "add tags to a multiple combobox with Enter, keeping its list open, and drop the last chip with Backspace from the empty input",
    given: { examples: [comboboxTagNote] },
    parameters: {},
    when: [
        {
            step: { action: "fill", target: { role: "combobox", name: "Tags" }, value: "fo" },
            then: [
                {
                    observe: { kind: "texts", target: { css: "[data-slot=combobox-chip]" } },
                    is: ["travel"],
                },
                {
                    observe: {
                        kind: "state",
                        target: { role: "combobox", name: "Tags" },
                        state: "expanded",
                    },
                    is: true,
                },
            ],
        },
        {
            step: { action: "press", key: "Enter" },
            then: [
                {
                    observe: { kind: "texts", target: { css: "[data-slot=combobox-chip]" } },
                    is: ["travel", "food"],
                },
                {
                    observe: {
                        kind: "state",
                        target: { role: "combobox", name: "Tags" },
                        state: "expanded",
                    },
                    is: true,
                },
            ],
        },
        {
            step: { action: "press", key: "Backspace" },
            then: [
                {
                    observe: { kind: "texts", target: { css: "[data-slot=combobox-chip]" } },
                    is: ["travel"],
                },
                {
                    observe: {
                        kind: "state",
                        target: { role: "combobox", name: "Tags" },
                        state: "expanded",
                    },
                    is: true,
                },
            ],
        },
    ],
});

/** Create a tag from text no option has. */
export const comboboxCreateFromText = defineScenario({
    of: Combobox,
    interaction: viewInteraction,
    name: "create-from-text",
    description: "offer to create an option from text no option has, and choose it with Enter",
    given: { examples: [comboboxTagNote] },
    parameters: {},
    when: [
        {
            step: { action: "fill", target: { role: "combobox", name: "Tags" }, value: "summer" },
            then: [
                {
                    observe: { kind: "texts", target: { role: "option" } },
                    is: ['Create "\u2068summer\u2069"'],
                },
                {
                    observe: { kind: "texts", target: { css: "[data-slot=combobox-chip]" } },
                    is: ["travel"],
                },
            ],
        },
        {
            step: { action: "press", key: "Enter" },
            then: [
                {
                    observe: { kind: "texts", target: { role: "option" } },
                    is: ["travel", "food", "family"],
                },
                {
                    observe: { kind: "texts", target: { css: "[data-slot=combobox-chip]" } },
                    is: ["travel", "summer"],
                },
            ],
        },
    ],
});
