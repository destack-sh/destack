import { t } from "@destack/locale";
import { defineAction } from "@destack/object";
import { items } from "../action/action.ts";

/** Start a link from the end of the row's item. */
export const linkItem = defineAction({
    name: "item.link",
    title: t`Link`,
    category: t`Timeline`,
    keybinding: "l",
    input: { items },
});

/** Show more of the window to the left. */
export const panLeft = defineAction({
    name: "view.panLeft",
    title: t`Pan left`,
    category: t`View`,
    keybinding: "alt+left",
});

/** Show more of the window to the right. */
export const panRight = defineAction({
    name: "view.panRight",
    title: t`Pan right`,
    category: t`View`,
    keybinding: "alt+right",
});
