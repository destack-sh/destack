import {
    Change,
    type DatabaseConnection,
    type Row,
    type Table,
    DatabaseError,
    Key,
    Log,
    Condition,
    Namespace,
    Predicate,
    type Match,
} from "@destack/db";
import { found, schema } from "@destack/schema";
import type { Controller } from "@destack/runtime/control";
import {
    type ChangeOperation,
    type ChangeTrigger,
    type ObjectChange,
    type RunClient,
    RunEvent,
} from "@destack/runtime/trigger";
import { ObjectType } from "../object/object.ts";
import { Clock } from "@destack/environment";

/** The rows one snapshot page reads: a few hundred kilobytes. */
const SNAPSHOT_PAGE_ROWS = 500;

/** The runs recorded at once: 32 requests of a few ms hold 500 changes under 0.1 seconds. */
const SEND_CONCURRENCY = 32;

/** How often a change trigger looks again in its lag, advancing its slot: three times. */
const SLOT_ADVANCES = 3;

/** Follow served change triggers, recording each admitted change's call as a machine run. */
export class ChangeController implements Controller {
    /** The controller's name in reports. */
    readonly name = "change";
    /** The tables the triggers follow. */
    readonly watches: readonly Table[];
    /** The triggers followed side by side. */
    readonly concurrency: number;
    /** The database with the followed objects and their log. */
    readonly #database: DatabaseConnection;
    /** The machine recording the runs. */
    readonly #runs: RunClient;
    /** The triggers, by name. */
    readonly #followed: ReadonlyMap<string, ChangeTrigger>;
    /** Report a change a trigger passes over or loses. */
    readonly #report: (error: unknown) => void;
    /** The clock each change's lag deadline runs on. */
    readonly #clock: Clock;

