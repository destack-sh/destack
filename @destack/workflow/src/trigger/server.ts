import {
    and,
    Change,
    Condition,
    eq,
    inArray,
    TABLE,
    DatabaseError,
    type DatabaseConnection,
    Key,
    Log,
    type Match,
    Namespace,
    now,
    Predicate,
    type Row,
    type Select,
    type Table,
    type LogPosition,
    Transaction,
} from "@destack/db";
import { ObjectType, type ObjectReconciliation, ObjectWatch, refuseInput } from "@destack/object";
import type { AdapterFetch, ObjectStore } from "@destack/object/server";
import type { Cursor } from "@destack/db";
import { RequestId, type Call, Scope } from "@destack/sync";
import type { ServerContext } from "@destack/object/workload";
import { Failure } from "@destack/error";
import { Manager } from "@destack/access";
import { canonicalize, present, schema } from "@destack/schema";
import { LIVE_STATES } from "../run/run.ts";
import type { TriggerType, Workflow } from "../workflow/workflow.ts";
import type { ChangeOperation, ObjectChange } from "./change.ts";
import { Schedule, ScheduleOn, ScheduleSkip } from "./schedule.ts";
import { ChangeTrigger, ScheduleTrigger, type Trigger, WebhookTrigger } from "./trigger.ts";
import type { WebhookDelivery } from "../webhook/webhook.ts";
import type {} from "@destack/package/import-meta";
import { WorkflowError } from "../error/index.ts";

/** The parts of a workflow its triggers start runs through. */
type StartedWorkflow<Input> = Pick<
    Workflow<Input>,
    "name" | "package" | "scope" | "input" | "start" | "run" | "trigger" | "triggers"
>;

/** The most missed occurrences a schedule looks back over at once. */
const MAX_MISSED_OCCURRENCES = 100;

/** How long a trigger's checkpoint keeps changes past its last advance: a week. */
const SLOT_MILLISECONDS = 7 * 24 * 60 * 60_000;

/** The rows one snapshot page reads: a few hundred kilobytes. */
const SNAPSHOT_PAGE_ROWS = 500;

/** A trigger's row as its workflow's trigger type stores it. */
type TriggerRecord = Select<TriggerType["table"]>;

/** A workflow's triggers as its server fires them: schedules, changes and webhooks. */
export class TriggerServer<Input> {
    /** The workflow the triggers start runs of. */
    readonly #workflow: StartedWorkflow<Input>;
    /** The declared triggers, by name. */
    readonly #declared: ReadonlyMap<string, Trigger<Input>>;

    /** Host a workflow's declared triggers, refusing repeated names. */
    constructor(workflow: StartedWorkflow<Input>) {
        // require each trigger to have a name of its own
        const declared = new Map<string, Trigger<Input>>();
        for (const trigger of workflow.triggers) {
            if (declared.has(trigger.name)) {
                throw new TypeError(`${workflow.name} has two triggers named ${trigger.name}`);
            }
            declared.set(trigger.name, trigger);
        }
        this.#workflow = workflow;
        this.#declared = declared;
    }

