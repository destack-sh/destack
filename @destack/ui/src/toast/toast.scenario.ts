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
    when: [
        { action: "click", target: { role: "button", name: "Archive" } },
        { action: "press", key: "Alt+t" },
    ],
    then: {
        observe: { focused: { kind: "focused" } },
        each: [{ focused: "Archive" }, { focused: "Notifications" }],
    },
});
