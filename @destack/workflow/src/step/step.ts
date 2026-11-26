import { type JsonCondition, type Select } from "@destack/db";
import { ObjectReference, Subject } from "@destack/sync";
import { Duration } from "@destack/schema";
import { type StepType, type Workflow } from "../workflow/workflow.ts";

/** A step's row as its workflow's step type stores it. */
export type StepRecord = Select<StepType["table"]>;

/** A workflow another run invokes: its name, input, runs, how they start, and their output. */
export type InvokedWorkflow<Input, Output> = Pick<
    Workflow<Input, Output>,
    "name" | "input" | "output" | "run" | "start"
>;

/** What happens to an invoked run once its invoking run ends, as Temporal's parent close policy. */
export interface InvokeOptions {
    /** Cancel the invoked run with its parent, by default, or abandon it to run on. */
    readonly onParentClose?: "cancel" | "abandon";
}

/** Who decides on an object, how many approvals approve it, and how long the run waits. */
export interface ApprovalRequest {
    /** The principals asked to decide. */
    readonly approvers: readonly ObjectReference[];
    /** The approvals that approve the object, one by default. */
    readonly count?: number;
    /** How long the run waits for the decision, absent to wait until it comes. */
    readonly timeout?: Duration;
}

/** What a run's function knows of its run. */
export interface RunContext {
    /** The run. */
    readonly id: string;
    /** The scope the run acts in. */
    readonly scope: string;
    /** Whom the run acts for. */
    readonly actor: Subject;
}

/** The objects a waiting run waits for: a type's rows meeting a condition. */
export interface Wait {
    /** The object type, by its reference key. */
    readonly type: string;
    /** The condition one of its rows meets once the run should advance. */
    readonly condition: JsonCondition;
}
