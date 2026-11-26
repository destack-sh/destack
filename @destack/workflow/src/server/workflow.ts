import { present, schema } from "@destack/schema";
import { ObjectReference, ObjectTypeReference, RequestId } from "@destack/sync";
import { now, TABLE } from "@destack/db";
import { CallSite, type ObjectReconciliation, type ObjectType } from "@destack/object";
import type { StepRecord } from "../step/step.ts";
import {
    recorded,
    type Workflow,
    type WorkflowCode,
    type WorkflowImplementation,
} from "../workflow/workflow.ts";
import { Run } from "./run.ts";
import { TriggerServer } from "./trigger.ts";
import { WorkflowError } from "../error/index.ts";

/** Implement a workflow with its code: its runs and steps advanced, its triggers fired. */
export function implementWorkflow<Input, Output, Waits extends ObjectType, Name extends string>(
    workflow: Workflow<Input, Output, Waits, Name>,
    code: WorkflowCode<Input, Output, Waits>,
): WorkflowImplementation {
    // read the workflow's types
    const { run, step, waits } = workflow;

    // serve the triggers through the workflow's server
    const triggers = new TriggerServer(workflow);
    const webhooks = triggers.webhooks(code.server);

    // advance each run of a reconciliation through the workflow's code
    const advance = async (reconciliation: ObjectReconciliation<typeof run>) => {
        await Promise.all(
            reconciliation.rows.map((row) =>
                new Run(workflow, row).advance(code.run, reconciliation),
            ),
        );
    };
    const objects = [
        run.implement({
            methods: {
                create: (call, next) => Run.create(workflow, call, next),
                reset: (call) => new Run(workflow, present(call.target, "the run")).reset(call),
                // close the children ending with the run on each transition ending it
                complete: (call, next) =>
                    Run.end(workflow, call, present(call.target, "the run"), "completed", next),
                fail: (call, next) =>
                    Run.end(workflow, call, present(call.target, "the run"), "failed", next),
                cancel: (call, next) =>
                    Run.end(workflow, call, present(call.target, "the run"), "cancelled", next),
            },
            controllers: [
                {
                    // advance a pending run, and a waiting one once a type it waits for changes
                    name: "advance",
                    when: { state: { in: ["pending", "waiting"] } },
                    reads: waits.map((type) => ({
                        table: type.table,
                        fields: Object.keys(type.table[TABLE].logged),
                        matches: {
                            column: "waitCondition",
                            where: {
                                state: "waiting",
                                waitType: ObjectTypeReference.key(type.typeReference),
                            },
                        },
                    })),
                    reconcile: async (reconciliation) => advance(reconciliation),
                },
                {
                    // wake a run once its sleep, retry, limit or deadline comes, which its controller then advances
                    name: "wake",
                    when: {
                        state: { in: ["pending", "waiting"] },
                        wakeAt: { lte: now },
                    },
                    reconcile: async ({ rows, mutate }) => {
                        await mutate(
                            rows.map((row) =>
                                run.call("delay", {
                                    ...run.target(row),
                                    attempts: row.attempts,
                                    wakeAt: null,
                                    failure: row.failure,
                                }),
                            ),
                        );
                    },
                },
            ],
        }),
        step.implement({
            methods: {
                // relate a placed step's runner, which reads and ends it
                create: {
                    relate: (context) => {
                        const columns = schema.looseObject({}).parse(context.input);
                        const isPlaced = typeof columns["runnerId"] === "string";

                        return {
                            relationships: isPlaced
                                ? [
                                      {
                                          relation: "runner",
                                          subject: ObjectReference.at("runner", columns),
                                      },
                                  ]
                                : [],
                        };
                    },
                },
                // forward the runner's call as the run's actor before the transaction
                forward: {
                    prepare: async (context) => {
                        // read the run of an open placed step
                        requirePlaced(context.target);
                        const { store } = context.execution;
                        const parent = present(
                            await context.read(run.table).findFirst({
                                where: { id: context.target.parentId },
                            }),
                            `the run of step '${context.target.name}'`,
                        );

                        // act as the run's caller, within its actor's grant
                        const caller = await new Run(workflow, parent).caller(store, context.now);

                        // read a query, else make the mutation
                        const called = context.input.call;
                        const declared = present(
                            store.typeOf(called),
                            `the type of '${called.type.type}.${called.method}'`,
                        ).method(called.method);
                        const answered = !declared.mutates
                            ? await store.query(caller, called, CallSite.store)
                            : (
                                  await store.mutate(
                                      caller,
                                      {
                                          id: RequestId.create(store.environment),
                                          calls: [called],
                                      },
                                      { site: CallSite.store },
                                  )
                              )[0];

                        return recorded(answered);
                    },
                    handle: async (context) => context.prepared,
                },
                // record a placed step's output once, which wakes its run
                complete: async (context) => {
                    requirePlaced(context.target);
                    await context.update({
                        output: context.input.output,
                        completedAt: context.now,
                    });

                    return {};
                },
                // record a placed step's failure once, which wakes its run
                fail: async (context) => {
                    requirePlaced(context.target);
                    await context.update({
                        failure: context.input.failure,
                        completedAt: context.now,
                    });

                    return {};
                },
            },
        }),
        triggers.implement(),
    ];

    return { objects, mounts: Object.fromEntries(webhooks) };
}

/** Refuse ending a step that is not placed, or that ended already. */
function requirePlaced(target: StepRecord): void {
    if (target.function === null || target.completedAt !== null) {
        throw WorkflowError.of(
            "STEP_INVALID",
            `step '${target.name}' is invalid: only an open placed step ends by its runner`,
        );
    }
}
