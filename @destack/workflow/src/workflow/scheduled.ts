import { type JsonValue, schema } from "@destack/schema";
import { Call } from "@destack/sync";
import type { Step } from "../step/step.ts";
import type { RunStart, Workflow } from "./workflow.ts";

/** The input of a scheduled call's run: the call it makes. */
const ScheduledCallInput = schema.object({
    /** The object method call the run makes. */
    call: Call,
});
/** The input of a scheduled call's run. */
export type ScheduledCallInput = schema.Infer<typeof ScheduledCallInput>;

/** A call a workflow's run makes once, now or at a time, as the run's actor. */
export const ScheduledCall = {
    /** The input a workflow of scheduled calls declares. */
    input: ScheduledCallInput,

    /** The output a workflow of scheduled calls declares: the call's result. */
    output: schema.json(),

    /** Build the call starting a run that makes a call once, later or now, as its actor. */
    send(
        workflow: Pick<Workflow<ScheduledCallInput>, "start">,
        call: Call,
        options: Pick<RunStart, "at" | "key" | "actor">,
    ): Call {
        return workflow.start({ call }, options);
    },

    /** Make a run's call as its one step, answering the call's result. */
    async run(step: Step, input: ScheduledCallInput): Promise<JsonValue> {
        return schema.json().parse(await step.call("call", input.call));
    },
};
