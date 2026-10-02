import { type Table, defineDatabase } from "@destack/db";
import { journal } from "@destack/audit";
import { announcement, delivery, notification, subscription } from "@destack/notification";
import { comment, reaction } from "@destack/social";
import { branchTables } from "@destack/space/object";
import { project, task } from "../object/index.ts";

/** The tables of projects and tasks, with their comments, notifications and branches, for a database embedding them. */
export const tasksTables: readonly Table[] = [
    ...[
        project,
        task,
        comment,
        reaction,
        subscription,
        notification,
        announcement,
        delivery,
    ].flatMap((object) => object.tables),
    ...branchTables,
    journal,
];

/** The database of one space's projects and tasks, with their comments and notifications. */
export const tasksDatabase = defineDatabase({ name: "main", tables: tasksTables });
