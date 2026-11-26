import { t } from "@destack/locale";
import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { drawerShareNoteOpen } from "./drawer.example.tsx";
import { Drawer } from "./drawer.tsx";

/** The open drawer's content. */
const content = { css: "[data-slot=drawer-content]" };

/** Swipe a drawer closed with a finger. */
export const drawerSwipeClosed = defineScenario({
    of: Drawer,
    interaction: viewInteraction,
    name: "swipeClosed",
    description: t`Swipe an open drawer toward its edge with a finger and see it closed.`,
    given: { examples: [drawerShareNoteOpen], globals: { pointer: "touch" } },
    parameters: {},
    when: [
        {
            step: { action: "pointerMove", target: content, position: { x: 200, y: 16 } },
            then: [{ observe: { kind: "visible", target: content }, is: true }],
        },
        { step: { action: "pointerDown" } },
        {
            step: { action: "pointerMove", target: content, position: { x: 200, y: 140 } },
            then: [
                { observe: { kind: "attribute", target: content, name: "data-swiping" }, is: "" },
            ],
        },
        {
            step: { action: "pointerUp" },
            then: [{ observe: { kind: "visible", target: content }, is: false }],
        },
    ],
});
