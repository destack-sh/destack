import type { DatabaseConnection, Row, Table } from "@destack/db";
import { DatabaseError } from "@destack/db/error";
import { Log, type Change } from "@destack/db/log";
import { Condition, type Match } from "@destack/db/query";
import type { Controller } from "@destack/service/control";
import type { RunClient, TriggerOf } from "@destack/service/trigger";
import type { ObjectChange, WatchOperation } from "@destack/service/watch";
import { ObjectType } from "../object/object.ts";

/** The rows one snapshot page reads: a few hundred kilobytes. */
const SNAPSHOT_PAGE_ROWS = 500;

/** The runs recorded at once: 32 requests of a few milliseconds keep a page of 500 changes under a tenth of a second. */
const SEND_CONCURRENCY = 32;

/** How often a watch looks again within its lag, advancing its slot however quiet its table: three times. */
const SLOT_ADVANCES = 3;

/** A watch of the served objects. */
type Watch = TriggerOf<"watch">;

/** Follow the watches of served objects, recording the call of each change they admit as a run in the space's cell. */
export class WatchController implements Controller {
    /** The controller's name in reports. */
    readonly name = "watch";
    /** The watched tables. */
    readonly watches: readonly Table[];
    /** The watches followed side by side. */
    readonly concurrency: number;
    /** The database keeping the watched objects and their log. */
    readonly #database: DatabaseConnection;
    /** The cell recording the runs. */
    readonly #runs: RunClient;
    /** The watches, by name. */
    readonly #watched: ReadonlyMap<string, Watch>;
    /** Report a change the watch passes over or loses. */
    readonly #report: (error: unknown) => void;
    /** Read the current time. */
    readonly #now: () => number;