    /** Implement the trigger type: check created schedules and fire active ones. */
    implement(): ObjectType {
        const { trigger } = this.#workflow;

        return trigger.implement({
            methods: {
                create: (call, next) => {
                    // check the created schedule and its runs' input
                    const schedule = this.#checked(call.input.schedule, call.input.input);
                    const nextScheduledAt =
                        new Schedule(schedule.timing).following(call.now) ?? null;

                    return next(call.with({ input: { ...call.input, schedule, nextScheduledAt } }));
                },
                update: (call, next) => {
                    // check the changed schedule and input
                    const { target, input } = call;
                    const schedule = this.#checked(
                        input.schedule ?? target.schedule,
                        input.input ?? target.input,
                    );
                    const nextScheduledAt =
                        new Schedule(schedule.timing).following(call.now) ?? null;

                    return next(call.with({ input: { ...input, schedule, nextScheduledAt } }));
                },
            },
            controllers: [
                // start each active change trigger's runs from its source's changes, at each start and change
                ...[...this.#declared.values()]
                    .filter(
                        (declared): declared is ChangeTrigger<Input> => declared.kind === "change",
                    )
                    .map((declared) => ({
                        name: declared.name,
                        pending: {
                            state: "active" as const,
                            kind: "change" as const,
                            name: declared.name,
                        },
                        key: ["name" as const],
                        watches: [
                            ObjectWatch.of(TriggerServer.#table(declared), () => [
                                { name: declared.name },
                            ]),
                        ],
                        reconcile: async (reconciliation: ObjectReconciliation<typeof trigger>) => {
                            await this.#follow(reconciliation.store, declared);
                        },
                    })),
                {
                    // fire a declared schedule once, recording its next occurrence
                    name: "start",
                    pending: {
                        state: "active",
                        kind: "schedule",
                        nextScheduledAt: { isNull: true },
                    },
                    reconcile: async (reconciliation) => {
                        await Promise.all(
                            reconciliation.rows.map((row) => this.#fireDue(row, reconciliation)),
                        );
                    },
                },
                {
                    // fire each active schedule's due occurrences once its next one comes due
                    pending: { state: "active", kind: "schedule", nextScheduledAt: { lte: now } },
                    reconcile: async (reconciliation) => {
                        await Promise.all(
                            reconciliation.rows.map((row) => this.#fireDue(row, reconciliation)),
                        );
                    },
                },
            ],
        });
    }

    /** Build the cursor recording the declared triggers' rows at each start. */
    cursor(): (store: ObjectStore) => Cursor {
        const { trigger } = this.#workflow;

        return (store) => ({
            name: `${trigger.name}:declared`,
            tables: [],
            take: async () => this.#declare(store, this.scope(store)),
        });
    }

    /** Serve the triggers' webhooks, starting a run of each active trigger per delivery. */
    webhooks(
        server: Pick<ServerContext, "resources"> | undefined,
    ): ReadonlyMap<string, AdapterFetch> {
        // group the webhook triggers by their webhook, refusing to serve one without the installation
        const triggers = [...this.#declared.values()].filter((each) => isWebhook(each));
        const webhooks = Map.groupBy(triggers, (trigger) => trigger.webhook);
        const [first] = triggers;
        if (first !== undefined && server === undefined) {
            throw new TypeError(
                `${this.#workflow.name} serves webhook trigger ${first.name} without its installation`,
            );
        }

        // start a run of each active trigger once per delivery
        const receive =
            (fired: readonly WebhookTrigger<Input>[]) =>
            async (delivery: WebhookDelivery, store: ObjectStore) => {
                for (const trigger of fired) {
                    await this.#requireActive(store, trigger.name);
                    const event = { key: WebhookTrigger.key(delivery), at: delivery.receivedAt };
                    await this.start(store, trigger.name, trigger.input(delivery), event, {
                        key: `trigger/${trigger.name}/${event.key}`,
                    });
                }
            };

        return new Map(
            server === undefined
                ? []
                : [...webhooks].map(([webhook, fired]) => [
                      webhook.name,
                      webhook.serve(receive(fired), server),
                  ]),
        );
    }

    /** Check a created schedule's timing and its runs' input, answering the checked schedule. */
    #checked(schedule: unknown, input: unknown): ScheduleOn {
        // check the timing and the input
        const checked = parsed(ScheduleOn, schedule);
        Schedule.require(checked.timing);
        parsed(this.#workflow.input, input);

        return checked;
    }

    /** Fire a schedule's due occurrences, recording its next one. */
    async #fireDue(row: TriggerRecord, reconciliation: ObjectReconciliation): Promise<void> {
        // find the latest occurrences due since the last one or within the deadline
        const { now: time, store } = reconciliation;
        const schedule = ScheduleOn.parse(row.schedule);
        const { timing, deadline } = schedule;
        const after = row.lastScheduledAt ?? time - deadline - 1;
        const occurrences = new Schedule(timing);
        const { due, earlier } = occurrences.recent(after, time, MAX_MISSED_OCCURRENCES);
        const last = due.at(-1);
        const next = occurrences.following(time) ?? null;
        if (last === undefined) {
            await reconciliation.mutate([
                this.#workflow.trigger.call("record", {
                    ...this.#workflow.trigger.target(row),
                    lastScheduledAt: row.lastScheduledAt,
                    nextScheduledAt: next,
                    skipped: row.skipped,
                    lastSkip: row.lastSkip,
                }),
            ]);

            return;
        }

