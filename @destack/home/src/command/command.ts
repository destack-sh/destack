import { defineView } from "@destack/view/declare";

/** The command palette over the commands and views of the person's installations. */
export const command = defineView({
    name: "command",
    categories: ["command"],
    component: () => import("./palette.tsx"),
});
