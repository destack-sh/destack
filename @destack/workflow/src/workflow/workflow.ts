import { contained, relation, through } from "@destack/access";
import { space } from "@destack/account/object";
import { and, eq, TABLE } from "@destack/db";
import { field, Limits, method, ObjectType, ObjectWatch } from "@destack/object";
import { DeclarationName, ModuleMetadata } from "@destack/package";
import type { Declaration } from "@destack/package/declare";
import { RetryPolicy } from "@destack/runtime/timer";
import { canonicalize, Duration, type JsonValue, schema } from "@destack/schema";
import {
    type Call,
    Failure,
    ObjectReference,
    ObjectTypeReference,
    type Subject,
} from "@destack/sync";
import type { RunContext, Step } from "../step/step.ts";
import { ENDED_STATES, LIVE_STATES, Run, RUN_STATES } from "../run/run.ts";
import type { ObjectStore } from "@destack/object/server";
import type { Controller } from "@destack/runtime/control";
import type { ServerContext, WebhookHandler } from "@destack/runtime/workload";
import { ScheduleOn, ScheduleSkip } from "../trigger/schedule.ts";
import { Trigger, type TriggerDefinition, TRIGGER_KINDS } from "../trigger/trigger.ts";
import { TriggerHost } from "../trigger/host.ts";

/** A durable function of a package, run step by step as objects of the space it runs in. */
export interface Workflow<Input = unknown, Output = unknown> extends Declaration {
    /** The declaration kind. */
    readonly kind: "workflow";
    /** The schema of a run's input. */
    readonly input: schema.Schema<Input>;
    /** The schema of a run's output, absent for a workflow returning nothing. */
    readonly output: schema.Schema<Output> | undefined;
    /** The object types a run waits for, whose writes wake it. */
    readonly waits: readonly ObjectType[];
    /** How a run retries an advance that failed without a final refusal. */
    readonly retries: RetryPolicy;
    /** How long a run lasts at most from its creation, absent for no limit. */
    readonly timeout: Duration | undefined;
    /** How long an ended run stays before it is removed, absent to stay. */
    readonly expiring: Duration | undefined;
    /** How often runs start and how many advance at once. */
    readonly limits: Limits;
    /** Whether a run holds its concurrency slot from its start to its end. */
    readonly isSingleton: boolean;
    /** The declared triggers starting runs: schedules, object changes and webhook deliveries. */
    readonly triggers: readonly Trigger<Input>[];
    /** The workflow's runs. */
    readonly run: ObjectType;
    /** The steps its runs took, nested in them. */
    readonly step: ObjectType;
    /** The triggers starting its runs: each declared one, and each schedule a person created. */
    readonly trigger: ObjectType;
    /** The object types the workflow declares: its runs, their steps and its triggers. */
    readonly objects: Readonly<Record<string, ObjectType>>;
    /** Build the call starting a run, which a method makes in its own transaction. */
    readonly start: (input: NoInfer<Input>, options?: RunStart) => Call;
    /** Implement the workflow with its function and the triggers starting it. */
    readonly implement: (
        run: (step: Step, input: Input, context: RunContext) => Promise<Output>,
        options?: WorkflowServing,
    ) => WorkflowImplementation;
}

