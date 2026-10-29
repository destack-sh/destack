import { none, relation, through } from "@destack/access";
import { space } from "@destack/account/object";
import { and, eq, TABLE } from "@destack/db";
import { defineObject, field, Limits, method, ObjectType, ObjectWatch } from "@destack/object";
import { DeclarationName, ModuleMetadata } from "@destack/package";
import type { Declaration } from "@destack/package/declare";
import { RetryPolicy } from "@destack/environment";
import { canonicalize, Duration, type JsonValue, present, schema } from "@destack/schema";
import { type Call, ObjectReference, ObjectTypeReference, type Subject } from "@destack/sync";
import { Failure } from "@destack/error";
import { changeRequest } from "@destack/object";
import type { RunContext, Step } from "../step/step.ts";
import { ENDED_STATES, LIVE_STATES, Run, RUN_STATES } from "../run/run.ts";
import type { ObjectStore } from "@destack/object/server";
import type { Controller } from "@destack/db";
import type { InstalledServerContext, WebhookHandler } from "@destack/object/workload";
import { ScheduleOn, ScheduleSkip } from "../trigger/schedule.ts";
import {
    type Trigger,
    type TriggerDefinition,
    TRIGGER_KINDS,
    triggerOf,
} from "../trigger/trigger.ts";
import { TriggerServer } from "../trigger/server.ts";

/** The type-only key branding a run type with its workflow's name. */
declare const WORKFLOW: unique symbol;

/** A workflow's run type, typed by the workflow's name so waits tell it apart. */
export type WorkflowRun<Name extends string = string> = RunType & {
    /** The workflow the runs belong to, as a type only. */
    readonly [WORKFLOW]?: Name;
};

/** The object types a workflow's runs wait for: its declared ones, and change requests. */
export type WaitOf<Waits extends ObjectType> = Waits | typeof changeRequest;

/** A durable function of a package, run step by step as objects of the scope it runs in. */
export interface Workflow<
    Input = unknown,
    Output = unknown,
    Waits extends ObjectType = ObjectType,
    Name extends string = string,
> extends Declaration {
    /** The declaration kind. */
    readonly kind: "workflow";
    /** Where its runs live: in a space, or in the universe. */
    readonly scope: "space" | "universe";
    /** The schema of a run's input. */
    readonly input: schema.Schema<Input>;
    /** The schema of a run's output, `NO_OUTPUT` for a workflow returning nothing. */
    readonly output: schema.Schema<Output>;
    /** The object types a run waits for, whose writes wake it, always change requests. */
    readonly waits: readonly WaitOf<Waits>[];
    /** How a run retries an advance that failed without a final refusal. */
    readonly retries: RetryPolicy;
    /** How long a run lasts at most from its creation, absent for no limit. */
    readonly timeout: Duration | undefined;
    /** How long an ended run stays before it is removed, absent to stay. */
    readonly expiring: Duration | undefined;
    /** How often runs start and how many advance at once. */
    readonly limits: Limits;
    /** Whether a run takes its concurrency slot from its start to its end. */
    readonly isSingleton: boolean;
    /** The declared triggers starting runs: schedules, object changes and webhook deliveries. */
    readonly triggers: readonly Trigger<Input>[];
    /** The workflow's runs. */
    readonly run: WorkflowRun<Name>;
    /** The steps its runs took, nested in them. */
    readonly step: StepType;
    /** The triggers starting its runs: each declared one, and each schedule a person created. */
    readonly trigger: TriggerType;
    /** The object types the workflow declares: its runs, their steps and its triggers. */
    readonly objects: Readonly<Record<string, ObjectType>>;
    /** Build the call starting a run, which a method makes in its own transaction. */
    readonly start: (input: NoInfer<Input>, options: RunStart) => Call<Run["row"]>;
    /** Implement the workflow with its function and the triggers starting it. */
    readonly implement: (
        run: (step: Step<WaitOf<Waits>>, input: Input, context: RunContext) => Promise<Output>,
        options?: WorkflowServing,
    ) => WorkflowImplementation;
}

/** A workflow as its declarations read it, apart from the function implementing it. */
export type WorkflowDeclaration<Input = unknown, Output = unknown> = Omit<
    Workflow<Input, Output>,
    "implement"
