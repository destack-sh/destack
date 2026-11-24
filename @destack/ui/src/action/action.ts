import { t } from "@destack/locale";
import { defineAction } from "@destack/object";

/** Select everything the list shows. */
export const selectAll = defineAction({
    name: "item.selectAll",
    title: t`Select all`,
    category: t`Edit`,
    keybinding: "mod+a",
});

/** Clear the selection of the list. */
export const clearSelection = defineAction({
    name: "item.clearSelection",
    title: t`Clear selection`,
    category: t`Edit`,
    keybinding: "escape",
});

/** Select the item, or take it out of the selection. */
export const toggleSelection = defineAction({
    name: "item.toggleSelection",
    title: t`Select`,
    category: t`Edit`,
    keybinding: "x",
});

/** Open the item. */
export const openItem = defineAction({
    name: "item.open",
    title: t`Open`,
    category: t`Edit`,
    keybinding: "enter",
});

/** Show the item after the one shown. */
export const nextItem = defineAction({
    name: "item.next",
    title: t`Next`,
    category: t`Navigation`,
    keybinding: "j",
});

/** Show the item before the one shown. */
export const previousItem = defineAction({
    name: "item.previous",
    title: t`Previous`,
    category: t`Navigation`,
    keybinding: "k",
});

/** Save the item as a file. */
export const downloadItem = defineAction({
    name: "item.download",
    title: t`Download`,
    category: t`File`,
});

/** Send the item again after it failed. */
export const retryItem = defineAction({
    name: "item.retry",
    title: t`Try again`,
    category: t`File`,
});

/** Choose files to add where the person is. */
export const attachFiles = defineAction({
    name: "file.attach",
    title: t`Add files`,
    category: t`File`,
});

/** Show what the person sees closer. */
export const zoomIn = defineAction({
    name: "view.zoomIn",
    title: t`Zoom in`,
    category: t`View`,
    keybinding: ["plus", "="],
});

/** Show more of what the person sees. */
export const zoomOut = defineAction({
    name: "view.zoomOut",
    title: t`Zoom out`,
    category: t`View`,
    keybinding: "-",
});

/** Show what the person sees closer, or all of it again. */
export const toggleZoom = defineAction({
    name: "view.toggleZoom",
    title: t`Zoom`,
    category: t`View`,
    keybinding: "z",
});

/** Show all of what the person sees. */
export const zoomToFit = defineAction({
    name: "view.zoomToFit",
    title: t`Zoom to fit`,
    category: t`View`,
    keybinding: "0",
});

/** Add an item where the person is. */
export const createItem = defineAction({
    name: "item.create",
    title: t`Add`,
    category: t`Edit`,
    keybinding: "c",
});

/** Add a copy of the selected items. */
export const duplicateItems = defineAction({
    name: "item.duplicate",
    title: t`Duplicate`,
    category: t`Edit`,
    keybinding: "mod+d",
});

/** Remove the selected items. */
export const removeItems = defineAction({
    name: "item.remove",
    title: t`Delete`,
    category: t`Edit`,
    keybinding: { key: "delete", mac: ["backspace", "delete"] },
    isDestructive: true,
});

/** Open the menu of what the person is on, as the menu key does. */
export const openMenu = defineAction({
    name: "item.openMenu",
    title: t`Open menu`,
    category: t`Edit`,
    keybinding: "shift+f10",
});

/** Hide the column. */
export const hideColumn = defineAction({
    name: "column.hide",
    title: t`Hide`,
    category: t`View`,
});
