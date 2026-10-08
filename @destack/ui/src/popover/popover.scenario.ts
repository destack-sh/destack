import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { popoverRenameModal } from "./popover.example.tsx";
import { Popover } from "./popover.tsx";

/** Hand the focus back to a modal popover's trigger as it closes. */
export const popoverRefocusTriggerOnClose = defineScenario({
    of: Popover,
    interaction: viewInteraction,
    name: "refocus-trigger-on-close",
    description:
        "close a modal popover from its close button and hand the focus back to its trigger",
    given: { examples: [popoverRenameModal] },
    when: [
        { action: "focus", target: { role: "textbox", name: "Title" } },
        { action: "click", target: { role: "button", name: "Save" } },
    ],
    then: {
        observe: {
            focused: { kind: "focused" },
            expanded: {
                kind: "state",
                target: { role: "button", name: "Rename" },
                state: "expanded",
            },
        },
        each: [
            { focused: "Title", expanded: true },
            { focused: "Rename", expanded: false },
        ],
    },
});
