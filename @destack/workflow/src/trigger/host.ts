import { principal } from "@destack/access";
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
    type Table,
} from "@destack/db";
import { type MethodContext, ObjectType, type ObjectReconciliation } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import type { Controller } from "@destack/runtime/control";
import { CallError, refuseInput } from "@destack/runtime/error";
import { RequestId } from "@destack/runtime/request";
import { refusal } from "@destack/runtime/server";
import type { ServerContext, WebhookHandler } from "@destack/runtime/workload";
import { canonicalize, schema } from "@destack/schema";
import type { Call, Subject } from "@destack/sync";
import { LIVE_STATES } from "../run/run.ts";
import type { Workflow } from "../workflow/workflow.ts";
import type { ChangeOperation, ObjectChange } from "./change.ts";
import { Schedule, ScheduleOn, ScheduleSkip } from "./schedule.ts";
import {
    type ChangeTrigger,
    type ScheduleTrigger,
    Trigger,
    type TriggerEvent,
    TRIGGER_KINDS,
    type WebhookTrigger,
} from "./trigger.ts";
import { WebhookOn, type WebhookVerifier } from "./webhook.ts";

/** The parts of a workflow its triggers start runs through. */
type StartedWorkflow<Input> = Pick<
    Workflow<Input>,
    "name" | "package" | "input" | "start" | "run" | "trigger" | "triggers"
>;

/** The most missed occurrences a schedule looks back over at once. */
const MAX_MISSED_OCCURRENCES = 100;

/** The rows one snapshot page reads: a few hundred kilobytes. */
const SNAPSHOT_PAGE_ROWS = 500;

/** How often a change trigger looks again in its lag, advancing its slot: three times. */
const SLOT_ADVANCES = 3;

/** The status of a delivery that started a run (RFC 9110 15.3.3). */
const ACCEPTED = 202;

/** A trigger's row as firing it reads it. */
const TriggerRecord = schema.looseObject({
    /** The trigger. */
    id: schema.string(),
    /** The space it fires in. */
    scope: schema.string(),
    /** Its name. */
    name: schema.string(),
    /** Whether its package declares it or a manager created it. */
    origin: schema.enum(["declared", "created"]),
    /** The kind of event it fires on. */
    kind: schema.enum(TRIGGER_KINDS),
    /** Its schedule. */
    schedule: ScheduleOn.nullable(),
    /** A created schedule's run input. */
    input: schema.json().nullable(),
    /** Whether it fires or is paused. */
    state: schema.enum(["active", "paused"]),
    /** The latest occurrence fired or skipped. */
    lastScheduledAt: schema.number().nullable(),
    /** The occurrences skipped so far. */
    skipped: schema.number().int(),
});
/** A trigger's row as firing it reads it. */
type TriggerRecord = schema.Infer<typeof TriggerRecord>;

/** A workflow's triggers as its server fires them: schedules, changes and webhooks. */
export class TriggerHost<Input> {
    /** The workflow the triggers start runs of. */
    readonly #workflow: StartedWorkflow<Input>;
    /** The declared triggers, by name. */
    readonly #declared: ReadonlyMap<string, Trigger<Input>>;
    /** The installation's resources and keys, the source of webhook secrets. */
    readonly #server: Pick<ServerContext, "resources" | "keys"> | undefined;
    /** The verifier of each webhook trigger's deliveries, for the installation served. */
    readonly #verifiers: ReadonlyMap<string, WebhookVerifier>;

