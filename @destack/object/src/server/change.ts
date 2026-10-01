import type { DatabaseConnection, Row, Table } from "@destack/db";
import { DatabaseError } from "@destack/db/error";
import { Log, type Change } from "@destack/db/log";
import { Condition, type Match } from "@destack/db/query";
import type { Controller } from "@destack/service/control";
import type {
    ChangeOperation,
    ChangeTrigger,
    ObjectChange,
    RunClient,
} from "@destack/service/trigger";
import { ObjectType } from "../object/object.ts";

/** The rows one snapshot page reads: a few hundred kilobytes. */
const SNAPSHOT_PAGE_ROWS = 500;

/** The runs recorded at once: 32 requests of a few milliseconds keep a page of 500 changes under a tenth of a second. */
const SEND_CONCURRENCY = 32;

/** How often a change trigger looks again within its lag, advancing its slot however quiet its table: three times. */
const SLOT_ADVANCES = 3;

/** Follow the change triggers of served objects, recording the call of each change they admit as a run in the space's cell. */
export class ChangeController implements Controller {
    /** The controller's name in reports. */
    readonly name = "change";
    /** The tables the triggers follow. */
    readonly watches: readonly Table[];
    /** The triggers followed side by side. */
    readonly concurrency: number;
    /** The database keeping the followed objects and their log. */
    readonly #database: DatabaseConnection;
    /** The cell recording the runs. */
    readonly #runs: RunClient;
    /** The triggers, by name. */
    readonly #followed: ReadonlyMap<string, ChangeTrigger>;
    /** Report a change a trigger passes over or loses. */
    readonly #report: (error: unknown) => void;
    /** Read the current time. */
    readonly #now: () => number;

