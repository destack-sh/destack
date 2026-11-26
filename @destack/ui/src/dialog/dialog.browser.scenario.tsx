import { t } from "@destack/locale";
import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { Dialog } from "./dialog.tsx";
import { dialogDefault } from "./dialog.example.tsx";

/** The rename dialog. */
const dialog = { role: "dialog", name: "Rename note" };

/** Open a dialog from its trigger, dismiss it with Escape, and return the focus to the trigger. */
export const dialogOpenAndEscape = defineScenario({
    of: Dialog,
    interaction: viewInteraction,
    name: "openAndEscape",
    description: t`Open a dialog from its trigger, dismiss it with Escape and focus the trigger again.`,
    given: { examples: [dialogDefault] },
    parameters: {},
    when: [
        {
            step: { action: "click", target: { role: "button", name: "Rename" } },
            then: [{ observe: { kind: "visible", target: dialog }, is: true }],
        },
        {
            step: { action: "press", key: "Escape" },
            then: [
                { observe: { kind: "visible", target: dialog }, is: false },
                { observe: { kind: "focused" }, is: "Rename" },
            ],
        },
    ],
});
