import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { toasterArchiveUndo } from "./toast.example.tsx";
import { Toaster } from "./toast.tsx";

/** The one toast shown. */
const toast = { css: "[data-slot=toast]" };

/** Swipe a toast away with a finger. */
export const toasterSwipeAway = defineScenario({
    of: Toaster,
    interaction: viewInteraction,
    name: "swipe-away",
    description: "show a toast, swipe it toward its edge with a finger and see it dismissed",
    given: { examples: [toasterArchiveUndo], globals: { pointer: "touch" } },
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