    /** Follow change triggers over a database, recording runs in a machine. */
    constructor(
        database: DatabaseConnection,
        triggers: readonly ChangeTrigger[],
        runs: RunClient,
        report: (error: unknown) => void,
        clock: Clock = Clock.system,
    ) {
        // hold the database, the machine, the report and the clock
        this.#database = database;
        this.#runs = runs;
        this.#report = report;
        this.#clock = clock;

        // require each trigger to follow a served object type, under a name its package gives no other trigger
        const followed = new Map<string, ChangeTrigger>();
        for (const trigger of triggers) {
            const key = ChangeController.#key(trigger);
            if (!(trigger.on.change.object instanceof ObjectType)) {
                throw new TypeError(`trigger ${trigger.name} has no object type`);
            } else if (followed.has(key)) {
                throw new TypeError(
                    `${trigger.package.name} declares two triggers named ${trigger.name}`,
                );
            }
            followed.set(key, trigger);
        }
        this.#followed = followed;
        this.watches = [...new Set(triggers.map((trigger) => ChangeController.#table(trigger)))];
        this.concurrency = Math.max(1, triggers.length);
    }

    /** Select the triggers of a changed table. */
    keys(change: Change): readonly string[] {
        return [...this.#followed.values()]
            .filter((trigger) => ChangeController.#table(trigger) === change.table)
            .map((trigger) => ChangeController.#key(trigger));
    }

    /** List every trigger. */
    async list(): Promise<readonly string[]> {
        return [...this.#followed.keys()];
    }

    /** Record a trigger's admitted runs since its slot, advancing it before it lapses. */
    async reconcile(key: string): Promise<number> {
        // continue after the trigger's slot, or start it
        const trigger = found(this.#followed, key);
        const change = trigger.on.change;
        const table = ChangeController.#table(trigger);
        const log = new Log(this.#database);
        const slot = `trigger/${key}`;
        const epoch = await log.epoch();
        const match =
            change.where === undefined
                ? undefined
                : Predicate.compile(
                      Condition.resolve(change.where, Namespace.fields(table)),
                      table,
                  );
        let after = (await log.slot(slot)) ?? (await this.#start(trigger, slot, epoch));

        // record each admitted change page by page, advancing the slot after each page
        try {
            for (;;) {
                const page = await log.read({ tables: [table], after });
                const seen = page.changes.flatMap((logged) => {
                    const admitted = ChangeController.#see(trigger, logged, epoch, match);

                    return admitted === undefined
                        ? []
                        : [{ change: admitted, at: logged.changedAt }];
                });
                await this.#record(trigger, seen);
                after = Math.max(after, page.sequence);
                await log.advance(slot, after, this.#clock.now() + change.maxLag);
                if (page.changes.length === 0) {
                    break;
                }
            }
        } catch (error) {
            // continue at the head once the log compacted changes the trigger had not recorded, reporting the loss
            if (!(error instanceof DatabaseError && error.code === "CHANGES_COMPACTED")) {
                throw error;
            }
            const head = (await log.position()).sequence;
            await log.advance(slot, head, this.#clock.now() + change.maxLag);
            this.#report(
                new Error(`trigger ${trigger.name} lost the changes after ${after} to compaction`, {
                    cause: error,
                }),
            );
        }

        // look again well before the slot lapses, however quiet the table
        return Math.floor(change.maxLag / SLOT_ADVANCES);
    }

    /** Start a trigger at the log's head, first recording any snapshot rows it starts with. */
    async #start(trigger: ChangeTrigger, slot: string, epoch: string): Promise<number> {
        // start the slot at the head, or resume the snapshot begun at an earlier head
        const change = trigger.on.change;
        const log = new Log(this.#database);
        const snapshotSlot = `${slot}/snapshot`;
        const begun = change.from === "snapshot" ? await log.slot(snapshotSlot) : undefined;
        const sequence = begun ?? (await log.position()).sequence;
        if (change.from === "now") {
            await log.advance(slot, sequence, this.#clock.now() + change.maxLag);

            return sequence;
        }

        // record the snapshot's matching rows as created at the snapshot's reading, page by page
        const snapshot = log.at({ epoch, sequence });
        const at = this.#clock.now();
        const table = ChangeController.#table(trigger);
        let last: Row | undefined;
        for (;;) {
            await log.advance(snapshotSlot, sequence, this.#clock.now() + change.maxLag);
            const rows = await snapshot.ordered(table, {
                where: change.where ?? {},
                orderBy: { id: "asc" },
                ...(last === undefined ? {} : { after: last }),
                limit: SNAPSHOT_PAGE_ROWS,
            });
            const created = rows.map((row) => ({
                change: {
                    position: { epoch, sequence },
                    operation: "create" as const,
                    key: Key.of(table, row),
                    after: row,
                },
                at,
            }));
            await this.#record(trigger, created);
            last = rows.at(-1);
            if (rows.length < SNAPSHOT_PAGE_ROWS) {
                break;
            }
        }

        // follow the log after the snapshot
        await log.advance(slot, sequence, this.#clock.now() + change.maxLag);
        await log.drop(snapshotSlot);

        return sequence;
    }

    /** Record the runs of changes' calls once per change and snapshot row. */
    async #record(
        trigger: ChangeTrigger,
        changes: readonly { readonly change: ObjectChange; readonly at: number }[],
    ): Promise<void> {
        // build each change's request, reporting and passing over a change the trigger builds no call of
        const requests = changes.flatMap(({ change, at }) => {
            const key =
                change.key === undefined ? {} : { key: schema.string().parse(change.key["id"]) };
            try {
                const call = trigger.call(change);
                const event = { change: { position: change.position, ...key } };

                return [{ request: { call, triggerName: trigger.name, event }, at }];
            } catch (error) {
                this.#report(
                    new Error(
                        `trigger ${trigger.name} cannot build the call of change ${change.position.sequence}`,
                        { cause: error },
                    ),
                );

                return [];
            }
        });

        // send them several at a time in request order
        for (let start = 0; start < requests.length; start += SEND_CONCURRENCY) {
            const chunk = requests.slice(start, start + SEND_CONCURRENCY);
            await Promise.all(
                chunk.map(async ({ request, at }) => {
                    // send under the request derived from the trigger and the change, one however often sent
                    const requestId = await RunEvent.derive(
                        trigger.package.id,
                        trigger.name,
                        request.event,
                        at,
                    );
                    await this.#runs.send(request, { requestId });
                }),
            );
        }
    }

    /** Read a trigger's key: its package and its name. */
    static #key(trigger: ChangeTrigger): string {
        return `${trigger.package.id}/${trigger.name}`;
    }

    /** Read the table of a trigger's object type. */
    static #table(trigger: ChangeTrigger): Table {
        // refuse a trigger following another kind of declaration
        const object = trigger.on.change.object;
        if (!ObjectType.is(object)) {
            throw new TypeError(`trigger ${trigger.name} has no object type`);
        }

        return object.table;
    }

    /** See a change through a trigger's condition: entering rows created, leaving rows deleted. */
    static #see(
        trigger: ChangeTrigger,
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
        if (operation === undefined || !trigger.on.change.operations.includes(operation)) {
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
