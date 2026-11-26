import { t } from "@destack/locale";
import { defineAction } from "@destack/object";

/** Show the sidebar, or hide it. */
export const toggleSidebar = defineAction({
    name: "sidebar.toggle",
    title: t.context("action")`Toggle sidebar`,
    category: t.context("title")`View`,
    keybinding: "mod+b",
});
