import { t } from "@destack/locale";
import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { toasterDefault } from "./toast.example.tsx";
import { Toaster } from "./toast.tsx";

/** Move the focus to the toasts with their shortcut. */
export const toasterFocusWithShortcut = defineScenario({
    of: Toaster,
    interaction: viewInteraction,
    name: "focusWithShortcut",
    description: t`Move the focus to the toasts with F6.`,
    given: { examples: [toasterDefault] },
    parameters: {},
    when: [
        {
            step: { action: "click", target: { role: "button", name: "Archive" } },
            then: [{ observe: { kind: "focused" }, is: "Archive" }],
        },
        {
            step: { action: "press", key: "F6" },
            then: [{ observe: { kind: "focused" }, is: "Notifications" }],
        },
    ],
});
