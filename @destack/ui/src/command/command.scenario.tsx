import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import {
    commandControlledHighlight,
    commandNoteCommands,
    commandSearchDetail,
    commandTaskActions,
} from "./command.example.tsx";
import { Command } from "./command.tsx";

/** Move a command list's highlight with the arrow keys and choose with Enter. */
export const commandMoveHighlightWithArrowKeys = defineScenario({
    of: Command,
    interaction: viewInteraction,
    name: "move-highlight-with-arrow-keys",
    description:
        "move the highlight with the arrow keys past disabled options, stopping at the ends, and choose with Enter",
    given: { examples: [commandNoteCommands] },
    parameters: {},
    when: [
        {
            step: { action: "focus", target: { role: "combobox" } },
            then: [
                {
                    observe: { kind: "name", target: { role: "option", selected: true } },
                    is: "New note",
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "" },
            ],
        },
        {
            step: { action: "press", key: "ArrowDown" },
            then: [
                {
                    observe: { kind: "name", target: { role: "option", selected: true } },
                    is: "Delete note",
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "" },
            ],
        },
        {
            step: { action: "press", key: "ArrowDown" },
            then: [
                {
                    observe: { kind: "name", target: { role: "option", selected: true } },
                    is: "Appearance",
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "" },
            ],
        },
        {
            step: { action: "press", key: "ArrowDown" },
            then: [
                {
                    observe: { kind: "name", target: { role: "option", selected: true } },
                    is: "Appearance",
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "" },
            ],
        },
        {
            step: { action: "press", key: "ArrowUp" },
            then: [
                {
                    observe: { kind: "name", target: { role: "option", selected: true } },
                    is: "Delete note",
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "" },
            ],
        },
        {
            step: { action: "press", key: "Enter" },
            then: [
                {
                    observe: { kind: "name", target: { role: "option", selected: true } },
                    is: "Delete note",
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "Delete note" },
            ],
        },
    ],
});

/** Follow a highlight its owner controls. */
export const commandFollowControlledHighlight = defineScenario({
    of: Command,
    interaction: viewInteraction,
    name: "follow-controlled-highlight",
    description: "follow a controlled highlight and report the value the arrow keys move to",
    given: { examples: [commandControlledHighlight] },
    parameters: {},
    when: [
        {
            step: { action: "press", key: "ArrowUp", target: { role: "combobox" } },
            then: [
                {
                    observe: { kind: "name", target: { role: "option", selected: true } },
                    is: "Appearance",
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "New note" },
            ],
        },
        {
            step: { action: "set", properties: { value: "New note" } },
            then: [
                {
                    observe: { kind: "name", target: { role: "option", selected: true } },
                    is: "New note",
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "New note" },
            ],
        },
    ],
});

/** Jump to the first and last options with Home and End. */
export const commandJumpToEnds = defineScenario({
    of: Command,
    interaction: viewInteraction,
    name: "jump-to-ends",
    description:
        "move the highlight to the last enabled option with End and back to the first with Home",
    given: { examples: [commandNoteCommands] },
    parameters: {},
    when: [
        {
            step: { action: "focus", target: { role: "combobox" } },
            then: [
                {
                    observe: { kind: "name", target: { role: "option", selected: true } },
                    is: "New note",
                },
            ],
        },
        {
            step: { action: "press", key: "End" },
            then: [
                {
                    observe: { kind: "name", target: { role: "option", selected: true } },
                    is: "Appearance",
                },
            ],
        },
        {
            step: { action: "press", key: "Home" },
            then: [
                {
                    observe: { kind: "name", target: { role: "option", selected: true } },
                    is: "New note",
                },
            ],
        },
    ],
});

/** The chip of an open page. */
const pageChip = { css: "[data-slot=command-page-chip]" };

/** Open a page from an option, leave it with Backspace, and run a choice on another. */
export const commandPages = defineScenario({
    of: Command,
    interaction: viewInteraction,
    name: "pages",
    description: "open a page from an option, leave it with Backspace, and run a choice on it",
    given: { examples: [commandTaskActions] },
    parameters: {},
    when: [
        {
            step: { action: "focus", target: { role: "combobox" } },
            then: [{ observe: { kind: "count", target: pageChip }, is: 0 }],
        },
        {
            step: { action: "press", key: "Enter" },
            then: [
                { observe: { kind: "count", target: pageChip }, is: 1 },
                {
                    observe: { kind: "visible", target: { role: "option", name: "Ada Lovelace" } },
                    is: true,
                },
            ],
        },
        {
            step: { action: "press", key: "Backspace" },
            then: [
                { observe: { kind: "count", target: pageChip }, is: 0 },
                {
                    observe: { kind: "visible", target: { role: "option", name: "Ada Lovelace" } },
                    is: false,
                },
            ],
        },
        { step: { action: "press", key: "Enter" } },
        { step: { action: "press", key: "ArrowDown" } },
        {
            step: { action: "press", key: "Enter" },
            then: [
                { observe: { kind: "count", target: pageChip }, is: 0 },
                {
                    observe: { kind: "visible", target: { text: "Assigned to Jonas Weber" } },
                    is: true,
                },
            ],
        },
    ],
});

/** The detail beside a search's entries. */
const detail = { css: "[data-slot=command-detail] h3" };

/** Show the highlighted entry's detail beside the search's entries as the highlight moves. */
export const commandDetailFollowsHighlight = defineScenario({
    of: Command,
    interaction: viewInteraction,
    name: "detail-follows-highlight",
    description:
        "show the highlighted entry's detail beside the search's entries as the highlight moves",
    given: { examples: [commandSearchDetail] },
    parameters: {},
    when: [
        {
            step: { action: "focus", target: { role: "combobox" } },
            then: [{ observe: { kind: "name", target: detail }, is: "Trip to Lisbon" }],
        },
        {
            step: { action: "press", key: "ArrowDown" },
            then: [{ observe: { kind: "name", target: detail }, is: "Launch" }],
        },
    ],
});
