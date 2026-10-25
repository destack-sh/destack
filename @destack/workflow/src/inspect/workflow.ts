import type { ObjectType } from "@destack/object";
import { DeclarationReference } from "@destack/package/declare";
import { Duration, JsonValue, toJsonSchema } from "@destack/schema";
import { WorkflowDescription } from "../workflow/description.ts";
import { NO_OUTPUT, type WorkflowDeclaration } from "../workflow/workflow.ts";
import { describeTrigger } from "./trigger.ts";

/** Read a workflow's run type, which the build describes as one of the package's objects. */
export function workflowRun(workflow: WorkflowDeclaration): ObjectType {
    return workflow.run;
}

/** Read a workflow's step type, which the build describes as one of the package's objects. */
export function workflowStep(workflow: WorkflowDeclaration): ObjectType {
    return workflow.step;
}

/** Read a workflow's trigger type, which the build describes as one of the package's objects. */
export function workflowTrigger(workflow: WorkflowDeclaration): ObjectType {
    return workflow.trigger;
}

/** Describe a workflow: its schemas, waits, retries, timeout, expiry and limits. */
export function describeWorkflow<Input, Output>(
    workflow: WorkflowDeclaration<Input, Output>,
): WorkflowDescription {
    // describe the input and output schemas and the types runs wait for
    const { limits, timeout, expiring } = workflow;

    return WorkflowDescription.parse({
        name: workflow.name,
        input: JsonValue.object(toJsonSchema(workflow.input)),
        ...(Object.is(workflow.output, NO_OUTPUT)
            ? {}
            : { output: JsonValue.object(toJsonSchema(workflow.output)) }),
        waits: workflow.waits.map((type) => DeclarationReference.of(type)),
        retries: workflow.retries,
        ...(timeout === undefined ? {} : { timeout: Duration.milliseconds(timeout) }),
        ...(expiring === undefined ? {} : { expiring: Duration.milliseconds(expiring) }),
        limits: {
            ...(limits.rate === undefined
                ? {}
                : {
                      rate: {
                          count: limits.rate.count,
                          every: Duration.milliseconds(limits.rate.every),
                          burst: limits.rate.burst ?? limits.rate.count,
                      },
                  }),
            ...(limits.concurrency === undefined ? {} : { concurrency: limits.concurrency }),
            key: limits.key ?? "scope",
        },
        isSingleton: workflow.isSingleton,
        triggers: workflow.triggers.map((trigger) => describeTrigger(trigger)),
    });
}