/** What a server needs beside a workflow's function: the source of its webhooks' secrets. */
export interface WorkflowServing {
    /** The installation's resources and derived keys, the source of webhook secrets. */
    readonly server?: Pick<ServerContext, "resources" | "keys">;
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

/** How a run starts: the key one live run takes, what another start of the key does, and when. */
export interface RunStart {
    /** The key one live run takes at a time, absent for none. */
    readonly key?: string;
    /** What a start of a live run's key does: answer it, or replace its input before it begins. */
    readonly mode?: "dedupe" | "replace";
    /** When the run starts, at once by default, in UTC epoch milliseconds. */
    readonly at?: number;
    /** The principal the run acts as, the starting principal by default. */
    readonly actor?: Subject;
}

/** What a workflow declares. */
export interface WorkflowDefinition<Input, Output> {
    /** The workflow's name, unique among the package's declarations. */
    readonly name: string;
    /** The schema of a run's input. */
    readonly input: schema.Schema<Input>;
    /** The schema of a run's output, absent for a workflow returning nothing. */
    readonly output?: schema.Schema<Output>;
    /** The object types a run waits for, whose writes wake it. */
    readonly waits?: readonly ObjectType[];
    /** How a run retries a failed advance, Temporal's default policy filling what is left out. */
    readonly retries?: Partial<RetryPolicy>;
    /** How long a run lasts at most from its creation. */
    readonly timeout?: Duration;
    /** How long an ended run stays before it is removed. */
    readonly expiring?: { readonly after: Duration };
    /** How often runs start and how many advance at once. */
    readonly limits?: Limits;
    /** Whether a run holds its concurrency slot from its start to its end. */
    readonly singleton?: boolean;
    /** The declared triggers starting runs, by name. */
    readonly triggers?: Readonly<Record<string, TriggerDefinition<Input>>>;
}

/** Declare a workflow: its input and output, waits, retries, timeout, limits and runs' expiry. */
export function defineWorkflow<Input, Output = void>(
    definition: WorkflowDefinition<Input, Output>,
    module?: ModuleMetadata,
): Workflow<Input, Output> {
    // stamp the declaring package, and refuse a singleton without a concurrency limit
    const owner = ModuleMetadata.require(module, "defineWorkflow").package;
    const name = DeclarationName.parse(definition.name);
    const limits = definition.limits ?? {};
    if (definition.singleton === true && limits.concurrency === undefined) {
        throw new TypeError(`singleton workflow ${name} declares no concurrency limit`);
    }

    // declare the runs, which their actor reads and controls and the store advances
    const run = ObjectType.declare(
        {
            name: `${name}-run`,
            plural: `${name}-runs`,
            identity: "run",
            scope: space,
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
                            isInternal: true,
                            fields: ["waitType", "wakeAt"],
                        },
                        complete: {
                            from: ["pending", "waiting"],
                            to: "completed",
                            permission: "advance",
                            isInternal: true,
                            ...(definition.output === undefined ? {} : { fields: ["output"] }),
                        },
                        fail: {
                            from: ["pending", "waiting"],
                            to: "failed",
                            permission: "advance",
                            isInternal: true,
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
                            isInternal: true,
                            fields: ["attempts", "wakeAt", "failure"],
                        },
                        restart: {
                            from: [...RUN_STATES],
                            to: "pending",
                            permission: "advance",
                            isInternal: true,
                            fields: [
                                "attempts",
                                "wakeAt",
                                "failure",
                                ...(definition.output === undefined ? [] : ["output"]),
                            ],
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
                // the output of a completed run of a workflow with one
                ...(definition.output === undefined
                    ? {}
                    : { output: field.json(schema.json()).optional() }),
                /** The failure the run last answered, its final one once failed. */
                failure: field.json(Failure).optional(),
            },
            permissions: {
                // let the actor read and control a run, and the installations of its space advance it
                read: relation("actor"),
                control: relation("actor"),
                advance: contained(),
            },
            constraints: {
                key: {
                    kind: "index",
                    on: ["scope", "key"],
                    unique: true,
                    where: { state: { in: [...LIVE_STATES] }, key: { isNotNull: true } },
                },
            },
            ...(definition.expiring === undefined
                ? {}
                : {
                      expiring: [
                          {
                              after: definition.expiring.after,
                              from: "updatedAt",
                              where: { state: { in: [...ENDED_STATES] } },
                          },
                      ],
                  }),
            methods: {
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
                    isInternal: true,
                }),
                /** Mark the run's first advance. */
                begin: method.update("advance", { fields: ["startedAt"], isInternal: true }),
                /** Replace a run's input and start time before it first advances. */
                replace: method.update("advance", {
                    fields: ["input", "wakeAt"],
                    isInternal: true,
                }),
                /** Run a run again from a step, forgetting that step and every later one. */
                reset: method.mutation({
                    permission: "control",
                    input: schema.object({ step: schema.string().min(1) }),
                    output: schema.object({ forgotten: schema.number().int() }),
                }),
            },
        },
        module,
    );

    // declare the steps, in the order a run took them, each recording its output, failure or wake time
    const step = ObjectType.declare(
        {
            name: `${name}-step`,
            plural: `${name}-steps`,
            identity: "step",
            scope: space,
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
                /** When the step completed, absent while it waits. */
                completedAt: field.time().optional(),
            },
            permissions: { read: through("parent", "read"), advance: contained() },
            constraints: { name: { kind: "unique", on: ["parentId", "name"] } },
            methods: {
                create: method.create("advance", { isInternal: true }),
                update: method.update("advance", {
                    fields: ["output", "failure", "wakeAt", "completedAt"],
                    isInternal: true,
                }),
                delete: method.delete("advance", { isInternal: true }),
            },
        },
        module,
    );

    // declare the triggers: one row per declared trigger, and the schedules a space's managers create
    const trigger = ObjectType.declare(
        {
            name: `${name}-trigger`,
            plural: `${name}-triggers`,
            identity: "trigger",
            scope: space,
            fields: {
                /** The trigger's name, unique in its space. */
                name: field.string(schema.string().min(1)).immutable(),
                /** Whether the workflow's package declares the trigger or a manager created it. */
                origin: field.enum(["declared", "created"]).immutable().default("created"),
                /** The kind of event the trigger fires on. */
                kind: field.enum(TRIGGER_KINDS).immutable().default("schedule"),
                /** The schedule whose occurrences fire, absent for a change or webhook trigger. */
                schedule: field.json(ScheduleOn).optional(),
                /** The input of a created schedule's runs, absent for a declared trigger. */
                input: field.json(schema.json()).optional(),
                /** Whether the trigger fires or is paused. */
                state: field.state({
                    initial: "active",
                    transitions: {
                        pause: { from: ["active"], to: "paused", permission: "manage" },
                        resume: { from: ["paused"], to: "active", permission: "manage" },
                    },
                }),
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
                advance: contained(),
            },
            constraints: { name: { kind: "unique", on: ["scope", "name"] } },
            methods: {
                create: method.create("manage", { fields: ["name", "schedule", "input"] }),
                /** Record a trigger the workflow's package declares. */
                declare: method.create("advance", {
                    fields: ["name", "origin", "kind", "schedule"],
                    isInternal: true,
                }),
                update: method.update("manage", { fields: ["schedule", "input"] }),
                delete: method.delete("manage"),
                /** Remove a trigger its package no longer declares. */
                retire: method.delete("advance", { isInternal: true }),
                /** Record the occurrences fired and skipped, and a changed declared schedule. */
                record: method.update("advance", {
                    fields: ["lastScheduledAt", "skipped", "lastSkip", "schedule"],
                    isInternal: true,
                }),
            },
        },
        module,
    );

    // build the workflow, its implementation serving the run type with its controller
    const waits = definition.waits ?? [];
    const timeout = definition.timeout;
    const workflow: Workflow<Input, Output> = Object.freeze({
        kind: "workflow" as const,
        name,
        package: owner,
        input: definition.input,
        output: definition.output,
        waits,
        retries: RetryPolicy.of(definition.retries),
        timeout,
        expiring: definition.expiring?.after,
        limits,
        isSingleton: definition.singleton === true,
        triggers: Object.entries(definition.triggers ?? {}).map(([named, declared]) =>
            Trigger.of(named, declared),
        ),
        run,
        step,
        trigger,
        objects: { [`${name}Run`]: run, [`${name}Step`]: step, [`${name}Trigger`]: trigger },
        start: (input: Input, options: RunStart = {}) =>
            run.call("create", {
                input: recorded(definition.input.parse(input)),
                ...(options.key === undefined ? {} : { key: options.key }),
                ...(options.mode === undefined ? {} : { mode: options.mode }),
                ...(options.at === undefined ? {} : { wakeAt: options.at }),
                ...(options.actor === undefined
                    ? {}
                    : ObjectReference.columns("actor", options.actor)),
            }),
        implement: (
            code: (step: Step, input: Input, context: RunContext) => Promise<Output>,
            options: WorkflowServing = {},
        ) => {
            const host = new TriggerHost(workflow, options.server);
            const objects = [
                run.implement({
                    methods: {
                        create: (call, next) => Run.create(workflow, call, next),
                        reset: (call) => Run.reset(workflow, call),
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
                                    Run.advance(workflow, code, row, reconciliation),
                                ),
                            );
                            const due = wakes.filter((wait) => wait !== undefined);

                            return due.length === 0 ? undefined : Math.min(...due);
                        },
                    },
                }),
                step,
                host.implement(),
            ];

            return { objects, controllers: host.controllers(), webhooks: host.webhooks() };
        },
    });

    return workflow;
}

/** Read a value as the JSON a run or step records. */
export function recorded(value: unknown): JsonValue {
    return schema.json().parse(JSON.parse(canonicalize(value ?? null)));
}
