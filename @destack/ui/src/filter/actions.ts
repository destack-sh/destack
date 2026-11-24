import { t } from "@destack/locale";
import { defineAction } from "@destack/object";

/** Add a condition to the filter. */
export const addFilter = defineAction({
    name: "filter.add",
    title: t`Filter`,
    category: t`Filter`,
    keybinding: "f",
});

/** Remove every condition from the filter. */
export const clearFilter = defineAction({
    name: "filter.clear",
    title: t`Clear filters`,
    category: t`Filter`,
    keybinding: "shift+f",
});