    /** Follow change triggers over a database, recording runs in a cell. */
    constructor(
        database: DatabaseConnection,
        triggers: readonly ChangeTrigger[],
        runs: RunClient,
        report: (error: unknown) => void,
        now: () => number = Date.now,
    ) {
        // keep the database, the cell, the report and the clock
        this.#database = database;
        this.#runs = runs;
        this.#report = report;
        this.#now = now;

        // require each trigger to follow a served object type, under a name its package gives no other trigger
        const followed = new Map<string, ChangeTrigger>();
        for (const trigger of triggers) {
            const key = ChangeController.#key(trigger);
            if (!(trigger.on.change.object instanceof ObjectType)) {
                throw new TypeError(`trigger ${trigger.name} follows no object type`);
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

    /** Record the runs of a trigger's admitted changes since its slot, advance the slot past them, and look again before the slot lapses. */
    async reconcile(key: string): Promise<number> {
        // continue after the trigger's slot, or start it
        const trigger = this.#followed.get(key)!;
        const change = trigger.on.change;
        const table = ChangeController.#table(trigger);
        const log = new Log(this.#database);
        const slot = `trigger/${key}`;
        const epoch = await log.epoch();
        const match = change.where && Condition.compile(change.where, table);
        let after = (await log.slot(slot)) ?? (await this.#start(trigger, slot, epoch));

        // record each admitted change page by page, advancing the slot after each page
        try {
            for (;;) {
                const page = await log.read({ tables: [table], after });
                const seen = page.changes.flatMap(
                    (logged) => ChangeController.#see(trigger, logged, epoch, match) ?? [],
                );
                await this.#record(trigger, seen);
                after = Math.max(after, page.sequence);
                await log.advance(slot, after, this.#now() + change.maxLag);
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
            await log.advance(slot, head, this.#now() + change.maxLag);
            this.#report(
                new Error(`trigger ${trigger.name} lost the changes after ${after} to compaction`, {
                    cause: error,
                }),
            );
        }

        // look again well before the slot lapses, however quiet the table
        return Math.floor(change.maxLag / SLOT_ADVANCES);
    }

    /** Start a trigger at the log's head, recording its snapshot's rows first when it starts with them. */
    async #start(trigger: ChangeTrigger, slot: string, epoch: string): Promise<number> {
        // start the slot at the head, or resume the snapshot begun at an earlier head
        const change = trigger.on.change;
        const log = new Log(this.#database);
        const snapshotSlot = `${slot}/snapshot`;
        const begun = change.from === "snapshot" ? await log.slot(snapshotSlot) : undefined;
        const sequence = begun ?? (await log.position()).sequence;
        if (change.from === "now") {
            await log.advance(slot, sequence, this.#now() + change.maxLag);

            return sequence;
        }

        // record every matching row at the snapshot's position as created, page by page in identifier order, advancing its slot
        const snapshot = log.at({ epoch, sequence });
        const table = ChangeController.#table(trigger);
        let last: Row | undefined;
        for (;;) {
            await log.advance(snapshotSlot, sequence, this.#now() + change.maxLag);
            const rows = await snapshot.ordered(table, {
                where: change.where ?? Condition.all(),
                order: [{ column: "id", direction: "asc" }],
                ...(last === undefined ? {} : { after: { id: last.id } }),
                count: SNAPSHOT_PAGE_ROWS,
            });
            const created = rows.map((row) => ({
                position: { epoch, sequence },
                operation: "create" as const,
                key: { id: row.id },
                after: row,
            }));
            await this.#record(trigger, created);
            last = rows.at(-1);
            if (rows.length < SNAPSHOT_PAGE_ROWS) {
                break;
            }
        }

        // follow the log after the snapshot
        await log.advance(slot, sequence, this.#now() + change.maxLag);
        await log.drop(snapshotSlot);

        return sequence;
    }

    /** Record the runs of some changes' calls, once per change and snapshot row, several at a time; a change whose call fails to build is reported and passed over. */
    async #record(trigger: ChangeTrigger, changes: readonly ObjectChange[]): Promise<void> {
        // build each change's request, reporting and passing over a change the trigger builds no call of
        const requests = changes.flatMap((change) => {
            const key = change.key === undefined ? {} : { key: String(change.key.id) };
            try {
                const call = trigger.call(change);

                return [
                    {
                        call,
                        triggerName: trigger.name,
                        event: { change: { position: change.position, ...key } },
                    },
                ];
            } catch (error) {
                this.#report(
                    new Error(
                        `trigger ${trigger.name} builds no call of change ${change.position.sequence}`,
                        { cause: error },
                    ),
                );

                return [];
            }
        });

        // send them several at a time; the cell orders a trigger's runs by the requests their events derive
        for (let start = 0; start < requests.length; start += SEND_CONCURRENCY) {
            const chunk = requests.slice(start, start + SEND_CONCURRENCY);
            await Promise.all(chunk.map((request) => this.#runs.send(request)));
        }
    }

    /** Read a trigger's key: its package and its name. */
    static #key(trigger: ChangeTrigger): string {
        return `${trigger.package.id}/${trigger.name}`;
    }

    /** Read the table of a trigger's object type. */
    static #table(trigger: ChangeTrigger): Table {
        return (trigger.on.change.object as ObjectType).table as Table;
    }

    /** See a change through a trigger's condition: entering rows created, leaving rows deleted. */
    static #see(
        trigger: ChangeTrigger,
        change: Change,
        epoch: string,
        match: Match | undefined,
    ): ObjectChange | undefined {
        // match each image and classify the change by the matching images
        const before =
            change.before && (match === undefined || Condition.matches(match, change.before));
        const after =
            change.after && (match === undefined || Condition.matches(match, change.after));
        const operation: ChangeOperation | undefined =
            before && after ? "update" : after ? "create" : before ? "delete" : undefined;
        if (operation === undefined || !trigger.on.change.operations.includes(operation)) {
            return undefined;
        }

        return {
            position: { epoch, sequence: change.sequence },
            operation,
            ...(before ? { before: change.before } : {}),
            ...(after ? { after: change.after } : {}),
        };
    }
}
