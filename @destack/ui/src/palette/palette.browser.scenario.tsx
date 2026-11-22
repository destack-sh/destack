import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { commandPaletteLaunch } from "./palette.example.tsx";
import { CommandPalette } from "./palette.tsx";

/** The palette's dialog. */
const dialog = { role: "dialog", name: "Command palette" } as const;

/** The palette's search. */
const search = {
    css: "[data-slot=command-dialog] > [data-slot=command] [data-slot=command-input]",
} as const;

/** The chips of the pages opened over the root. */
const chips = { css: "[data-slot=command-page-chip]" } as const;

/** The actions panel. */
const panel = { css: "[data-slot=palette-actions]" } as const;

/** Search, drill into a page and back, open and close an entry's actions, then run one. */
export const commandPaletteNavigate = defineScenario({
    of: CommandPalette,
    interaction: viewInteraction,
    name: "navigate",
    description:
        "search, drill into a page and back, open an entry's actions and close them, then run an entry",
    given: { examples: [commandPaletteLaunch] },
    parameters: {},
    when: [
        {
            step: { action: "click", target: { role: "button", name: "Open palette" } },
            then: [{ observe: { kind: "visible", target: dialog }, is: true }],
        },
        {
            step: { action: "fill", target: search, value: "assign" },
            then: [{ observe: { kind: "count", target: chips }, is: 0 }],
        },
        {
            step: { action: "press", key: "Tab", target: search },
            then: [
                { observe: { kind: "count", target: chips }, is: 1 },
                {
                    observe: { kind: "visible", target: { role: "option", name: "Ada Lovelace" } },
                    is: true,
                },
            ],
        },
        {
            step: { action: "press", key: "Backspace", target: search },
            then: [{ observe: { kind: "count", target: chips }, is: 0 }],
        },
        { step: { action: "fill", target: search, value: "record" } },
        {
            step: { action: "press", key: "ControlOrMeta+k", target: search },
            then: [{ observe: { kind: "count", target: panel }, is: 1 }],
        },
        {
            step: { action: "press", key: "Escape" },
            then: [
                { observe: { kind: "count", target: panel }, is: 0 },
                { observe: { kind: "visible", target: dialog }, is: true },
            ],
        },
        {
            step: { action: "press", key: "Enter", target: search },
            then: [
                { observe: { kind: "visible", target: dialog }, is: false },
                {
                    observe: { kind: "visible", target: { text: "Opened Record the demo" } },
                    is: true,
                },
            ],
        },
    ],
});