>;

/** What a server needs beside a workflow's function: the source of its webhooks' secrets. */
export interface WorkflowServing {
    /** The installation's resources, keys and environment, which its webhooks verify with. */
    readonly server?: Pick<InstalledServerContext, "resources" | "keys" | "environment">;
}

/** A workflow as a server serves it: its types, its trigger controllers and its webhooks. */
export interface WorkflowImplementation {
    /** The run, step and trigger types with their handlers and controllers. */
    readonly objects: readonly ObjectType[];
    /** The controllers recording the declared triggers and following their changes. */
    readonly controllers: readonly ((store: ObjectStore) => Controller)[];
    /** Answer the declared triggers' webhook deliveries through a store. */
    readonly webhooks: (store: ObjectStore) => WebhookHandler;
}

/** How a run starts: its actor, the key one live run takes, a repeated start's effect, and when. */
export interface RunStart {
    /** The key one live run takes at a time, absent for none. */
    readonly key?: string;
    /** What a start of a live run's key does: answer it, or replace its input before it begins. */
    readonly mode?: "dedupe" | "replace";
    /** When the run starts, at once by default, in UTC epoch milliseconds. */
    readonly at?: number;
    /** The principal the run acts as, which its creation records. */
    readonly actor: Subject;
}

/** What a workflow declares. */
export interface WorkflowDefinition<
    Input,
    Output,
    Waits extends ObjectType = ObjectType,
    Name extends string = string,
> {
    /** The workflow's name, unique among the package's declarations. */
    readonly name: Name;
    /** Where its runs live: in a space by default, or in the universe. */
    readonly scope?: "space" | "universe";
    /** The schema of a run's input. */
    readonly input: schema.Schema<Input>;
    /** The schema of a run's output, `NO_OUTPUT` for a workflow returning nothing. */
    readonly output: schema.Schema<Output>;
    /** The object types a run waits for, whose writes wake it. */
    readonly waits?: readonly Waits[];
    /** How a run retries a failed advance, Temporal's default policy filling what is left out. */
    readonly retries?: Partial<RetryPolicy>;
    /** How long a run lasts at most from its creation. */
    readonly timeout?: Duration;
    /** How long an ended run stays before it is removed. */
    readonly expiring?: { readonly after: Duration };
    /** How often runs start and how many advance at once. */
    readonly limits?: Limits;
    /** Whether a run takes its concurrency slot from its start to its end. */
    readonly singleton?: boolean;
    /** The declared triggers starting runs, by name. */
    readonly triggers?: Readonly<Record<string, TriggerDefinition<Input>>>;
}

/** The output of a workflow returning nothing, which its runs record as null. */
export const NO_OUTPUT = schema.union([schema.void(), schema.null()]);

/** Declare a workflow: its input and output, waits, retries, timeout, limits and runs' expiry. */
export function defineWorkflow<
    Input,
    Output,
    Waits extends ObjectType = never,
    const Name extends string = string,
