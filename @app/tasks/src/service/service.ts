import { activity, announcement, subscription } from "@destack/notification";
import { defineService } from "@destack/service";
import { comment, reaction } from "@destack/social";
import { branchObjects } from "@destack/space/object";
import { project, task } from "../object/index.ts";

/** Projects and tasks, with their comments, activities, sharing, sync and branches. */
export const tasksService = defineService("tasks", {
    objects: {
        project,
        task,
        comment,
        reaction,
        subscription,
        activity,
        announcement,
        ...branchObjects,
    },
});
