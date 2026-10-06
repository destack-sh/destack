import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { menubarEditorMenus } from "./menubar.example.tsx";
import { Menubar } from "./menubar.tsx";

/** Move along a menubar and across its open menus. */
export const menubarMoveAcrossMenus = defineScenario({
    of: Menubar,
    interaction: viewInteraction,
    name: "move-across-menus",
    description: "move between a menubar's triggers and cross from one open menu to the next",
    given: { examples: [menubarEditorMenus] },
    when: [
        { action: "focus", target: { role: "menuitem", name: "File" } },
        { action: "press", key: "ArrowRight" },
        { action: "press", key: "ArrowRight" },
        { action: "press", key: "ArrowDown" },
        { action: "press", key: "ArrowRight" },
        { action: "press", key: "Escape" },
    ],
    then: {
        observe: {
            focused: { kind: "focused" },
            tabStop: { kind: "name", target: { css: "[data-slot=menubar-trigger][tabindex='0']" } },
        },
        each: [
            { focused: "File", tabStop: "File" },
            { focused: "Edit", tabStop: "Edit" },
            { focused: "File", tabStop: "File" },
            { focused: "New note ⌘N", tabStop: "File" },
            { focused: "Undo ⌘Z", tabStop: "Edit" },
            { focused: "Edit", tabStop: "Edit" },
        ],
    },
});