>(
    definition: WorkflowDefinition<Input, Output, Waits, Name>,
    module?: ModuleMetadata,
): Workflow<Input, Output, Waits, Name> {
    // stamp the declaring package, and refuse a singleton without a concurrency limit
    const owner = ModuleMetadata.require(module, "defineWorkflow").package;
    const name = DeclarationName.parse(definition.name);
    const limits = definition.limits ?? {};
    if (definition.singleton === true && limits.concurrency === undefined) {
        throw new TypeError(`singleton workflow ${name} declares no concurrency limit`);
    }

    // declare the runs, their steps and the triggers starting them, where the workflow places them
    const placement = definition.scope ?? "space";
    const scope = placement === "universe" ? "universe" : space;
    const run = declareRun(name, scope, definition.expiring?.after, module);
    const step = declareStep(name, scope, run, module);
    const trigger = declareTrigger(name, scope, module);

    // wait for change requests in every workflow, as any call step may await approval
    const waits = [
        changeRequest,
        ...(definition.waits ?? []).filter((type: ObjectType) => type !== changeRequest),
    ];

    // build the workflow, its implementation serving the run type with its controller
    const timeout = definition.timeout;
    const workflow: Workflow<Input, Output, Waits, Name> = Object.freeze({
        kind: "workflow" as const,
        name,
        package: owner,
        scope: placement,
        input: definition.input,
        output: definition.output,
        waits,
        retries: RetryPolicy.of(definition.retries),
        timeout,
        expiring: definition.expiring?.after,
        limits,
        isSingleton: definition.singleton === true,
        triggers: Object.entries(definition.triggers ?? {}).map(([named, declared]) =>
            triggerOf(named, declared),
        ),
        run,
        step,
        trigger,
        objects: { [`${name}Run`]: run, [`${name}Step`]: step, [`${name}Trigger`]: trigger },
        start: (input: Input, options: RunStart) =>
            run.call("create", {
                input: recorded(definition.input.parse(input)),
                ...(options.key === undefined ? {} : { key: options.key }),
                ...(options.mode === undefined ? {} : { mode: options.mode }),
                ...(options.at === undefined ? {} : { wakeAt: options.at }),
                ...ObjectReference.columns("actor", options.actor),
            }),
        implement: (
            code: (step: Step<WaitOf<Waits>>, input: Input, context: RunContext) => Promise<Output>,
            options: WorkflowServing = {},
        ) => {
            const triggers = new TriggerServer(workflow, options.server);
            const objects = [
                run.implement({
                    methods: {
                        create: (call, next) => Run.create(workflow, call, next),
                        reset: (call) =>
                            new Run(workflow, present(call.target, "the run")).reset(call),
                        // close the children ending with the run on each transition ending it
                        complete: (call, next) =>
                            Run.end(workflow, call, present(call.target, "the run"), next),
                        fail: (call, next) =>
                            Run.end(workflow, call, present(call.target, "the run"), next),
                        cancel: (call, next) =>
                            Run.end(workflow, call, present(call.target, "the run"), next),
                    },
                    controller: {
                        pending: { state: { in: ["pending", "waiting"] } },
                        watches: waits.map((type) =>
                            ObjectWatch.of(type.table, async (_changed, database) =>
                                (
                                    await database
                                        .select({ id: run.table[TABLE].column("id") })
                                        .from(run.table)
                                        .where(
                                            and(
                                                eq(run.table[TABLE].column("state"), "waiting"),
                                                eq(
                                                    run.table[TABLE].column("waitType"),
                                                    ObjectTypeReference.key(type.typeReference),
                                                ),
                                            ),
                                        )
                                ).map((row) => ({ id: schema.string().parse(row.id) })),
                            ),
                        ),
                        reconcile: async (reconciliation) => {
                            // advance each pending run, looking again at the earliest wake time
                            const wakes = await Promise.all(
                                reconciliation.rows.map((row) =>
                                    new Run(workflow, row).advance(code, reconciliation),
                                ),
                            );
                            const due = wakes.filter((wait) => wait !== undefined);

                            return due.length === 0 ? undefined : Math.min(...due);
                        },
                    },
                }),
                step,
                triggers.implement(),
            ];

            return { objects, controllers: triggers.controllers(), webhooks: triggers.webhooks() };
        },
    });

    return workflow;
}

/** Read a value as the JSON a run or step records. */
export function recorded(value: unknown): JsonValue {
    return schema.json().parse(JSON.parse(canonicalize(value ?? null)));
}

