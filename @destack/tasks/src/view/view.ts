import { t } from "@destack/locale";
import { defineView } from "@destack/view/declare";
import { project, task } from "../object/index.ts";

/** The open tasks, searchable, with a quick create: the tracker's start. */
export const tasks = defineView({
    name: "tasks",
    title: t`Tasks`,
    categories: ["main"],
    objects: [task, project],
    component: () => import("./tasks.tsx"),
});

/** The tasks as a board, one column per state. */
export const board = defineView({
    name: "board",
    title: t`Board`,
    objects: [task],
    component: () => import("./board.tsx"),
});

/** The space's projects, each with its progress. */
export const projects = defineView({
    name: "projects",
    title: t`Projects`,
    objects: [project, task],
    component: () => import("./projects.tsx"),
});
