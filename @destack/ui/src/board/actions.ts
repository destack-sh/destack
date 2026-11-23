import { t } from "@destack/locale";
import { defineAction } from "@destack/object";
import { schema } from "@destack/schema";

/** Move the cards a column to the left. */
export const moveCardLeft = defineAction({
    name: "card.moveLeft",
    title: t`Move left`,
    category: t`Board`,
    keybinding: "shift+alt+left",
});

/** Move the cards a column to the right. */
export const moveCardRight = defineAction({
    name: "card.moveRight",
    title: t`Move right`,
    category: t`Board`,
    keybinding: "shift+alt+right",
});

/** Move the cards a lane up. */
export const moveCardUp = defineAction({
    name: "card.moveUp",
    title: t`Move up`,
    category: t`Board`,
    keybinding: "shift+alt+up",
});

/** Move the cards a lane down. */
export const moveCardDown = defineAction({
    name: "card.moveDown",
    title: t`Move down`,
    category: t`Board`,
    keybinding: "shift+alt+down",
});

/** Move the cards to a column. */
export const moveCardTo = defineAction({
    name: "card.moveTo",
    title: t`Move to…`,
    category: t`Board`,
    input: { column: schema.string() },
});
