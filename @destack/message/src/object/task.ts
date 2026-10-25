import { defineWorkflow, TaskInput, TaskOutput } from "@destack/workflow";

/** The calls messages make once as their space, such as forgetting a gone browser. */
export const task = defineWorkflow({ name: "task", input: TaskInput, output: TaskOutput });
