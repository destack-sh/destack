import { t } from "@destack/locale";
import { defineView } from "@destack/view/declare";
import { task } from "../object/index.ts";

/** The open tasks, searchable, with a quick create: the tracker's start. */
export const tasks = defineView({
    name: "tasks",
    title: t`Tasks`,
    categories: ["main"],
    objects: [task],
    component: () => import("./tasks.tsx"),
});

/** The tasks as a board, one column per state. */
export const board = defineView({
    name: "board",
    title: t`Board`,
    objects: [task],
    component: () => import("./board.tsx"),
});
