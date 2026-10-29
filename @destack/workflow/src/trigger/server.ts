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
    Predicate,
    type Row,
    type Select,
    type Table,
    type LogPosition,
    Transaction,
} from "@destack/db";
import { ObjectType, type ObjectReconciliation, refuseInput, refusal } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import type { Controller, ReconcileController } from "@destack/db";
import { RequestId, type Call, Scope } from "@destack/sync";
import type { InstalledServerContext, WebhookHandler } from "@destack/object/workload";
import { Failure } from "@destack/error";
import { Manager } from "@destack/access";
import { canonicalize, present, schema } from "@destack/schema";
import { LIVE_STATES } from "../run/run.ts";
import type { TriggerType, Workflow } from "../workflow/workflow.ts";
import type { ChangeOperation, ObjectChange } from "./change.ts";
import { Schedule, ScheduleOn, ScheduleSkip } from "./schedule.ts";
import { ChangeTrigger, ScheduleTrigger, type Trigger, WebhookTrigger } from "./trigger.ts";
import { WebhookReceiver } from "./webhook.ts";
import type {} from "@destack/package/import-meta";
import { WorkflowError } from "../error/index.ts";

/** The parts of a workflow its triggers start runs through. */
type StartedWorkflow<Input> = Pick<
    Workflow<Input>,
    "name" | "package" | "scope" | "input" | "start" | "run" | "trigger" | "triggers"
>;

/** A webhook trigger as its installation serves it, with the receiver of its deliveries. */
interface ServedWebhook<Input> {
    /** The trigger. */
    readonly trigger: WebhookTrigger<Input>;
    /** The receiver verifying its deliveries for the installation. */
    readonly receiver: WebhookReceiver;
}

/** The most missed occurrences a schedule looks back over at once. */
const MAX_MISSED_OCCURRENCES = 100;

/** How long a trigger's slot retains changes after its last renewal: the change window, a week. */
const SLOT_MILLISECONDS = 7 * 24 * 60 * 60_000;

/** How often an active trigger renews its slot: daily, a seventh of its retention. */
const RENEWAL_MILLISECONDS = 24 * 60 * 60_000;

/** The rows one snapshot page reads: a few hundred kilobytes. */
const SNAPSHOT_PAGE_ROWS = 500;

/** The status of a delivery that started a run (RFC 9110 15.3.3). */
const ACCEPTED = 202;

/** A trigger's row as its workflow's trigger type stores it. */
type TriggerRecord = Select<TriggerType["table"]>;

/** A workflow's triggers as its server fires them: schedules, changes and webhooks. */
export class TriggerServer<Input> {
    /** The workflow the triggers start runs of. */
    readonly #workflow: StartedWorkflow<Input>;
    /** The declared triggers, by name. */
    readonly #declared: ReadonlyMap<string, Trigger<Input>>;
    /** The webhook triggers as the installation serves them, by name. */
    readonly #webhooks: ReadonlyMap<string, ServedWebhook<Input>>;

