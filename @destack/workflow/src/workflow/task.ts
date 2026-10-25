import { type JsonValue, schema } from "@destack/schema";
import { Call } from "@destack/sync";
import type { Step } from "../step/step.ts";
import type { WaitOf } from "./workflow.ts";

/** The input of a task's run: the call it makes. */
export const TaskInput = schema.object({
    /** The object method call the run makes. */
    call: Call,
});
/** The input of a task's run. */
export type TaskInput = schema.Infer<typeof TaskInput>;

/** The output of a task's run: the call's result. */
export const TaskOutput = schema.json();

/** A call a package's task workflow makes once, now or at a time, as Cloud Tasks' tasks. */
export class Task {
    /** The run's steps. */
    readonly #step: Step<WaitOf<never>>;
    /** The run's input: the call it makes. */
    readonly #input: TaskInput;

    /** Run a task in a run's steps. */
    constructor(step: Step<WaitOf<never>>, input: TaskInput) {
        this.#step = step;
        this.#input = input;
    }

    /** Make the run's call as its one step, as its actor, answering the call's result. */
    async run(): Promise<JsonValue> {
        return TaskOutput.parse(await this.#step.call("call", this.#input.call));
    }
}