    /** Follow watches over a database, recording runs in a cell. */
    constructor(
        database: DatabaseConnection,
        watches: readonly Watch[],
        runs: RunClient,
        report: (error: unknown) => void,
        now: () => number = Date.now,
    ) {
        // keep the database, the cell, the report and the clock
        this.#database = database;
        this.#runs = runs;
        this.#report = report;
        this.#now = now;

        // require each watch to follow a served object type, under a name its package gives no other watch
        const watched = new Map<string, Watch>();
        for (const watch of watches) {
            const key = WatchController.#key(watch);
            if (!(watch.object instanceof ObjectType)) {
                throw new TypeError(`watch ${watch.name} follows no object type`);
            } else if (watched.has(key)) {
                throw new TypeError(
                    `${watch.package.name} declares two watches named ${watch.name}`,
                );
            }
            watched.set(key, watch);
        }
        this.#watched = watched;
        this.watches = [...new Set(watches.map((watch) => WatchController.#table(watch)))];
        this.concurrency = Math.max(1, watches.length);
    }

    /** Select the watches of a changed table. */
    keys(change: Change): readonly string[] {
        return [...this.#watched.values()]
            .filter((watch) => WatchController.#table(watch) === change.table)
            .map((watch) => WatchController.#key(watch));
    }

    /** List every watch. */
    async list(): Promise<readonly string[]> {
        return [...this.#watched.keys()];
    }

    /** Record the runs of a watch's admitted changes since its slot, advance the slot past them, and look again before the slot lapses. */
    async reconcile(key: string): Promise<number> {
        // continue after the watch's slot, or start it
        const watch = this.#watched.get(key)!;
        const table = WatchController.#table(watch);
        const log = new Log(this.#database);
        const slot = `watch/${key}`;
        const epoch = await log.epoch();
        const match = watch.where && Condition.compile(watch.where, table);
        let after = (await log.slot(slot)) ?? (await this.#start(watch, slot, epoch));

        // record each admitted change page by page, advancing the slot after each page
        try {
            for (;;) {
                const page = await log.read({ tables: [table], after });
                const seen = page.changes.flatMap((change) => {
                    const entry = WatchController.#see(watch, change, epoch, match);

                    return entry === undefined
                        ? []
                        : [{ change: entry, sequence: change.sequence }];
                });
                await this.#record(watch, seen, epoch);
                after = Math.max(after, page.sequence);
                await log.advance(slot, after, this.#now() + watch.maxLag);
                if (page.changes.length === 0) {
                    break;
                }
            }
        } catch (error) {
            // continue at the head once the log compacted changes the watch had not recorded, reporting the loss
            if (!(error instanceof DatabaseError && error.code === "CHANGES_COMPACTED")) {
                throw error;
            }
            const head = (await log.position()).sequence;
            await log.advance(slot, head, this.#now() + watch.maxLag);
            this.#report(
                new Error(`watch ${watch.name} lost the changes after ${after} to compaction`, {
                    cause: error,
                }),
            );
        }

        // look again well before the slot lapses, however quiet the table
        return Math.floor(watch.maxLag / SLOT_ADVANCES);
    }

    /** Start a watch at the log's head, recording its snapshot's rows first when it starts with them. */
    async #start(watch: Watch, slot: string, epoch: string): Promise<number> {
        // start the slot at the head, or resume the snapshot begun at an earlier head
        const log = new Log(this.#database);
        const snapshotSlot = `${slot}/snapshot`;
        const begun = watch.from === "snapshot" ? await log.slot(snapshotSlot) : undefined;
        const sequence = begun ?? (await log.position()).sequence;
        if (watch.from === "now") {
            await log.advance(slot, sequence, this.#now() + watch.maxLag);

            return sequence;
        }

        // record every matching row at the snapshot's position as created, page by page in identifier order, advancing its slot
        const snapshot = log.at({ epoch, sequence });
        const table = WatchController.#table(watch);
        let last: Row | undefined;
        for (;;) {
            await log.advance(snapshotSlot, sequence, this.#now() + watch.maxLag);
            const rows = await snapshot.ordered(table, {
                where: watch.where ?? Condition.all(),
                order: [{ column: "id", direction: "asc" }],
                ...(last === undefined ? {} : { after: { id: last.id } }),
                count: SNAPSHOT_PAGE_ROWS,
            });
            const created = rows.map((row) => ({
                change: {
                    position: { epoch, sequence },
                    operation: "create" as const,
                    key: { id: row.id },
                    after: row,
                },
                sequence,
                key: String(row.id),
            }));
            await this.#record(watch, created, epoch);
            last = rows.at(-1);
            if (rows.length < SNAPSHOT_PAGE_ROWS) {
                break;
            }
        }

        // follow the log after the snapshot
        await log.advance(slot, sequence, this.#now() + watch.maxLag);
        await log.drop(snapshotSlot);

        return sequence;
    }

    /** Record the runs of some changes' calls, once per change and snapshot row, several at a time; a change whose call fails to build is reported and passed over. */
    async #record(
        watch: Watch,
        changes: readonly {
            readonly change: ObjectChange;
            readonly sequence: number;
            readonly key?: string;
        }[],
        epoch: string,
    ): Promise<void> {
        // build each change's request, reporting and passing over a change the watch builds no call of
        const cause = { cause: "watch" as const, packageId: watch.package.id, trigger: watch.name };
        const requests = changes.flatMap(({ change, sequence, key }) => {
            try {
                const call = watch.call(change);

                return [{ ...cause, call, epoch, sequence, ...(key === undefined ? {} : { key }) }];
            } catch (error) {
                this.#report(
                    new Error(`watch ${watch.name} builds no call of change ${sequence}`, {
                        cause: error,
                    }),
                );

                return [];
            }
        });

        // send them several at a time; the cell orders a watch's runs by position
        for (let start = 0; start < requests.length; start += SEND_CONCURRENCY) {
            const chunk = requests.slice(start, start + SEND_CONCURRENCY);
            await Promise.all(chunk.map((request) => this.#runs.send(request)));
        }
    }

    /** Read a watch's key: its package and its name. */
    static #key(watch: Watch): string {
        return `${watch.package.id}/${watch.name}`;
    }

    /** Read the table of a watch's object type. */
    static #table(watch: Watch): Table {
        return (watch.object as ObjectType).table as Table;
    }

    /** See a change through a watch's condition: entering rows created, leaving rows deleted. */
    static #see(
        watch: Watch,
        change: Change,
        epoch: string,
        match: Match | undefined,
    ): ObjectChange | undefined {
        // match each image and classify the change by the matching images
        const before =
            change.before && (match === undefined || Condition.matches(match, change.before));
        const after =
            change.after && (match === undefined || Condition.matches(match, change.after));
        const operation: WatchOperation | undefined =
            before && after ? "update" : after ? "create" : before ? "delete" : undefined;
        if (operation === undefined || !watch.on.includes(operation)) {
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
