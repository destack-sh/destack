import { t } from "@destack/locale";
import { defineAction } from "@destack/object";

/** Go to the next change of a diff. */
export const nextChange = defineAction({
    name: "change.next",
    title: t`Next change`,
    category: t`Navigation`,
    keybinding: "j",
});

/** Go to the previous change of a diff. */
export const previousChange = defineAction({
    name: "change.previous",
    title: t`Previous change`,
    category: t`Navigation`,
    keybinding: "k",
});

/** Accept a change of a diff. */
export const acceptChange = defineAction({
    name: "change.accept",
    title: t`Accept`,
    category: t`Edit`,
    keybinding: "y",
});

/** Reject a change of a diff. */
export const rejectChange = defineAction({
    name: "change.reject",
    title: t`Reject`,
    category: t`Edit`,
    keybinding: "n",
});

/** Comment on the selected lines of a diff, or on the change a person is at. */
export const commentChange = defineAction({
    name: "change.comment",
    title: t`Comment`,
    category: t`Edit`,
    keybinding: "c",
});