        // skip the missed, late and superseded occurrences, counting the uncounted earlier ones on the first
        const isDue = (at: number) => time - at <= deadline;
        const skips: ScheduleSkip[] = due.flatMap((at, index) =>
            at === last && isDue(at) ? [] : [skip(at, isDue(at), index === 0 ? earlier : 0)],
        );

        // fire the latest occurrence within its deadline as the schedule's concurrency lets it
        const fired = isDue(last) ? await this.#occur(row, schedule, last, store) : undefined;
        const allSkips = fired === undefined ? skips : [...skips, ...fired];
        const latest = allSkips.at(-1);
        await reconciliation.mutate([
            this.#workflow.trigger.call("record", {
                ...this.#workflow.trigger.target(row),
                lastScheduledAt: last,
                nextScheduledAt: next,
                skipped: row.skipped + allSkips.length,
                lastSkip: latest ?? null,
            }),
        ]);
    }

    /** Start an occurrence's run as the schedule's concurrency lets it, answering a skip. */
    async #occur(
        row: TriggerRecord,
        schedule: ScheduleOn,
        at: number,
        store: ObjectStore,
    ): Promise<readonly ScheduleSkip[] | undefined> {
        // find the live run of a schedule refusing overlaps
        const workflow = this.#workflow;
        const key =
            schedule.concurrency === "allow" ? `trigger/${row.name}/${at}` : `trigger/${row.name}`;
        const [live] =
            schedule.concurrency === "allow"
                ? []
                : await store.database
                      .select()
                      .from(workflow.run.table)
                      .where(this.#liveRun(row.scope, key));
        if (live !== undefined && schedule.concurrency === "forbid") {
            return [{ at, reason: "overlap" }];
        }

        // cancel the run a replacing occurrence displaces, then start the occurrence's run
        const cancel: Call[] =
            live === undefined
                ? []
                : [
                      workflow.run.call(
                          "cancel",
                          workflow.run.target({
                              scope: row.scope,
                              id: schema.string().parse(live["id"]),
                          }),
                      ),
                  ];
        const input =
            row.managerName === null
                ? workflow.input.parse(row.input)
                : this.#schedule(row.name).input({ at });
        const event = { key: ScheduleTrigger.key({ at }), at };
        await this.start(store, row.name, input, event, { key, before: cancel, scope: row.scope });

        return undefined;
    }

    /** Start a trigger's run for an event once, under a request the event's key and time derive. */
    async start(
        store: ObjectStore,
        name: string,
        input: Input,
        event: { readonly key: string; readonly at: number },
        options: {
            readonly key?: string;
            readonly before?: readonly Call[];
            readonly scope?: string;
        },
    ): Promise<void> {
        // derive the request from the trigger's identity and the event, stamped at the event's time
        const workflow = this.#workflow;
        const id = await RequestId.derive(
            event.at,
            `${this.identity(name)} ${event.key}`,
            store.environment.crypto,
        );
        const self = store.as(workflow.run);
        const run = workflow.start(input, {
            actor: self.subject,
            ...(options.key === undefined ? {} : { key: options.key }),
        });
        await store.mutate(
            self,
            { id, calls: [...(options.before ?? []), run] },
            { scope: options.scope ?? this.scope(store) },
        );
    }

    /** Read a trigger's row in a space by its unique name, absent while not recorded. */
    async row(
        database: DatabaseConnection,
        scope: string,
        name: string,
    ): Promise<TriggerRecord | undefined> {
        // read the row by the space and the name
        const { trigger } = this.#workflow;
        const column = (field: string) => trigger.table[TABLE].column(field);
        const [row] = await database
            .select()
            .from(trigger.table)
            .where(and(eq(column("scope"), scope), eq(column("name"), name)));

        return row;
    }

    /** Read a trigger's identity, which names its log checkpoint and derives its requests. */
    identity(name: string): string {
        return `trigger/${this.#workflow.package.id}/${this.#workflow.name}/${name}`;
    }

    /** Read the log checkpoint of a change trigger's first snapshot, paged through rows. */
    snapshotCheckpoint(name: string): string {
        return `${this.identity(name)}/snapshot`;
    }

    /** Drop a change trigger's log checkpoints, retaining no more changes for it. */
    async dropCheckpoints(database: DatabaseConnection, name: string): Promise<void> {
        const log = new Log(database);
        await log.drop(this.identity(name));
        await log.drop(this.snapshotCheckpoint(name));
    }

    /** Read the scope the workflow's runs live in: the store's space, or the universe. */
    scope(store: Pick<ObjectStore, "space">): string {
        return this.#workflow.scope === "universe"
            ? Scope.universe.id
            : present(store.space, `the space ${this.#workflow.name} runs in`);
    }

    /** The workflow the triggers start, as controllers read it. */
    get workflow(): StartedWorkflow<Input> {
        return this.#workflow;
    }

    /** Refuse a delivery to a paused or unrecorded trigger, which its sender delivers again. */
    async #requireActive(store: ObjectStore, name: string): Promise<void> {
        const row = await this.row(store.database, this.scope(store), name);
        // refuse a trigger not recorded yet
        if (row === undefined) {
            throw WorkflowError.of("TRIGGER_UNRECORDED", `trigger ${name} is not recorded yet`);
        }
        // refuse a paused trigger
        else if (row.state === "paused") {
            throw WorkflowError.of("TRIGGER_PAUSED", `trigger ${name} is paused`);
        }
    }

    /** Match the workflow's live run of a key in a space. */
    #liveRun(scope: string, key: string) {
        const column = (name: string) => this.#workflow.run.table[TABLE].column(name);

        return and(
            eq(column("scope"), scope),
            eq(column("key"), key),
            inArray(column("state"), [...LIVE_STATES]),
        );
    }

    /** Read a declared schedule trigger, refusing a name declaring none. */
    #schedule(name: string): ScheduleTrigger<Input> {
        const trigger = this.#declared.get(name);
        if (trigger === undefined || !isSchedule(trigger)) {
            throw new TypeError(`declared trigger ${name} has no schedule`);
        }

        return trigger;
    }

    /** Create missing declared rows, update changed schedules, retire undeclared attached rows. */
    async #declare(store: ObjectStore, scope: string): Promise<void> {
        // leave a scope's own package, which no installation's build declares triggers for
        const { trigger } = this.workflow;
        const { server } = store.implementation(trigger);
        if (server === undefined) {
            return;
        }

        // read the rows the workflow's installation declared in the space, detached ones too
        const manager = { installationId: server.installation, packageId: trigger.package.id };
        const column = (name: string) => trigger.table[TABLE].column(name);
        const rows = await store.database
            .select()
            .from(trigger.table)
            .where(
                and(
                    eq(column("scope"), scope),
                    eq(column("managerInstallationId"), manager.installationId),
                    eq(column("managerPackageId"), manager.packageId),
                ),
            );

        // create each missing declared trigger, and update an attached one's schedule that changed, each due at its next occurrence
        const calls: (readonly [string, Call])[] = [];
        const at = store.clock.now();
        const due = (schedule: ScheduleOn) => new Schedule(schedule.timing).following(at) ?? null;
        for (const declared of this.#declared.values()) {
            const { kind } = declared;
            const schedule = declared.kind === "schedule" ? declared.schedule : undefined;
            const row = rows.find((each) => each.name === declared.name);
            if (row === undefined) {
                calls.push([
                    declared.name,
                    trigger.call("declare", {
                        ...trigger.in(scope),
                        name: declared.name,
                        kind,
                        ...(schedule === undefined ? {} : { schedule }),
                    }),
                ]);
            } else if (
                Manager.isManaged(row) &&
                schedule !== undefined &&
                canonicalize(schedule) !== canonicalize(row.schedule)
            ) {
                calls.push([
                    declared.name,
                    trigger.call("record", {
                        ...trigger.target(row),
                        lastScheduledAt: row.lastScheduledAt,
                        nextScheduledAt: due(schedule),
                        skipped: row.skipped,
                        lastSkip: null,
                        schedule,
                    }),
                ]);
            }
        }

        // retire the attached rows no declaration has any more
        const retired = rows.filter(
            (each) =>
                Manager.isManaged(each) &&
                ![...this.#declared.values()].some((declared) => declared.name === each.name),
        );
        calls.push(
            ...retired.map(
                (row) => [row.name, trigger.call("retire", trigger.target(row))] as const,
            ),
        );
        if (calls.length === 0) {
            return;
        }

        // record each call as the trigger's manager, dropping the retired triggers' log checkpoints in the same commit
        await Transaction.run(store.database, async (transaction) => {
            for (const [name, call] of calls) {
                await store.mutate(
                    store.as(trigger),
                    { id: RequestId.create(store.environment), calls: [call] },
                    {
                        scope,
                        transaction,
                        manager: { ...manager, name: `${trigger.collection}/${name}` },
                    },
                );
            }
            for (const row of retired) {
                transaction.record((committing) => this.dropCheckpoints(committing, row.name));
            }
        });
    }

    /** Start the runs of a change trigger's matching changes since its checkpoint. */
    async #follow(store: ObjectStore, trigger: ChangeTrigger<Input>): Promise<void> {
        // wait for the trigger's row to resume or reset it while it is not active
        const change = trigger.change;
        const database = store.database;
        const scope = this.scope(store);
        const row = await this.row(database, scope, trigger.name);
        if (row?.state !== "active") {
            return;
        }

        // continue after the trigger's checkpoint, or start it
        const table = TriggerServer.#table(trigger);
        const log = new Log(database);
        const reader = { name: this.identity(trigger.name), tables: [table] };
        const position = await log.position();
        const timeline = { log: position.log, epoch: position.epoch };
        const match =
            change.where === undefined
                ? undefined
                : Predicate.compile(
                      Condition.resolve(change.where, Namespace.fields(table)),
                      table,
                  );
        const placed = await log.checkpoint(reader.name);

        // start each matching change's run page by page, advancing the checkpoint past each page of changes
        try {
            let after = placed ?? (await this.#begin(store, trigger, position));
            for (;;) {
                const page = await log.read({ tables: [table], after });
                for (const logged of page.changes) {
                    const matching = TriggerServer.#see(change.operations, logged, timeline, match);
                    if (matching !== undefined) {
                        await this.#startChange(store, trigger, matching, logged.changedAt);
                    }
                }
                after = Math.max(after, page.sequence);

                // advance past a page of changes, ending at the head
                if (page.changes.length === 0) {
                    break;
                }
                await log.advance(reader, after, store.clock.now() + SLOT_MILLISECONDS);
            }
        } catch (error) {
            // stop the trigger on changes compacted past its checkpoint until a manager resets it
            if (!DatabaseError.is(error, "CHANGES_COMPACTED")) {
                throw error;
            }
            await this.#stop(store, trigger, row, error);
        }
    }

    /** Stop a trigger that lost changes to compaction, dropping its checkpoints for a reset. */
    async #stop(
        store: ObjectStore,
        trigger: ChangeTrigger<Input>,
        row: TriggerRecord,
        cause: unknown,
    ): Promise<void> {
        // record why the trigger stopped
        const { name } = trigger;
        const type = this.workflow.trigger;
        const failure = Failure.of(
            WorkflowError.of("CHANGES_COMPACTED", `trigger ${name} lost changes to compaction`, {
                cause,
            }),
        );

        // record the stop, dropping the checkpoints in the same commit
        await Transaction.run(store.database, async (transaction) => {
            await store.mutate(
                store.as(type),
                {
                    id: RequestId.create(store.environment),
                    calls: [type.call("stop", { ...type.target(row), failure })],
                },
                { scope: row.scope, transaction },
            );
            transaction.record((committing) => this.dropCheckpoints(committing, name));
        });
    }

    /** Start the checkpoint at the log's head, first starting a snapshot's matching runs. */
    async #begin(
        store: ObjectStore,
        trigger: ChangeTrigger<Input>,
        position: LogPosition,
    ): Promise<number> {
        // start the checkpoint at the head, or resume the snapshot begun at an earlier head
        const change = trigger.change;
        const log = new Log(store.database);
        const table = TriggerServer.#table(trigger);
        const reader = { name: this.identity(trigger.name), tables: [table] };
        const snapshotReader = { name: this.snapshotCheckpoint(trigger.name), tables: [table] };
        const begun =
            change.from === "snapshot" ? await log.checkpoint(snapshotReader.name) : undefined;
        const sequence = begun ?? position.sequence;
        const timeline = { log: position.log, epoch: position.epoch };
        if (change.from === "now") {
            await log.advance(reader, sequence, store.clock.now() + SLOT_MILLISECONDS);

            return sequence;
        }

        // start a run for each matching row as created at the snapshot's reading, page by page
        const snapshot = log.at({ ...timeline, sequence });
        const at = store.clock.now();
        let last: Row | undefined;
        for (;;) {
            // keep the snapshot's checkpoint for a week past each page
            await log.advance(snapshotReader, sequence, store.clock.now() + SLOT_MILLISECONDS);
            const rows = await snapshot.ordered(table, {
                where: change.where ?? {},
                orderBy: { id: "asc" },
                ...(last === undefined ? {} : { after: last }),
                limit: SNAPSHOT_PAGE_ROWS,
            });
            for (const row of rows) {
                await this.#startChange(
                    store,
                    trigger,
                    {
                        position: { ...timeline, sequence },
                        operation: "create",
                        key: Key.of(table, row),
                        after: row,
                    },
                    at,
                );
            }
            last = rows.at(-1);
            if (rows.length < SNAPSHOT_PAGE_ROWS) {
                break;
            }
        }

        // follow the log after the snapshot
        await log.advance(reader, sequence, store.clock.now() + SLOT_MILLISECONDS);
        await log.drop(snapshotReader.name);

        return sequence;
    }

    /** Start a change's run once per change, however often the trigger looks at it. */
    async #startChange(
        store: ObjectStore,
        trigger: ChangeTrigger<Input>,
        change: ObjectChange,
        at: number,
    ): Promise<void> {
        const { name } = trigger;
        const event = { key: ChangeTrigger.key(change), at };
        await this.start(store, name, trigger.input(change), event, {});
    }

    /** Read the table of a trigger's object type, refusing another kind of declaration. */
    static #table(trigger: ChangeTrigger): Table {
        const object = trigger.change.object;
        if (!ObjectType.is(object)) {
            throw new TypeError(`trigger ${trigger.name} does not follow an object type`);
        }

        return object.table;
    }

    /** Classify a change by which of its images a condition matches. */
    static #operation(before: boolean, after: boolean): ChangeOperation | undefined {
        // update a row matching before and after
        if (before && after) {
            return "update";
        }
        // create a row entering the condition
        else if (after) {
            return "create";
        }
        // delete a row leaving the condition
        else if (before) {
            return "delete";
        }
        // ignore a row matching neither
        else {
            return undefined;
        }
    }

    /** See a change through a trigger's condition: entering rows created, leaving rows deleted. */
    static #see(
        operations: readonly ChangeOperation[],
        change: Change,
        timeline: Omit<LogPosition, "sequence">,
        match: Match | undefined,
    ): ObjectChange | undefined {
        // match each image and classify the change by the matching images
        const image = { before: Change.before(change), after: Change.after(change) };
        const before =
            image.before !== null &&
            (match === undefined || Predicate.matches(match, image.before));
        const after =
            image.after !== null && (match === undefined || Predicate.matches(match, image.after));
        const operation = TriggerServer.#operation(before, after);
        if (operation === undefined || !operations.includes(operation)) {
            return undefined;
        }

        return {
            position: { ...timeline, sequence: change.sequence },
            operation,
            ...(before && image.before !== null ? { before: image.before } : {}),
            ...(after && image.after !== null ? { after: image.after } : {}),
        };
    }
}

/** Parse a manager's value by its schema, refusing invalid input as the call's own. */
function parsed<Value>(validator: schema.Schema<Value>, value: unknown): Value {
    const result = validator.safeParse(value);
    if (!result.success) {
        throw refuseInput(value, result.error.issues);
    }

    return result.data;
}

/** Describe a skipped occurrence: missed in a gap, late, or superseded by a later one. */
function skip(at: number, isDue: boolean, earlier: number | "many"): ScheduleSkip {
    // stand for the occurrences omitted before the first one looked at
    if (earlier === "many" || earlier > 0) {
        return { at, reason: "missed", earlier };
    }
    // describe a late occurrence
    else if (!isDue) {
        return { at, reason: "late" };
    }
    // describe an occurrence a later one supersedes
    else {
        return { at, reason: "superseded" };
    }
}

/** Report whether a trigger fires on webhook deliveries. */
function isWebhook<Input>(trigger: Trigger<Input>): trigger is WebhookTrigger<Input> {
    return trigger.kind === "webhook";
}

/** Report whether a trigger fires on a schedule. */
function isSchedule<Input>(trigger: Trigger<Input>): trigger is ScheduleTrigger<Input> {
    return trigger.kind === "schedule";
}
