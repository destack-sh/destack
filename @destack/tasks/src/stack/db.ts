import { defineDatabase } from "@destack/db/declare";
import { defineJournal } from "@destack/service/database";
import { comment, project, task } from "../object/index.ts";

/** Replayable method requests. */
export const tasksJournal = defineJournal("journal");

/** The database of one space's projects, tasks and comments. */
export const tasksDatabase = defineDatabase({
    name: "main",
    tables: [...project.tables, ...task.tables, ...comment.tables, tasksJournal],
});
