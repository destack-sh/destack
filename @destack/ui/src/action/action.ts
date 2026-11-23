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
    keybinding: "delete",
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
