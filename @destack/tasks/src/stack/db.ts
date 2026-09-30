import type { Table } from "@destack/db";
import { defineDatabase } from "@destack/db/declare";
import { announcement, delivery, notification, subscription } from "@destack/notification";
import { defineJournal } from "@destack/service/database";
import { comment, reaction } from "@destack/social";
import { project, task } from "../object/index.ts";

/** Replayable method requests. */
export const tasksJournal = defineJournal("journal");

/** The tables of projects and tasks, with their comments and notifications, for a database embedding them. */
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
    tasksJournal,
];

/** The database of one space's projects and tasks, with their comments and notifications. */
export const tasksDatabase = defineDatabase({ name: "main", tables: tasksTables });
