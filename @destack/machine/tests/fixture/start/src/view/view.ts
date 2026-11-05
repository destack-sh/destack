import { space } from "@destack/account/object";
import { defineView } from "@destack/view/declare";

/** The start page: a heading over the account's spaces. */
export const start = defineView({
    name: "start",
    categories: ["home"],
    objects: [space],
    component: () => import("./start.tsx"),
});

/** The command palette over the person's installations: a heading alone. */
export const command = defineView({
    name: "command",
    categories: ["command"],
    component: () => import("./command.tsx"),
});
