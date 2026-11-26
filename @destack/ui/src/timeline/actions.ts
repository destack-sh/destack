import { t } from "@destack/locale";
import { defineAction } from "@destack/object";
import { items } from "../action/action.ts";

/** Start a link from the end of the row's item. */
export const linkItem = defineAction({
    name: "item.link",
    title: t.context("action")`Link`,
    category: t.context("title")`Timeline`,
    keybinding: "l",
    input: { items },
});

/** Show more of the window to the left. */
export const panLeft = defineAction({
    name: "view.panLeft",
    title: t.context("action")`Pan left`,
    category: t.context("title")`View`,
    keybinding: "alt+left",
});

/** Show more of the window to the right. */
export const panRight = defineAction({
    name: "view.panRight",
    title: t.context("action")`Pan right`,
    category: t.context("title")`View`,
    keybinding: "alt+right",
});
