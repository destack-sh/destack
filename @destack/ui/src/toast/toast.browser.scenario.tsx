import { t } from "@destack/locale";
import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { toasterDefault, toasterInDialog } from "./toast.example.tsx";
import { Toaster } from "./toast.tsx";

/** The one toast shown. */
const toast = { css: "[data-slot=toast]:not([data-leaving], [data-limited])" };

/** Swipe a toast away with a finger. */
export const toasterSwipeAway = defineScenario({
    of: Toaster,
    interaction: viewInteraction,
    name: "swipeAway",
    description: t`Show a toast, swipe it toward its edge with a finger and see it dismissed.`,
    given: { examples: [toasterDefault], globals: { pointer: "touch" } },
    parameters: {},
    when: [
        {
            step: { action: "click", target: { role: "button", name: "Archive" } },
            then: [{ observe: { kind: "count", target: toast }, is: 1 }],
        },
        { step: { action: "pointerMove", target: toast, position: { x: 40, y: 20 } } },
        { step: { action: "pointerDown" } },
        {
            step: { action: "pointerMove", target: toast, position: { x: 240, y: 20 } },
            then: [{ observe: { kind: "attribute", target: toast, name: "data-swiping" }, is: "" }],
        },
        {
            step: { action: "pointerUp" },
            then: [{ observe: { kind: "count", target: toast }, is: 0 }],
        },
    ],
});

/** Undo a removal from a toast while the dialog that removed it stays open. */
export const toasterUndoInDialog = defineScenario({
    of: Toaster,
    interaction: viewInteraction,
    name: "undoInDialog",
    description: t`Remove a member in an open dialog, then undo it from the toast above the dialog.`,
    given: { examples: [toasterInDialog] },
    parameters: {},
    when: [
        {
            step: { action: "click", target: { role: "button", name: "Members" } },
            then: [{ observe: { kind: "count", target: { css: "[data-slot=member]" } }, is: 3 }],
        },
        {
            step: { action: "click", target: { css: "[data-slot=member]:first-child button" } },
            then: [
                { observe: { kind: "count", target: { css: "[data-slot=member]" } }, is: 2 },
                { observe: { kind: "count", target: toast }, is: 1 },
            ],
        },
        {
            step: { action: "click", target: { role: "button", name: "Undo" } },
            then: [
                { observe: { kind: "count", target: { css: "[data-slot=member]" } }, is: 3 },
                {
                    observe: { kind: "visible", target: { role: "dialog", name: "Members" } },
                    is: true,
                },
            ],
        },
    ],
});
