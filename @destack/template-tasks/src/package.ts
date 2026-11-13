import { definePackage } from "@destack/object";
import { cycle, task, taskLabel, taskRelation, label, project } from "./object/index.ts";
import { board, tasks } from "./view/index.ts";

/** The handle stacks import to install the tracker. */
export default definePackage({
    objects: [project, cycle, label, task, taskLabel, taskRelation],
    views: [tasks, board],
});
