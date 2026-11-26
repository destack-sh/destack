import { t } from "@destack/locale";
import { defineAction } from "@destack/object";

/** Add a condition to the filter. */
export const addFilter = defineAction({
    name: "filter.add",
    title: t.context("action")`Filter`,
    category: t.context("title")`Filter`,
    keybinding: "f",
});

/** Remove every condition from the filter. */
export const clearFilter = defineAction({
    name: "filter.clear",
    title: t.context("action")`Clear filters`,
    category: t.context("title")`Filter`,
    keybinding: "shift+f",
});
