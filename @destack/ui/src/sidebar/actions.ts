import { t } from "@destack/locale";
import { defineAction } from "@destack/object";

/** Show the sidebar, or hide it. */
export const toggleSidebar = defineAction({
    name: "sidebar.toggle",
    title: t`Toggle sidebar`,
    category: t`View`,
    keybinding: "mod+b",
});
