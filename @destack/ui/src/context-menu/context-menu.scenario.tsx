import { t } from "@destack/locale";
import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { contextMenuNoteCardActions } from "./context-menu.example.tsx";
import { ContextMenu } from "./context-menu.tsx";

/** Open a context menu at the pointer. */
export const contextMenuOpenAtPointer = defineScenario({
    of: ContextMenu,
    interaction: viewInteraction,
    name: "openAtPointer",
    description: t`Open a context menu at the pointer onto its first item.`,
    given: { examples: [contextMenuNoteCardActions] },
    parameters: {},
    when: [
        {
            action: "click",
            button: "secondary",
            target: { text: "Groceries: milk, bread, apples" },
            position: { x: 40, y: 120 },
        },
    ],
    then: [
        {
            observe: { kind: "attribute", target: { role: "menu" }, name: "data-popover-open" },
            is: "undefined",
        },
        {
            observe: { kind: "attribute", target: { role: "menu" }, name: "style" },
            is: "--x-left: 40px; --x-top: 120px;",
        },
        { observe: { kind: "focused" }, is: "Open" },
    ],
});
