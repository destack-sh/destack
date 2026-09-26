import { defineService } from "@destack/service";
import { comment, project, task } from "../object/index.ts";

/** Projects, tasks and comments, with their sharing and sync. */
export const tasksService = defineService("tasks", {
    objects: { project, task, comment },
});
