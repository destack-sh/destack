import { t } from "@destack/locale";
import { definePackage } from "@destack/object";
import { setting } from "@destack/setting/object";
import { user } from "@destack/account/object";
import { cycle, task, taskLabel, taskRelation, label, project } from "./object/index.ts";
import { board, projects, tasks } from "./view/index.ts";

/** The handle stacks import to install the tracker. */
export default definePackage({
    title: t`Tasks`,
    description: t`Track a team's tasks in projects and cycles, as a list and a board.`,
    icon: "kanban",
    color: "orange",
    access: {
        permissions: {
            user: {
                // read and write the space's tasks and projects for the person
                space: [
                    task.permission("read"),
                    task.permission("write"),
                    project.permission("read"),
                    project.permission("write"),
                ],
            },
        },
    },
    objects: [project, cycle, label, task, taskLabel, taskRelation],
    views: [tasks, board, projects],
});
