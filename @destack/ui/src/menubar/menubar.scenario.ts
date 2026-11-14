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
    parameters: {},
    when: [
        {
            step: { action: "focus", target: { role: "menuitem", name: "File" } },
            then: [
                { observe: { kind: "focused" }, is: "File" },
                {
                    observe: {
                        kind: "name",
                        target: { css: "[data-slot=menubar-trigger][tabindex='0']" },
                    },
                    is: "File",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [
                { observe: { kind: "focused" }, is: "Edit" },
                {
                    observe: {
                        kind: "name",
                        target: { css: "[data-slot=menubar-trigger][tabindex='0']" },
                    },
                    is: "Edit",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [
                { observe: { kind: "focused" }, is: "File" },
                {
                    observe: {
                        kind: "name",
                        target: { css: "[data-slot=menubar-trigger][tabindex='0']" },
                    },
                    is: "File",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowDown" },
            then: [
                { observe: { kind: "focused" }, is: "New note ⌘N" },
                {
                    observe: {
                        kind: "name",
                        target: { css: "[data-slot=menubar-trigger][tabindex='0']" },
                    },
                    is: "File",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [
                { observe: { kind: "focused" }, is: "Undo ⌘Z" },
                {
                    observe: {
                        kind: "name",
                        target: { css: "[data-slot=menubar-trigger][tabindex='0']" },
                    },
                    is: "Edit",
                },
            ],
        },
        {
            step: { action: "press", key: "Escape" },
            then: [
                { observe: { kind: "focused" }, is: "Edit" },
                {
                    observe: {
                        kind: "name",
                        target: { css: "[data-slot=menubar-trigger][tabindex='0']" },
                    },
                    is: "Edit",
                },
            ],
        },
    ],
});
