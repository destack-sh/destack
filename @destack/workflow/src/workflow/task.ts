import { type JsonValue, schema } from "@destack/schema";
import { Call } from "@destack/sync";
import type { Step } from "../step/step.ts";

/** The input of a task's run: the call it makes. */
const TaskInput = schema.object({
    /** The object method call the run makes. */
    call: Call,
});
/** The input of a task's run. */
export type TaskInput = schema.Infer<typeof TaskInput>;

/** A call a package's task workflow makes once, now or at a time, as Cloud Tasks' tasks. */
export const Task = {
    /** The input a task workflow declares. */
    input: TaskInput,

    /** The output a task workflow declares: the call's result. */
    output: schema.json(),

    /** Make a run's call as its one step, as its actor, answering the call's result. */
    async run(step: Step, input: TaskInput): Promise<JsonValue> {
        return schema.json().parse(await step.call("call", input.call));
    },
};
