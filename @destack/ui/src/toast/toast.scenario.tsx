import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { toasterArchiveUndo } from "./toast.example.tsx";
import { Toaster } from "./toast.tsx";

/** Move the focus to the toasts with their shortcut. */
export const toasterFocusWithShortcut = defineScenario({
    of: Toaster,
    interaction: viewInteraction,
    name: "focus-with-shortcut",
    description: "move the focus to the toasts with Alt+T",
    given: { examples: [toasterArchiveUndo] },
    parameters: {},
    when: [
        {
            step: { action: "click", target: { role: "button", name: "Archive" } },
            then: [{ observe: { kind: "focused" }, is: "Archive" }],
        },
        {
            step: { action: "press", key: "Alt+t" },
            then: [{ observe: { kind: "focused" }, is: "Notifications" }],
        },
    ],
});
