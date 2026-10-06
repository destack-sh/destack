import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { contextMenuNoteCardActions } from "./context-menu.example.tsx";
import { ContextMenu } from "./context-menu.tsx";

/** Open a context menu at the pointer. */
export const contextMenuOpenAtPointer = defineScenario({
    of: ContextMenu,
    interaction: viewInteraction,
    name: "open-at-pointer",
    description: "open a context menu at the pointer onto its first item",
    given: { examples: [contextMenuNoteCardActions] },
    when: [
        {
            action: "rightClick",
            target: { text: "Groceries: milk, bread, apples" },
            position: { x: 40, y: 120 },
        },
    ],
    then: {
        observe: {
            anchor: { kind: "attribute", target: { role: "menu" }, name: "data-popover-open" },
            style: { kind: "attribute", target: { role: "menu" }, name: "style" },
            focused: { kind: "focused" },
        },
        end: { anchor: "undefined", style: "left: 40px; top: 120px;", focused: "Open ↵" },
    },
});
