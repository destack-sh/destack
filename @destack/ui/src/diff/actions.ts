import { t } from "@destack/locale";
import { defineAction } from "@destack/object";

/** Go to the next change of a diff. */
export const nextChange = defineAction({
    name: "change.next",
    title: t.context("action")`Next change`,
    category: t.context("title")`Navigation`,
    keybinding: "j",
});

/** Go to the previous change of a diff. */
export const previousChange = defineAction({
    name: "change.previous",
    title: t.context("action")`Previous change`,
    category: t.context("title")`Navigation`,
    keybinding: "k",
});

/** Accept a change of a diff. */
export const acceptChange = defineAction({
    name: "change.accept",
    title: t.context("action")`Accept`,
    category: t.context("title")`Edit`,
    keybinding: "y",
});

/** Reject a change of a diff. */
export const rejectChange = defineAction({
    name: "change.reject",
    title: t.context("action")`Reject`,
    category: t.context("title")`Edit`,
    keybinding: "n",
});

/** Comment on the selected lines of a diff, or on the change a person is at. */
export const commentChange = defineAction({
    name: "change.comment",
    title: t.context("action")`Comment`,
    category: t.context("title")`Edit`,
    keybinding: "c",
});
