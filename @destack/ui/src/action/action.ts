import { t } from "@destack/locale";
import { defineAction, defineVariable } from "@destack/object";
import { schema } from "@destack/schema";

/** The items a person is on in a list: the selection the one they are at is in, else that one. */
export const items = defineVariable({ name: "items", value: schema.array(schema.string()) });

/** Select everything the list shows. */
export const selectAll = defineAction({
    name: "item.selectAll",
    title: t.context("action")`Select all`,
    category: t.context("title")`Edit`,
    keybinding: "mod+a",
});

/** Clear the selection of the list. */
export const clearSelection = defineAction({
    name: "item.clearSelection",
    title: t.context("action")`Clear selection`,
    category: t.context("title")`Edit`,
    keybinding: "escape",
});

/** Select the item, or take it out of the selection. */
export const toggleSelection = defineAction({
    name: "item.toggleSelection",
    title: t.context("action")`Select`,
    category: t.context("title")`Edit`,
    keybinding: "x",
});

/** Open the item. */
export const openItem = defineAction({
    name: "item.open",
    title: t.context("action")`Open`,
    category: t.context("title")`Edit`,
    keybinding: "enter",
    input: { items },
});

/** Show the item after the one shown. */
export const nextItem = defineAction({
    name: "item.next",
    title: t.context("action")`Next`,
    category: t.context("title")`Navigation`,
    keybinding: "j",
});

/** Show the item before the one shown. */
export const previousItem = defineAction({
    name: "item.previous",
    title: t.context("action")`Previous`,
    category: t.context("title")`Navigation`,
    keybinding: "k",
});

/** Move the items to a section of their list, such as a board's column. */
export const moveItemsTo = defineAction({
    name: "item.moveTo",
    title: t.context("action")`Move to…`,
    category: t.context("title")`Edit`,
    input: { items, section: schema.string() },
});

/** Move the items to the section on the left. */
export const moveItemsLeft = defineAction({
    name: "item.moveLeft",
    title: t.context("action")`Move left`,
    category: t.context("title")`Edit`,
    keybinding: "shift+alt+left",
    input: { items },
});

/** Move the items to the section on the right. */
export const moveItemsRight = defineAction({
    name: "item.moveRight",
    title: t.context("action")`Move right`,
    category: t.context("title")`Edit`,
    keybinding: "shift+alt+right",
    input: { items },
});

/** Move the items up by one. */
export const moveItemsUp = defineAction({
    name: "item.moveUp",
    title: t.context("action")`Move up`,
    category: t.context("title")`Edit`,
    keybinding: "shift+alt+up",
    input: { items },
});

/** Move the items down by one. */
export const moveItemsDown = defineAction({
    name: "item.moveDown",
    title: t.context("action")`Move down`,
    category: t.context("title")`Edit`,
    keybinding: "shift+alt+down",
    input: { items },
});

/** Save the item as a file. */
export const downloadItem = defineAction({
    name: "item.download",
    title: t.context("action")`Download`,
    category: t.context("title")`File`,
    input: { items: items.exactOptional() },
});

/** Send the item again after it failed. */
export const retryItem = defineAction({
    name: "item.retry",
    title: t.context("action")`Try again`,
    category: t.context("title")`File`,
    input: { items },
});

/** Choose files to add where the person is. */
export const attachFiles = defineAction({
    name: "file.attach",
    title: t.context("action")`Add files`,
    category: t.context("title")`File`,
});

/** Show what the person sees closer. */
export const zoomIn = defineAction({
    name: "view.zoomIn",
    title: t.context("action")`Zoom in`,
    category: t.context("title")`View`,
    keybinding: ["plus", "="],
});

/** Show more of what the person sees. */
export const zoomOut = defineAction({
    name: "view.zoomOut",
    title: t.context("action")`Zoom out`,
    category: t.context("title")`View`,
    keybinding: "-",
});

/** Show what the person sees closer, or all of it again. */
export const toggleZoom = defineAction({
    name: "view.toggleZoom",
    title: t.context("action")`Zoom`,
    category: t.context("title")`View`,
    keybinding: "z",
});

/** Show all of what the person sees. */
export const zoomToFit = defineAction({
    name: "view.zoomToFit",
    title: t`Zoom to fit`,
    category: t.context("title")`View`,
    keybinding: "0",
});

/** Add an item where the person is. */
export const createItem = defineAction({
    name: "item.create",
    title: t.context("action")`Add`,
    category: t.context("title")`Edit`,
    keybinding: "c",
});

/** Add a copy of the selected items. */
export const duplicateItems = defineAction({
    name: "item.duplicate",
    title: t.context("action")`Duplicate`,
    category: t.context("title")`Edit`,
    keybinding: "mod+d",
    input: { items },
});

/** Remove the selected items. */
export const removeItems = defineAction({
    name: "item.remove",
    title: t.context("action")`Delete`,
    category: t.context("title")`Edit`,
    keybinding: { key: "delete", mac: ["backspace", "delete"] },
    isDestructive: true,
    input: { items },
});

/** Open the menu of what the person is on, as the menu key does. */
export const openMenu = defineAction({
    name: "item.openMenu",
    title: t.context("action")`Open menu`,
    category: t.context("title")`Edit`,
    keybinding: "shift+f10",
});

/** Hide the column. */
export const hideColumn = defineAction({
    name: "column.hide",
    title: t.context("action")`Hide`,
    category: t.context("title")`View`,
});

/** Show what the keys do where the person is. */
export const showShortcuts = defineAction({
    name: "shortcuts.show",
    title: t.context("action")`Keyboard shortcuts`,
    category: t.context("title")`Help`,
    keybinding: "mod+/",
    focus: "any",
});
