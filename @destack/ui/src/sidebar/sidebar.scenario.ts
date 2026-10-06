import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { sidebarNotebooks } from "./sidebar.example.tsx";
import { Sidebar } from "./sidebar.tsx";

/** Collapse and expand the sidebar with its shortcut. */
export const sidebarToggleWithShortcut = defineScenario({
    of: Sidebar,
    interaction: viewInteraction,
    name: "toggle-with-shortcut",
    description: "collapse and expand the sidebar with Control+B or Meta+B",
    given: { examples: [sidebarNotebooks] },
    when: [
        { action: "press", key: "Control+b" },
        { action: "press", key: "Meta+b" },
    ],
    then: {
        observe: {
            expanded: {
                kind: "state",
                target: { role: "button", name: "Toggle sidebar", nth: 1 },
                state: "expanded",
            },
        },
        each: [{ expanded: false }, { expanded: true }],
    },
});