/** Declare a workflow's runs, which their actor controls and the store advances. */
function declareRun(
    name: string,
    scope: typeof space | "universe",
    expiring: Duration | undefined,
    module: ModuleMetadata | undefined,
) {
    // place the runs in a space or in the universe, advanced by their code through its role
    return defineObject(
        {
            name: `${name}-run`,
            plural: `${name}-runs`,
            identity: "run",
            scope,
            fields: {
                /** The run's input, as the workflow's input schema reads it. */
                input: field.json(schema.json()),
                /** The key one live run takes at a time, absent for none. */
                key: field.string().immutable().optional(),
                /** The principal the run acts as, the starting one unless the start names one. */
                actor: field.subject().caller().immutable(),
                /** Whether the run is due, waits to wake, is paused, or ended. */
                state: field.state({
                    initial: "pending",
                    transitions: {
                        wait: {
                            from: ["pending", "waiting"],
                            to: "waiting",
                            permission: "advance",
                            visibility: "package",
                            fields: ["waitType", "wakeAt"],
                        },
                        complete: {
                            from: ["pending", "waiting"],
                            to: "completed",
                            permission: "advance",
                            visibility: "package",
                            fields: ["output"],
                        },
                        fail: {
                            from: ["pending", "waiting"],
                            to: "failed",
                            permission: "advance",
                            visibility: "package",
                            fields: ["failure"],
                        },
                        pause: {
                            from: ["pending", "waiting"],
                            to: "paused",
                            permission: "control",
                        },
                        resume: { from: ["paused"], to: "pending", permission: "control" },
                        cancel: {
                            from: ["pending", "waiting", "paused"],
                            to: "cancelled",
                            permission: "control",
                        },
                        delay: {
                            from: ["pending", "waiting"],
                            to: "pending",
                            permission: "advance",
                            visibility: "package",
                            fields: ["attempts", "wakeAt", "failure"],
                        },
                        restart: {
                            from: [...RUN_STATES],
                            to: "pending",
                            permission: "advance",
                            visibility: "package",
                            fields: ["attempts", "wakeAt", "failure", "output"],
                        },
                    },
                }),
                /** The advances that failed and were retried so far. */
                attempts: field.integer().default(0),
                /** When the run is next due: its start, a sleep's end, a retry or a limit. */
                wakeAt: field.time().optional(),
                /** When the run times out, absent without a timeout. */
                deadline: field.time().immutable().optional(),
                /** When the run first advanced, absent before. */
                startedAt: field.time().optional(),
                /** The object type a waiting run waits for, by its reference key. */
                waitType: field.string().optional(),
                /** The output of a completed run, null for a workflow returning nothing. */
                output: field.json(schema.json()).optional(),
                /** The failure the run last answered, its final one once failed. */
                failure: field.json(Failure).optional(),
            },
            permissions: {
                // let the actor read and control a run, and its code advance it through its role
                read: relation("actor"),
                control: relation("actor"),
                advance: none(),
            },
            constraints: {
                key: {
                    kind: "index",
                    on: ["scope", "key"],
                    unique: true,
                    where: { state: { in: [...LIVE_STATES] }, key: { isNotNull: true } },
                },
            },
            ...(expiring === undefined
                ? {}
                : {
                      expiring: [
                          {
                              after: expiring,
                              from: "updatedAt",
                              where: { state: { in: [...ENDED_STATES] } },
                          },
                      ],
                  }),
        },
        () => ({
            create: method.create("advance", {
                fields: [
                    "input",
                    "key",
                    "wakeAt",
                    "deadline",
                    "actorPackageId",
                    "actorType",
                    "actorScope",
                    "actorId",
                ],
                input: schema.object({
                    /** What a start of a live run's key does. */
                    mode: schema.enum(["dedupe", "replace"]).exactOptional(),
                }),
                visibility: "package",
            }),
            /** Mark the run's first advance. */
            begin: method.update("advance", { fields: ["startedAt"], visibility: "package" }),
            /** Replace a run's input and start time before it first advances. */
            replace: method.update("advance", {
                fields: ["input", "wakeAt"],
                visibility: "package",
            }),
            /** Run a run again from a step, forgetting that step and every later one. */
            reset: method.mutation({
                permission: "control",
                input: schema.object({ step: schema.string().min(1) }),
                output: schema.object({ forgotten: schema.number().int() }),
            }),
        }),
        module,
    );
}