    /** Host a workflow's declared triggers, refusing a repeated name. */
    constructor(
        workflow: StartedWorkflow<Input>,
        server: Pick<ServerContext, "resources" | "keys"> | undefined,
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
        this.#server = server;

        // build each webhook trigger's verifier for the installation, once
        this.#verifiers = new Map(
            server === undefined
                ? []
                : [...declared.values()]
                      .filter((each) => isWebhook(each))
                      .map((each) => [each.name, each.webhook.signature.verifier(server)]),
        );
    }

    /** Implement the trigger type: check created schedules and fire active ones. */
    implement(): ObjectType {
        const { trigger } = this.#workflow;

        return trigger.implement({
            methods: {
                create: (call, next) => this.#requireCreated(call, next),
                update: (call, next) => this.#requireCreated(call, next),
                delete: (call, next) => this.#requireCreated(call, next),
            },
            controller: {
                pending: { state: "active", kind: "schedule" },
                reconcile: async (reconciliation) => {
                    // fire each schedule's due occurrences, looking again at the earliest next one
                    const wakes = await Promise.all(
                        reconciliation.rows.map((row) =>
                            this.#fireDue(TriggerRecord.parse(row), reconciliation),
                        ),
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
            (store) => new TriggerDeclaration(store, this.#workflow, [...this.#declared.values()]),
            ...changes.map(
                (trigger) => (store: ObjectStore) =>
                    new ChangeController(store, this.#workflow, trigger),
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
                    const found = this.#declared.get(name);
                    const trigger = found !== undefined && isWebhook(found) ? found : undefined;
                    if (trigger === undefined) {
                        throw new CallError("NOT_FOUND", { message: `no webhook trigger ${name}` });
                    }
                    await requireActive(store, this.#workflow, trigger.name);

                    // verify the delivery and start its run once per delivery
                    const delivery = await WebhookOn.receive(
                        trigger.webhook,
                        requireVerifier(this.#verifiers, trigger.name),
                        request,
                        `/${rest.join("/")}`,
                        store.clock.now(),
                        requireServer(this.#server, trigger.name),
                    );
                    await start(
                        store,
                        this.#workflow,
                        trigger.name,
                        trigger.input(delivery),
                        delivery,
                        {
                            key: `trigger/${trigger.name}/${delivery.id}`,
                        },
                    );

                    return new Response(null, { status: ACCEPTED });
                } catch (error) {
                    return refusal(error);
                }
            },
        });
    }

    /** Refuse a manager changing a declared trigger, and check a created schedule and its input. */
    async #requireCreated(context: MethodContext, next: () => Promise<unknown>): Promise<unknown> {
        // refuse changing or deleting a declared trigger
        const target =
            context.target === undefined ? undefined : TriggerRecord.parse(context.target);
        if (target?.origin === "declared") {
            throw new CallError("CONFLICT", {
                message: `trigger ${target.name} is declared by its package, which changes it`,
            });
        }

        // check a created schedule's timing and its runs' input
        const input = schema.looseObject({}).parse(context.input);
        const schedule = input["schedule"] ?? target?.schedule;
        if (context.name !== "delete") {
            Schedule.require(parsed(ScheduleOn, schedule).timing);
            parsed(this.#workflow.input, input["input"] ?? target?.input);
        }

        return next();
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
        const { due, earlier } = Schedule.recent(timing, after, now, MAX_MISSED_OCCURRENCES);
        const last = due.at(-1);
        const next = Schedule.following(timing, now);
        if (last === undefined) {
            return next === undefined ? undefined : next - now;
        }

        // skip the missed, late and superseded occurrences, the first carrying the uncounted earlier ones
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
                schedule,
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
                      .where(liveRunOf(workflow, row.scope, key));
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
        const declared = this.#declared.get(row.name);
        const input =
            row.origin === "created"
                ? workflow.input.parse(row.input)
                : requireSchedule(declared, row.name).input({ at });
        await start(
            store,
            workflow,
            row.name,
            input,
            { at },
            { key, before: cancel, scope: row.scope },
        );

        return undefined;
    }
}

/** The controller recording the rows of a workflow's declared triggers in the store's space. */
class TriggerDeclaration<Input> implements Controller {
    /** The controller's name in reports. */
    readonly name: string;
    /** The tables whose changes select keys: none, as declarations change with releases. */
    readonly watches: readonly Table[] = [];
    /** The store the rows live in. */
    readonly #store: ObjectStore;
    /** The workflow the triggers start. */
    readonly #workflow: StartedWorkflow<Input>;
    /** The declared triggers. */
    readonly #triggers: readonly Trigger<Input>[];

    /** Record a workflow's declared triggers in a store's space. */
    constructor(
        store: ObjectStore,
        workflow: StartedWorkflow<Input>,
        triggers: readonly Trigger<Input>[],
    ) {
        // name the controller after the trigger type
        this.name = `${workflow.trigger.name}:declared`;
        this.#store = store;
        this.#workflow = workflow;
        this.#triggers = triggers;
    }

    /** Select no keys from changes. */
    keys(): readonly string[] {
        return [];
    }

    /** List the workflow's space as the one key. */
    async list(): Promise<readonly string[]> {
        return [spaceOf(this.#store.serving(this.#workflow.run).subject)];
    }

    /** Create missing declared rows, update changed schedules and retire undeclared rows. */
    async reconcile(scope: string): Promise<undefined> {
        // read the declared rows the space has
        const { trigger } = this.#workflow;
        const column = (name: string) => trigger.table[TABLE].column(name);
        const rows = (
            await this.#store.database
                .select()
                .from(trigger.table)
                .where(and(eq(column("scope"), scope), eq(column("origin"), "declared")))
        ).map((row) => TriggerRecord.parse(row));

        // create each missing declared trigger, and update a declared schedule that changed
        const calls: Call[] = [];
        for (const declared of this.#triggers) {
            const { kind } = declared;
            const schedule = declared.kind === "schedule" ? declared.schedule : undefined;
            const row = rows.find((each) => each.name === declared.name);
            if (row === undefined) {
                calls.push(
                    trigger.call("declare", {
                        ...trigger.in(scope),
                        name: declared.name,
                        origin: "declared",
                        kind,
                        ...(schedule === undefined ? {} : { schedule }),
                    }),
                );
            } else if (
                schedule !== undefined &&
                canonicalize(schedule) !== canonicalize(row.schedule)
            ) {
                calls.push(
                    trigger.call("record", {
                        ...trigger.target(row),
                        lastScheduledAt: row.lastScheduledAt,
                        skipped: row.skipped,
                        lastSkip: null,
                        schedule,
                    }),
                );
            }
        }

        // retire the declared rows no declaration has any more
        for (const row of rows.filter(
            (each) => !this.#triggers.some((declared) => declared.name === each.name),
        )) {
            calls.push(trigger.call("retire", trigger.target(row)));
        }
        if (calls.length > 0) {
            await this.#store.mutate(
                this.#store.serving(trigger),
                { id: RequestId.create(this.#store.clock.now()), calls },
                { scope },
            );
        }

        return undefined;
    }
}

/** Follow a change trigger's object changes in the log, starting one run per matching change. */
class ChangeController<Input> implements Controller {
    /** The controller's name in reports. */
    readonly name: string;
    /** The changed object's table. */
    readonly watches: readonly Table[];
    /** The store the runs start in. */
    readonly #store: ObjectStore;
    /** The workflow the trigger starts. */
    readonly #workflow: StartedWorkflow<Input>;
    /** The followed trigger. */
    readonly #trigger: ChangeTrigger<Input>;

    /** Follow one change trigger over a store's database. */
    constructor(
        store: ObjectStore,
        workflow: StartedWorkflow<Input>,
        trigger: ChangeTrigger<Input>,
    ) {
        // name the controller after the trigger type and the trigger
        this.name = `${workflow.trigger.name}:${trigger.name}`;
        this.watches = [ChangeController.#table(trigger)];
        this.#store = store;
        this.#workflow = workflow;
        this.#trigger = trigger;
    }

    /** Select the trigger for a change of its table. */
    keys(change: Change): readonly string[] {
        return change.table === ChangeController.#table(this.#trigger) ? [this.#trigger.name] : [];
    }

    /** List the trigger as the one key. */
    async list(): Promise<readonly string[]> {
        return [this.#trigger.name];
    }

    /** Start the runs of the matching changes since the trigger's slot, advancing it. */
    async reconcile(): Promise<number> {
        // look again later while the trigger is paused
        const change = this.#trigger.change;
        const database = this.#store.database;
        const scope = spaceOf(this.#store.serving(this.#workflow.run).subject);
        if (!(await isActive(database, this.#workflow, scope, this.#trigger.name))) {
            return Math.floor(change.maxLag / SLOT_ADVANCES);
        }

        // continue after the trigger's slot, or start it
        const table = ChangeController.#table(this.#trigger);
        const log = new Log(database);
        const slot = `trigger/${this.#workflow.package.id}/${this.#workflow.name}/${this.#trigger.name}`;
        const epoch = await log.epoch();
        const match =
            change.where === undefined
                ? undefined
                : Predicate.compile(
                      Condition.resolve(change.where, Namespace.fields(table)),
                      table,
                  );
        let after = (await log.slot(slot)) ?? (await this.#begin(slot, epoch));

        // start each matching change's run page by page, advancing the slot after each page
        try {
            for (;;) {
                const page = await log.read({ tables: [table], after });
                for (const logged of page.changes) {
                    const matching = ChangeController.#see(change.operations, logged, epoch, match);
                    if (matching !== undefined) {
                        await this.#start(matching, logged.changedAt);
                    }
                }
                after = Math.max(after, page.sequence);
                await log.advance(slot, after, this.#store.clock.now() + change.maxLag);
                if (page.changes.length === 0) {
                    break;
                }
            }
        } catch (error) {
            // continue at the head once the log compacted changes the trigger had not taken, failing loudly
            if (!(error instanceof DatabaseError && error.code === "CHANGES_COMPACTED")) {
                throw error;
            }
            const head = (await log.position()).sequence;
            await log.advance(slot, head, this.#store.clock.now() + change.maxLag);
            throw new Error(
                `trigger ${this.#trigger.name} lost the changes after ${after} to compaction`,
                {
                    cause: error,
                },
            );
        }

        // look again well before the slot lapses, however quiet the table
        return Math.floor(change.maxLag / SLOT_ADVANCES);
    }

    /** Start the slot at the log's head, first starting the runs of a snapshot's matching rows. */
    async #begin(slot: string, epoch: string): Promise<number> {
        // start the slot at the head, or resume the snapshot begun at an earlier head
        const change = this.#trigger.change;
        const log = new Log(this.#store.database);
        const snapshotSlot = `${slot}/snapshot`;
        const begun = change.from === "snapshot" ? await log.slot(snapshotSlot) : undefined;
        const sequence = begun ?? (await log.position()).sequence;
        if (change.from === "now") {
            await log.advance(slot, sequence, this.#store.clock.now() + change.maxLag);

            return sequence;
        }

        // start a run for each matching row as created at the snapshot's reading, page by page
        const snapshot = log.at({ epoch, sequence });
        const at = this.#store.clock.now();
        const table = ChangeController.#table(this.#trigger);
        let last: Row | undefined;
        for (;;) {
            await log.advance(snapshotSlot, sequence, this.#store.clock.now() + change.maxLag);
            const rows = await snapshot.ordered(table, {
                where: change.where ?? {},
                orderBy: { id: "asc" },
                ...(last === undefined ? {} : { after: last }),
                limit: SNAPSHOT_PAGE_ROWS,
            });
            for (const row of rows) {
                await this.#start(
                    {
                        position: { epoch, sequence },
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
        await log.advance(slot, sequence, this.#store.clock.now() + change.maxLag);
        await log.drop(snapshotSlot);

        return sequence;
    }

    /** Start a change's run once per change, however often the trigger looks at it. */
    async #start(change: ObjectChange, at: number): Promise<void> {
        await start(
            this.#store,
            this.#workflow,
            this.#trigger.name,
            this.#trigger.input(change),
            change,
            {
                time: at,
            },
        );
    }

    /** Read the table of a trigger's object type, refusing another kind of declaration. */
    static #table<Input>(trigger: ChangeTrigger<Input>): Table {
        const object = trigger.change.object;
        if (!ObjectType.is(object)) {
            throw new TypeError(`trigger ${trigger.name} follows no object type`);
        }

        return object.table;
    }

    /** See a change through a trigger's condition: entering rows created, leaving rows deleted. */
    static #see(
        operations: readonly ChangeOperation[],
        change: Change,
        epoch: string,
        match: Match | undefined,
    ): ObjectChange | undefined {
        // match each image and classify the change by the matching images
        const image = { before: Change.before(change), after: Change.after(change) };
        const before =
            image.before !== null &&
            (match === undefined || Predicate.matches(match, image.before));
        const after =
            image.after !== null && (match === undefined || Predicate.matches(match, image.after));
        const operation: ChangeOperation | undefined =
            before && after ? "update" : after ? "create" : before ? "delete" : undefined;
        if (operation === undefined || !operations.includes(operation)) {
            return undefined;
        }

        return {
            position: { epoch, sequence: change.sequence },
            operation,
            ...(before && image.before !== null ? { before: image.before } : {}),
            ...(after && image.after !== null ? { after: image.after } : {}),
        };
    }
}

/** Start a trigger's run for an event once, under a request the trigger and the event derive. */
async function start<Input>(
    store: ObjectStore,
    workflow: StartedWorkflow<Input>,
    name: string,
    input: Input,
    event: TriggerEvent,
    options: {
        readonly key?: string;
        readonly before?: readonly Call[];
        readonly scope?: string;
        readonly time?: number;
    },
): Promise<void> {
    // derive the request from the trigger and the event, stamped at the event's time
    const time =
        options.time ??
        ("at" in event ? event.at : "receivedAt" in event ? event.receivedAt : store.clock.now());
    const id = await RequestId.derive(time, `${workflow.package.id}/${name} ${Trigger.key(event)}`);
    const self = store.serving(workflow.run);
    const run = workflow.start(input, {
        actor: self.subject,
        ...(options.key === undefined ? {} : { key: options.key }),
    });
    await store.mutate(
        self,
        { id, calls: [...(options.before ?? []), run] },
        { scope: options.scope ?? spaceOf(store.serving(workflow.run).subject) },
    );
}

/** Read whether a workflow's declared trigger is active in a space. */
async function isActive<Input>(
    database: DatabaseConnection,
    workflow: StartedWorkflow<Input>,
    scope: string,
    name: string,
): Promise<boolean> {
    // read the trigger by its unique name in the space
    const column = (field: string) => workflow.trigger.table[TABLE].column(field);
    const [row] = await database
        .select()
        .from(workflow.trigger.table)
        .where(and(eq(column("scope"), scope), eq(column("name"), name)));

    return row !== undefined && TriggerRecord.parse(row).state === "active";
}

/** Refuse a delivery to a paused or unrecorded trigger, which its sender delivers again. */
async function requireActive<Input>(
    store: ObjectStore,
    workflow: StartedWorkflow<Input>,
    name: string,
): Promise<void> {
    if (
        !(await isActive(
            store.database,
            workflow,
            spaceOf(store.serving(workflow.run).subject),
            name,
        ))
    ) {
        throw new CallError("SERVICE_UNAVAILABLE", { message: `trigger ${name} is paused` });
    }
}

/** Require the installation's resources and keys a webhook's secret comes from. */
function requireServer(
    server: Pick<ServerContext, "resources" | "keys"> | undefined,
    name: string,
): Pick<ServerContext, "resources" | "keys"> {
    if (server === undefined) {
        throw new TypeError(`webhook trigger ${name} is served without its installation's context`);
    }

    return server;
}

/** Require a webhook trigger's verifier, built once the installation's context is known. */
function requireVerifier(
    verifiers: ReadonlyMap<string, WebhookVerifier>,
    name: string,
): WebhookVerifier {
    const verifier = verifiers.get(name);
    if (verifier === undefined) {
        throw new TypeError(`webhook trigger ${name} is served without its installation's context`);
    }

    return verifier;
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

/** Match a workflow's live run of a key in a space. */
function liveRunOf<Input>(workflow: StartedWorkflow<Input>, scope: string, key: string) {
    const column = (name: string) => workflow.run.table[TABLE].column(name);

    return and(
        eq(column("scope"), scope),
        eq(column("key"), key),
        inArray(column("state"), [...LIVE_STATES]),
    );
}

/** Report whether a trigger fires on webhook deliveries. */
function isWebhook<Input>(trigger: Trigger<Input>): trigger is WebhookTrigger<Input> {
    return trigger.kind === "webhook";
}

/** Require a declared trigger to fire on a schedule. */
function requireSchedule<Input>(
    trigger: Trigger<Input> | undefined,
    name: string,
): ScheduleTrigger<Input> {
    if (trigger === undefined || !isSchedule(trigger)) {
        throw new TypeError(`declared trigger ${name} fires on no schedule`);
    }

    return trigger;
}

/** Report whether a trigger fires on a schedule. */
function isSchedule<Input>(trigger: Trigger<Input>): trigger is ScheduleTrigger<Input> {
    return trigger.kind === "schedule";
}

/** Read the space a serving principal acts in: an installation's scope, else the space itself. */
function spaceOf(subject: Subject): string {
    return principal.installation.is(subject) ? subject.scope : subject.id;
}
