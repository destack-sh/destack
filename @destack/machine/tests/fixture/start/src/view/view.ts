import { space } from "@destack/account/object";
import { defineView } from "@destack/view/declare";
import { t } from "@destack/locale";

/** The start page: a heading over the account's spaces. */
export const start = defineView({
    name: "start",
    title: t`Start`,
    categories: ["home"],
    objects: [space],
    component: () => import("./start.tsx"),
});

/** The command palette over the person's installations: a heading alone. */
export const command = defineView({
    name: "command",
    title: t`Command`,
    categories: ["command"],
    component: () => import("./command.tsx"),
});
