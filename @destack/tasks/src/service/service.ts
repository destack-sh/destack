import { announcement, delivery, notification, subscription } from "@destack/notification";
import { defineService } from "@destack/service";
import { comment, reaction } from "@destack/social";
import { project, task } from "../object/index.ts";

/** Projects and tasks, with their comments, notifications, sharing and sync. */
export const tasksService = defineService("tasks", {
    objects: {
        project,
        task,
        comment,
        reaction,
        subscription,
        notification,
        announcement,
        delivery,
    },
});
