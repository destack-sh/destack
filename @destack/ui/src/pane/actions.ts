import { t } from "@destack/locale";
import { defineAction, defineVariable } from "@destack/object";
import { schema } from "@destack/schema";
import { items } from "../action/action.ts";

/** The pane whose bar a person is on, by its id. */
export const currentPane = defineVariable({ name: "pane", value: schema.string() });

/** Open the selected tab again in a pane on the right. */
export const splitRight = defineAction({
    name: "pane.splitRight",
    title: t`Split right`,
    category: t`Panes`,
    keybinding: "mod+\\",
    input: { items: items.exactOptional() },
});

/** Open the selected tab again in a pane below. */
export const splitDown = defineAction({
    name: "pane.splitDown",
    title: t`Split down`,
    category: t`Panes`,
    keybinding: "mod+shift+\\",
    input: { items: items.exactOptional() },
});

/** Move a tab to another pane, or to one of its own. */
export const moveTabTo = defineAction({
    name: "pane.moveTabTo",
    title: t`Move to…`,
    category: t`Panes`,
    input: { items: items.exactOptional(), place: schema.string() },
});

/** Float the selected tab over the panes. */
export const floatTab = defineAction({
    name: "pane.floatTab",
    title: t`Float`,
    category: t`Panes`,
    input: { items: items.exactOptional() },
});

/** Move the selected tab to a window of its own. */
export const openWindow = defineAction({
    name: "pane.openWindow",
    title: t`Move to a new window`,
    category: t`Panes`,
    input: { items: items.exactOptional() },
});

/** Put a floating pane beside the others. */
export const dockPane = defineAction({
    name: "pane.dockPane",
    title: t`Dock`,
    category: t`Panes`,
    input: { pane: currentPane.exactOptional() },
});

/** Go to the pane on the left. */
export const focusPaneLeft = defineAction({
    name: "pane.focusLeft",
    title: t`Go to the pane on the left`,
    category: t`Panes`,
    keybinding: "mod+alt+shift+left",
});

/** Go to the pane on the right. */
export const focusPaneRight = defineAction({
    name: "pane.focusRight",
    title: t`Go to the pane on the right`,
    category: t`Panes`,
    keybinding: "mod+alt+shift+right",
});

/** Go to the pane above. */
export const focusPaneUp = defineAction({
    name: "pane.focusUp",
    title: t`Go to the pane above`,
    category: t`Panes`,
    keybinding: "mod+alt+shift+up",
});

/** Go to the pane below. */
export const focusPaneDown = defineAction({
    name: "pane.focusDown",
    title: t`Go to the pane below`,
    category: t`Panes`,
    keybinding: "mod+alt+shift+down",
});

/** Cover the other panes with one, or uncover them. */
export const maximizePane = defineAction({
    name: "pane.maximize",
    title: t`Maximize the pane`,
    category: t`Panes`,
    keybinding: "mod+shift+enter",
    input: { pane: currentPane.exactOptional() },
});

/** Close a pane with every tab in it. */
export const closePane = defineAction({
    name: "pane.close",
    title: t`Close the pane`,
    category: t`Panes`,
    input: { pane: currentPane.exactOptional() },
});

/** Give the panes even sizes. */
export const evenPanes = defineAction({
    name: "pane.even",
    title: t`Even out the panes`,
    category: t`Panes`,
    input: { pane: currentPane.exactOptional() },
});
