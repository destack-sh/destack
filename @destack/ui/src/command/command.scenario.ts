import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { commandNoteCommands, commandControlledHighlight } from "./command.example.tsx";
import { Command } from "./command.tsx";

/** Move a command list's highlight with the arrow keys and choose with Enter. */
export const commandMoveHighlightWithArrowKeys = defineScenario({
    of: Command,
    interaction: viewInteraction,
    name: "move-highlight-with-arrow-keys",
    description:
        "move the highlight with the arrow keys past disabled options, stopping at the ends, and choose with Enter",
    given: { examples: [commandNoteCommands] },
    when: [
        { action: "focus", target: { role: "combobox" } },
        { action: "press", key: "ArrowDown" },
        { action: "press", key: "ArrowDown" },
        { action: "press", key: "ArrowDown" },
        { action: "press", key: "ArrowUp" },
        { action: "press", key: "Enter" },
    ],
    then: {
        observe: {
            highlighted: { kind: "name", target: { role: "option", selected: true } },
            chosen: { kind: "text", target: { role: "status" } },
        },
        each: [
            { highlighted: "New note", chosen: "" },
            { highlighted: "Delete note", chosen: "" },
            { highlighted: "Appearance", chosen: "" },
            { highlighted: "Appearance", chosen: "" },
            { highlighted: "Delete note", chosen: "" },
            { highlighted: "Delete note", chosen: "Delete note" },
        ],
    },
});

/** Follow a highlight its owner holds. */
export const commandFollowControlledHighlight = defineScenario({
    of: Command,
    interaction: viewInteraction,
    name: "follow-controlled-highlight",
    description: "follow a controlled highlight and report the value the arrow keys move to",
    given: { examples: [commandControlledHighlight] },
    when: [
        { action: "press", key: "ArrowUp", target: { role: "combobox" } },
        { action: "set", properties: { value: "New note" } },
    ],
    then: {
        observe: {
            highlighted: { kind: "name", target: { role: "option", selected: true } },
            requested: { kind: "text", target: { role: "status" } },
        },
        each: [
            { highlighted: "Appearance", requested: "New note" },
            { highlighted: "New note", requested: "New note" },
        ],
    },
});

/** Jump to the first and last options with Home and End. */
export const commandJumpToEnds = defineScenario({
    of: Command,
    interaction: viewInteraction,
    name: "jump-to-ends",
    description:
        "move the highlight to the last enabled option with End and back to the first with Home",
    given: { examples: [commandNoteCommands] },
    when: [
        { action: "focus", target: { role: "combobox" } },
        { action: "press", key: "End" },
        { action: "press", key: "Home" },
    ],
    then: {
        observe: { highlighted: { kind: "name", target: { role: "option", selected: true } } },
        each: [
            { highlighted: "New note" },
            { highlighted: "Appearance" },
            { highlighted: "New note" },
        ],
    },
});
