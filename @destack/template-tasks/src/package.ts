import { t } from "@destack/locale";
import { definePackage } from "@destack/object";
import { cycle, task, taskLabel, taskRelation, label, project } from "./object/index.ts";
import { board, tasks } from "./view/index.ts";

/** The handle stacks import to install the tracker. */
export default definePackage({
    title: t`Tasks`,
    description: t`Track a team's tasks in projects and cycles, on a board.`,
    icon: "kanban",
    color: "orange",
    objects: [project, cycle, label, task, taskLabel, taskRelation],
    views: [tasks, board],
});