/** Declare a workflow's steps, each recording its output, failure or wake time. */
function declareStep(
    name: string,
    scope: typeof space | "universe",
    run: RunType,
    module: ModuleMetadata | undefined,
) {
    // nest the steps in their run, advanced as it is
    return defineObject(
        {
            name: `${name}-step`,
            plural: `${name}-steps`,
            identity: "step",
            scope,
            nested: { in: run, receive: "advance", delete: "cascade" },
            fields: {
                /** The step's name, unique within its run. */
                name: field.string().immutable(),
                /** The step's place in its run, counted from zero in the order the run took it. */
                position: field.integer().immutable(),
                /** The step's output once it completed. */
                output: field.json(schema.json()).optional(),
                /** The failure a call step answered, thrown again on every later advance. */
                failure: field.json(Failure).optional(),
                /** When a sleeping or waiting step wakes. */
                wakeAt: field.time().optional(),
                /** The change request a call step waits on while its call awaits approval. */
                changeRequestId: field.string().optional(),
                /** When the step completed, absent while it waits. */
                completedAt: field.time().optional(),
                /** The run an invoke step started. */
                child: field.object().optional(),
                /** Whether an invoke step's run is cancelled with this run or runs on. */
                onParentClose: field.enum(["cancel", "abandon"]).optional(),
            },
            permissions: { read: through("parent", "read"), advance: none() },
            constraints: { name: { kind: "unique", on: ["parentId", "name"] } },
        },
        () => ({
            create: method.create("advance", { visibility: "package" }),
            update: method.update("advance", {
                fields: ["output", "failure", "wakeAt", "changeRequestId", "completedAt"],
                visibility: "package",
            }),
            delete: method.delete("advance", { visibility: "package" }),
        }),
        module,
    );
}

/** Declare a workflow's triggers: declared ones, and schedules a space's managers create. */
function declareTrigger(
    name: string,
    scope: typeof space | "universe",
    module: ModuleMetadata | undefined,
) {
    // place the triggers beside the runs
    return defineObject(
        {
            name: `${name}-trigger`,
            plural: `${name}-triggers`,
            identity: "trigger",
            scope,
            fields: {
                /** The trigger's name, unique in its space. */
                name: field.string(schema.string().min(1)).immutable(),
                /** The kind of event the trigger fires on. */
                kind: field.enum(TRIGGER_KINDS).immutable().default("schedule"),
                /** The schedule whose occurrences fire, absent for a change or webhook trigger. */
                schedule: field.json(ScheduleOn).optional(),
                /** The input of a created schedule's runs, absent for a declared trigger. */
                input: field.json(schema.json()).optional(),
                /** Whether the trigger fires, is paused, or stopped on a failure until reset. */
                state: field.state({
                    initial: "active",
                    transitions: {
                        pause: { from: ["active"], to: "paused", permission: "manage" },
                        resume: { from: ["paused"], to: "active", permission: "manage" },
                        stop: {
                            from: ["active", "paused"],
                            to: "stopped",
                            permission: "advance",
                            fields: ["failure"],
                            visibility: "package",
                        },
                        reset: { from: ["stopped"], to: "active", permission: "manage" },
                    },
                }),
                /** The failure that stopped the trigger, absent while it never stopped. */
                failure: field.json(Failure).optional(),
                /** The latest occurrence fired or skipped. */
                lastScheduledAt: field.time().optional(),
                /** The occurrences skipped so far. */
                skipped: field.integer().default(0),
                /** The latest skipped occurrence and why it was skipped. */
                lastSkip: field.json(ScheduleSkip).optional(),
            },
            permissions: {
                // let the space's readers read and its managers create and pause triggers, and the store fire them
                read: through("scope", "read"),
                manage: through("scope", "manage"),
                advance: none(),
            },
            constraints: { name: { kind: "unique", on: ["scope", "name"] } },
        },
        () => ({
            create: method.create("manage", { fields: ["name", "schedule", "input"] }),
            /** Record a trigger the workflow's package declares. */
            declare: method.create("advance", {
                fields: ["name", "kind", "schedule"],
                visibility: "package",
            }),
            update: method.update("manage", { fields: ["schedule", "input"] }),
            delete: method.delete("manage"),
            /** Remove a trigger its package no longer declares. */
            retire: method.delete("advance", { visibility: "package" }),
            /** Record the occurrences fired and skipped, and a changed declared schedule. */
            record: method.update("advance", {
                fields: ["lastScheduledAt", "skipped", "lastSkip", "schedule"],
                visibility: "package",
            }),
        }),
        module,
    );
}

/** A workflow's run type. */
export type RunType = ReturnType<typeof declareRun>;
/** A workflow's step type. */
export type StepType = ReturnType<typeof declareStep>;
/** A workflow's trigger type. */
export type TriggerType = ReturnType<typeof declareTrigger>;