    /** Host a workflow's declared triggers, refusing repeated names and unserved webhooks. */
    constructor(
        workflow: StartedWorkflow<Input>,
        server: Pick<InstalledServerContext, "resources" | "keys" | "environment"> | undefined,
    ) {
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

        // build each webhook trigger's verifier once, refusing to serve webhooks without the installation's secrets
        const webhooks = [...declared.values()].filter((each) => isWebhook(each));
        const [first] = webhooks;
        if (first !== undefined && server === undefined) {
            throw new TypeError(
                `${workflow.name} serves webhook trigger ${first.name} without its installation's context`,
            );
        }
        this.#webhooks = new Map(
            server === undefined
                ? []
                : webhooks.map((trigger) => [
                      trigger.name,
                      {
                          trigger,
                          receiver: new WebhookReceiver(
                              trigger.webhook,
                              trigger.webhook.signature.verifier(server),
                              server,
                          ),
                      },
                  ]),
        );
    }

    /** Implement the trigger type: check created schedules and fire active ones. */
    implement(): ObjectType {
        const { trigger } = this.#workflow;

        return trigger.implement({
            methods: {
                create: (call, next) => {
                    // check the created schedule and its runs' input
                    const schedule = this.#checked(call.input.schedule, call.input.input);

                    return next(call.with({ input: { ...call.input, schedule } }));
                },
                update: (call, next) => {
                    // check the changed schedule and input
                    const { target, input } = call;
                    const schedule = this.#checked(
                        input.schedule ?? target.schedule,
                        input.input ?? target.input,
                    );

                    return next(call.with({ input: { ...input, schedule } }));
                },
            },
            controller: {
                pending: { state: "active", kind: "schedule" },
                reconcile: async (reconciliation) => {
                    // fire each schedule's due occurrences, looking again at the earliest next one
                    const wakes = await Promise.all(
                        reconciliation.rows.map((row) => this.#fireDue(row, reconciliation)),
                    );
                    const due = wakes.filter((wait) => wait !== undefined);

                    return due.length === 0 ? undefined : Math.min(...due);
                },
            },
        });
    }

    /** List the controllers recording the declared triggers' rows and following their changes. */
    controllers(): readonly ((store: ObjectStore) => Controller)[] {
        // follow the changes of each declared change trigger
        const changes = [...this.#declared.values()].filter(
            (trigger): trigger is ChangeTrigger<Input> => trigger.kind === "change",
        );

        return [
            (store) => new DeclarationController(store, this, [...this.#declared.values()]),
            ...changes.map(
                (trigger) => (store: ObjectStore) => new ChangeController(store, this, trigger),
            ),
        ];
    }

    /** Answer webhook deliveries by their trigger's name, starting one run per delivery. */
    webhooks(): (store: ObjectStore) => WebhookHandler {
        return (store) => ({
            fetch: async (request) => {
                try {
                    // find the active webhook trigger the first segment selects
                    const url = new URL(request.url);
                    const [, name = "", ...rest] = url.pathname.split("/");
                    const webhook = this.#webhooks.get(name);
                    if (webhook === undefined) {
                        throw WorkflowError.of(
                            "TRIGGER_NOT_FOUND",
                            `webhook trigger ${name} not found`,
                        );
                    }
                    const { trigger, receiver } = webhook;
                    await this.#requireActive(store, trigger.name);

                    // verify the delivery and start its run once per delivery
                    const delivery = await receiver.receive(
                        request,
                        `/${rest.join("/")}`,
                        store.clock.now(),
                    );
                    const event = { key: WebhookTrigger.key(delivery), at: delivery.receivedAt };
                    await this.start(store, trigger.name, trigger.input(delivery), event, {
                        key: `trigger/${trigger.name}/${event.key}`,
                    });

                    return new Response(null, { status: ACCEPTED });
                } catch (error) {
                    return refusal(error, store.environment.telemetry);
                }
            },
        });
    }

    /** Check a created schedule's timing and its runs' input, answering the checked schedule. */
    #checked(schedule: unknown, input: unknown): ScheduleOn {
        // check the timing and the input
        const checked = parsed(ScheduleOn, schedule);
        Schedule.require(checked.timing);
        parsed(this.#workflow.input, input);

        return checked;
    }

    /** Fire a schedule's due occurrences, answering the wait until its next one. */
    async #fireDue(
        row: TriggerRecord,
        reconciliation: ObjectReconciliation,
    ): Promise<number | undefined> {
        // find the latest occurrences due since the last one or within the deadline
        const { now, store } = reconciliation;
        const schedule = ScheduleOn.parse(row.schedule);
        const { timing, deadline } = schedule;
        const after = row.lastScheduledAt ?? now - deadline - 1;
        const occurrences = new Schedule(timing);
        const { due, earlier } = occurrences.recent(after, now, MAX_MISSED_OCCURRENCES);
        const last = due.at(-1);
        const next = occurrences.following(now);
        if (last === undefined) {
            return next === undefined ? undefined : next - now;
        }

        // skip the missed, late and superseded occurrences, counting the uncounted earlier ones on the first
        const isDue = (at: number) => now - at <= deadline;
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
                skipped: row.skipped + allSkips.length,
                lastSkip: latest ?? null,
            }),
        ]);

        return next === undefined ? undefined : next - now;
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
        // derive the request from the trigger's slot and the event, stamped at the event's time
        const workflow = this.#workflow;
        const id = await RequestId.derive(
            event.at,
            `${this.slot(name)} ${event.key}`,
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

    /** Read a trigger's identity, which names its log slot and derives its requests. */
    slot(name: string): string {
        return `trigger/${this.#workflow.package.id}/${this.#workflow.name}/${name}`;
    }

    /** Read the log slot of a change trigger's first snapshot, paged through rows. */
    snapshotSlot(name: string): string {
        return `${this.slot(name)}/snapshot`;
    }

    /** Drop a change trigger's log slots, retaining no more changes for it. */
    async dropSlots(database: DatabaseConnection, name: string): Promise<void> {
        const log = new Log(database);
        await log.drop(this.slot(name));
        await log.drop(this.snapshotSlot(name));
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
}

/** The controller recording the rows of a workflow's declared triggers in the store's space. */
class DeclarationController<Input> implements ReconcileController {
    /** The controller's name in reports. */
    readonly name: string;
    /** The tables whose changes select keys: none, as declarations change with releases. */
    readonly watches: readonly Table[];
    /** The store the rows live in. */
    readonly #store: ObjectStore;
    /** The server of the triggers. */
    readonly #server: TriggerServer<Input>;
    /** The declared triggers. */
    readonly #triggers: readonly Trigger<Input>[];

    /** Record a workflow's declared triggers in a store's space. */
    constructor(
        store: ObjectStore,
        server: TriggerServer<Input>,
        triggers: readonly Trigger<Input>[],
    ) {
        // name the controller after the trigger type
        this.name = `${server.workflow.trigger.name}:declared`;
        this.watches = [];
        this.#store = store;
        this.#server = server;
        this.#triggers = triggers;
    }

    /** Select no keys from changes. */
    keys(): readonly string[] {
        return [];
    }

    /** List the workflow's space as the one key. */
    async list(): Promise<readonly string[]> {
        return [this.#server.scope(this.#store)];
    }

    /** Create missing declared rows, update changed schedules, retire undeclared attached rows. */
    async reconcile(scope: string): Promise<undefined> {
        // read the rows the workflow's installation declared in the space, detached ones too
        const { trigger } = this.#server.workflow;
        const served = present(this.#store.readServer(trigger), `the server of ${trigger.name}`);
        const manager = { installationId: served.installation, packageId: trigger.package.id };
        const column = (name: string) => trigger.table[TABLE].column(name);
        const rows = await this.#store.database
            .select()
            .from(trigger.table)
            .where(
                and(
                    eq(column("scope"), scope),
                    eq(column("managerInstallationId"), manager.installationId),
                    eq(column("managerPackageId"), manager.packageId),
                ),
            );

        // create each missing declared trigger, and update an attached one's schedule that changed
        const calls: (readonly [string, Call])[] = [];
        for (const declared of this.#triggers) {
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
                !this.#triggers.some((declared) => declared.name === each.name),
        );
        calls.push(
            ...retired.map(
                (row) => [row.name, trigger.call("retire", trigger.target(row))] as const,
            ),
        );
        if (calls.length === 0) {
            return undefined;
        }

        // record each call as the trigger's manager, dropping the retired triggers' log slots in the same commit
        await Transaction.run(this.#store.database, async (transaction) => {
            for (const [name, call] of calls) {
                await this.#store.mutate(
                    this.#store.as(trigger),
                    { id: RequestId.create(this.#store.environment), calls: [call] },
                    {
                        scope,
                        transaction,
                        manager: { ...manager, name: `${trigger.collection}/${name}` },
                    },
                );
            }
            for (const row of retired) {
                transaction.record((committing) => this.#server.dropSlots(committing, row.name));
            }
        });

        return undefined;
    }
}

/** Follow a change trigger's object changes in the log, starting one run per matching change. */
class ChangeController<Input> implements ReconcileController {
    /** The controller's name in reports. */
    readonly name: string;
    /** The changed object's table, and the triggers' table, whose row activates this one. */
    readonly watches: readonly Table[];
    /** The store the runs start in. */
    readonly #store: ObjectStore;
    /** The server of the trigger. */
    readonly #server: TriggerServer<Input>;
    /** The followed trigger. */
    readonly #trigger: ChangeTrigger<Input>;

    /** Follow one change trigger over a store's database. */
    constructor(store: ObjectStore, server: TriggerServer<Input>, trigger: ChangeTrigger<Input>) {
        // name the controller after the trigger type and the trigger
        const { workflow } = server;
        this.name = `${workflow.trigger.name}:${trigger.name}`;
        this.watches = [ChangeController.#table(trigger), workflow.trigger.table];
        this.#store = store;
        this.#server = server;
        this.#trigger = trigger;
    }

    /** Select the trigger for a change of its table, or of its own row activating or pausing it. */
    keys(change: Change): readonly string[] {
        const isFollowed = change.table === ChangeController.#table(this.#trigger);
        const isOwn =
            Change.of(change, this.#server.workflow.trigger.table) &&
            Change.image(change)["name"] === this.#trigger.name;

        return isFollowed || isOwn ? [this.#trigger.name] : [];
    }

    /** List the trigger as the one key. */
    async list(): Promise<readonly string[]> {
        return [this.#trigger.name];
    }

    /** Start the runs of the matching changes since the trigger's slot, renewing it daily. */
    async reconcile(): Promise<number | undefined> {
        // wait for the trigger's row to resume or reset it while it is not active
        const change = this.#trigger.change;
        const database = this.#store.database;
        const scope = this.#server.scope(this.#store);
        const row = await this.#server.row(database, scope, this.#trigger.name);
        if (row?.state !== "active") {
            return undefined;
        }

        // continue after the trigger's slot, or start it
        const table = ChangeController.#table(this.#trigger);
        const log = new Log(database);
        const slot = this.#server.slot(this.#trigger.name);
        const position = await log.position();
        const timeline = { log: position.log, epoch: position.epoch };
        const match =
            change.where === undefined
                ? undefined
                : Predicate.compile(
                      Condition.resolve(change.where, Namespace.fields(table)),
                      table,
                  );
        const placed = await log.slot(slot);

        // start each matching change's run page by page, advancing the slot past each page of changes
        try {
            let after = placed ?? (await this.#begin(slot, position));
            let isRenewed = placed === undefined;
            for (;;) {
                const page = await log.read({ tables: [table], after });
                for (const logged of page.changes) {
                    const matching = ChangeController.#see(
                        change.operations,
                        logged,
                        timeline,
                        match,
                    );
                    if (matching !== undefined) {
                        await this.#start(matching, logged.changedAt);
                    }
                }
                after = Math.max(after, page.sequence);

                // advance past a page of changes, renewing the slot once on a look that found none
                if (page.changes.length > 0 || !isRenewed) {
                    await log.advance(slot, after, this.#store.clock.now() + SLOT_MILLISECONDS);
                    isRenewed = true;
                }
                if (page.changes.length === 0) {
                    break;
                }
            }
        } catch (error) {
            // stop the trigger on changes compacted past its slot until a manager resets it
            if (!DatabaseError.is(error, "CHANGES_COMPACTED")) {
                throw error;
            }
            await this.#stop(row, error);

            return undefined;
        }

        // look again within a day, renewing the slot while no change arrives
        return RENEWAL_MILLISECONDS;
    }

    /** Stop a trigger that lost changes to compaction, dropping its slots for a fresh reset. */
    async #stop(row: TriggerRecord, cause: unknown): Promise<void> {
        // record why the trigger stopped
        const { name } = this.#trigger;
        const { trigger } = this.#server.workflow;
        const failure = Failure.of(
            WorkflowError.of("CHANGES_COMPACTED", `trigger ${name} lost changes to compaction`, {
                cause,
            }),
        );

        // record the stop, dropping the slots in the same commit
        await Transaction.run(this.#store.database, async (transaction) => {
            await this.#store.mutate(
                this.#store.as(trigger),
                {
                    id: RequestId.create(this.#store.environment),
                    calls: [trigger.call("stop", { ...trigger.target(row), failure })],
                },
                { scope: row.scope, transaction },
            );
            transaction.record((committing) => this.#server.dropSlots(committing, name));
        });
    }

    /** Start the slot at the log's head, first starting the runs of a snapshot's matching rows. */
    async #begin(slot: string, position: LogPosition): Promise<number> {
        // start the slot at the head, or resume the snapshot begun at an earlier head
        const change = this.#trigger.change;
        const log = new Log(this.#store.database);
        const snapshotSlot = this.#server.snapshotSlot(this.#trigger.name);
        const begun = change.from === "snapshot" ? await log.slot(snapshotSlot) : undefined;
        const sequence = begun ?? position.sequence;
        const timeline = { log: position.log, epoch: position.epoch };
        if (change.from === "now") {
            await log.advance(slot, sequence, this.#store.clock.now() + SLOT_MILLISECONDS);

            return sequence;
        }

        // start a run for each matching row as created at the snapshot's reading, page by page
        const snapshot = log.at({ ...timeline, sequence });
        const at = this.#store.clock.now();
        const table = ChangeController.#table(this.#trigger);
        let last: Row | undefined;
        for (;;) {
            // keep the snapshot's slot for a week past each page
            await log.advance(snapshotSlot, sequence, this.#store.clock.now() + SLOT_MILLISECONDS);
            const rows = await snapshot.ordered(table, {
                where: change.where ?? {},
                orderBy: { id: "asc" },
                ...(last === undefined ? {} : { after: last }),
                limit: SNAPSHOT_PAGE_ROWS,
            });
            for (const row of rows) {
                await this.#start(
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
        await log.advance(slot, sequence, this.#store.clock.now() + SLOT_MILLISECONDS);
        await log.drop(snapshotSlot);

        return sequence;
    }

    /** Start a change's run once per change, however often the trigger looks at it. */
    async #start(change: ObjectChange, at: number): Promise<void> {
        const { name } = this.#trigger;
        const event = { key: ChangeTrigger.key(change), at };
        await this.#server.start(this.#store, name, this.#trigger.input(change), event, {});
    }

    /** Read the table of a trigger's object type, refusing another kind of declaration. */
    static #table<Input>(trigger: ChangeTrigger<Input>): Table {
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
        const operation = ChangeController.#operation(before, after);
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
