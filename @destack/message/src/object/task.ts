import { defineWorkflow, TaskInput, TaskOutput } from "@destack/workflow";
import { t } from "@destack/locale";

/** The calls messages make once as their space, such as forgetting a gone browser. */
export const task = defineWorkflow({
    name: "task",
    title: t`Task`,
    input: TaskInput,
    output: TaskOutput,
});
