import { t } from "@destack/locale";
import { defineAction } from "@destack/object";

/** Move the row up among its siblings. */
export const moveRowUp = defineAction({
    name: "row.moveUp",
    title: t`Move up`,
    category: t`Timeline`,
    keybinding: "mod+shift+up",
});

/** Move the row down among its siblings. */
export const moveRowDown = defineAction({
    name: "row.moveDown",
    title: t`Move down`,
    category: t`Timeline`,
    keybinding: "mod+shift+down",
});

/** Start a link from the end of the row's item. */
export const linkItem = defineAction({
    name: "item.link",
    title: t`Link`,
    category: t`Timeline`,
    keybinding: "l",
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
