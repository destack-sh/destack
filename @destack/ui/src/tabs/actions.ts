import { t } from "@destack/locale";
import { defineAction } from "@destack/object";
import { schema } from "@destack/schema";
import { items } from "../action/action.ts";

/** Close a tab. */
export const closeTab = defineAction({
    name: "tab.close",
    title: t.context("action")`Close`,
    category: t.context("title")`Tabs`,
    keybinding: "mod+w",
    input: { items: items.exactOptional() },
});

/** Close every tab but one. */
export const closeOtherTabs = defineAction({
    name: "tab.closeOthers",
    title: t.context("action")`Close others`,
    category: t.context("title")`Tabs`,
    input: { items: items.exactOptional() },
});

/** Close every tab after one. */
export const closeTabsAfter = defineAction({
    name: "tab.closeAfter",
    title: t`Close those after`,
    category: t.context("title")`Tabs`,
    input: { items: items.exactOptional() },
});

/** Close every tab. */
export const closeAllTabs = defineAction({
    name: "tab.closeAll",
    title: t.context("action")`Close all`,
    category: t.context("title")`Tabs`,
    input: { items: items.exactOptional() },
});

/** Open the tab closed last again. */
export const reopenTab = defineAction({
    name: "tab.reopen",
    title: t`Reopen closed tab`,
    category: t.context("title")`Tabs`,
    keybinding: "mod+shift+t",
});

/** Select the next tab. */
export const nextTab = defineAction({
    name: "tab.next",
    title: t.context("action")`Next tab`,
    category: t.context("title")`Tabs`,
    keybinding: "mod+alt+right",
});

/** Select the previous tab. */
export const previousTab = defineAction({
    name: "tab.previous",
    title: t.context("action")`Previous tab`,
    category: t.context("title")`Tabs`,
    keybinding: "mod+alt+left",
});

/** Select a tab by its place. */
export const goToTab = defineAction({
    name: "tab.goTo",
    title: t`Go to tab…`,
    category: t.context("title")`Tabs`,
    input: { tab: schema.string() },
});

/** Move a tab one place left. */
export const moveTabLeft = defineAction({
    name: "tab.moveLeft",
    title: t.context("action")`Move left`,
    category: t.context("title")`Tabs`,
    keybinding: "shift+alt+left",
    input: { items: items.exactOptional() },
});

/** Move a tab one place right. */
export const moveTabRight = defineAction({
    name: "tab.moveRight",
    title: t.context("action")`Move right`,
    category: t.context("title")`Tabs`,
    keybinding: "shift+alt+right",
    input: { items: items.exactOptional() },
});
