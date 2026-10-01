import {
    asc,
    decodeRow,
    defineTable,
    desc,
    encodeRow,
    integer,
    Key,
    gte,
    TABLE,
    text,
    type DatabaseConnection,
    type Table,
} from "@destack/db";
import type { LogPosition } from "@destack/db/log";
import { RowChange, type MutationOutcome } from "../query/page.ts";
import { type Call, type Mutation, mutation, Outbox } from "../outbox/outbox.ts";

/** The most changes one read of a prediction takes. */
const RECORD_CHANGES = 1000;

/** The predictions applied over a client's copy, one row per layer in the order applied. */
export const layer = defineTable("layer", {
    /** The layer: a queued mutation's identifier, or the checked-out branch's. */
    key: text("key").primaryKey(),
    /** The layer's place in the order applied. */
    position: integer("position").notNull().unique(),
    /** The predicted changes, as JSON text. */
    changes: text("changes").notNull(),
    /** The tables the prediction reads or writes, as a JSON array of SQL names. */
    reach: text("reach").notNull(),
});

/** The branch a client's copy shows over the main line. */
export const checkout = defineTable("checkout", {
    /** The one row's key. */
    slot: integer("slot").primaryKey(),
    /** The identifier of the checked-out branch. */
    branch: text("branch").notNull(),
});

/** The tables of a client's queue and predictions. */
export const predictionTables = [mutation, layer, checkout] as const;

/** The rows branches change on the server, as the client's copy has them. */
export interface BranchSource {
    /** The SQL names of the tables keeping branches and their rows. */
    readonly tables: readonly string[];
    /** Report whether a branch is open, as the copy has it. */
    isOpen(transaction: DatabaseConnection, branch: string): Promise<boolean>;
    /** Write a branch's rows over the copy's. */
    apply(transaction: DatabaseConnection, branch: string): Promise<void>;
}

/** A layer of the prediction: a queued mutation, or the checked-out branch's rows. */
type Layer =
    | {
          /** A queued mutation's calls. */
          readonly kind: "mutation";
          /** The mutation. */
          readonly mutation: Mutation;
      }
    | {
          /** A branch's rows. */
          readonly kind: "branch";
          /** The branch's identifier. */
          readonly branch: string;
      };

/** What a client's copy shows over the server's rows: queued mutations, and the checked-out branch. */
export class Prediction {
    /** The queue predicted. */
    readonly outbox: Outbox;
    /** The tables predictions change. */
    readonly tables: ReadonlyMap<string, Table>;
    /** Predict a mutation inside a transaction. */
    readonly #predict: (transaction: DatabaseConnection, mutation: Mutation) => Promise<unknown>;
    /** Read the tables a call's prediction may read. */
    readonly #reach: (call: Call) => readonly string[];
    /** The rows branches change, absent for a client without branches. */
    readonly #branches: BranchSource | undefined;

    /** Predict a queue over some tables. */
    constructor(
        tables: readonly Table[],
        predict: (transaction: DatabaseConnection, mutation: Mutation) => Promise<unknown>,
        reach: (call: Call) => readonly string[],
        branches?: BranchSource,
    ) {
        // keep the queue, the tables, and how calls and branches predict
        this.outbox = new Outbox();
        this.tables = new Map(tables.map((table) => [table[TABLE].sqlName, table]));
        this.#predict = predict;
        this.#reach = reach;
        this.#branches = branches;
    }

    /** Predict and queue a mutation on the checked-out branch, or on the main line when asked. */
    async add<Result>(
        database: DatabaseConnection,
        id: string,
        origin: string,
        predict: (
            transaction: DatabaseConnection,
        ) => Promise<{ readonly calls: readonly Call[]; readonly result: Result }>,
        options: {
            /** Whether the mutation goes on the main line even while a branch is checked out. */
            readonly isMainLine?: boolean;
        } = {},
    ): Promise<Result> {
        return database.transaction(async (transaction) => {
            // place the layer on top, or after the main line's below a checked-out branch
            const checkedOut = await this.checkedOut(transaction);
            const branch = options.isMainLine === true ? undefined : checkedOut;
            const stack = await this.#stack(transaction);
            const index =
                branch === undefined && checkedOut !== undefined
                    ? stack.findIndex((entry) => entry.kind === "branch")
                    : stack.length;

            // revert the layers above it, predict it, then predict them again on top
            await transaction.log.copying(() => this.#revertFrom(transaction, index + 1));
            const recorded = await this.#record(transaction, predict);
            await this.outbox.append(transaction, { id, origin, calls: recorded.calls, branch });
            await transaction.insert(layer).values({
                key: id,
                position: index + 1,
                changes: JSON.stringify(recorded.changes),
                reach: this.#reached(recorded.calls, recorded.changes),
            });
            await this.#apply(transaction, stack.slice(index), index + 2, recorded.sequence);

            return recorded.value;
        });
    }

    /** Read the checked-out branch, absent on the main line. */
    async checkedOut(database: DatabaseConnection): Promise<string | undefined> {
        const [row] = await database.select({ branch: checkout.branch }).from(checkout);

        return row?.branch;
    }

    /** Show a branch over the main line, or the main line alone. */
    async checkout(database: DatabaseConnection, branch: string | undefined): Promise<void> {
        await database.transaction(async (transaction) => {
            // revert, switch the branch shown, and predict again
            await transaction.log.copying(() => this.revert(transaction));
            await transaction.delete(checkout);
            if (branch !== undefined) {
                await transaction.insert(checkout).values({ slot: 1, branch });
            }
            await this.replay(transaction, []);
        });
    }

    /** Decide whether a page changing some tables reaches a layer. */
    async reaches(database: DatabaseConnection, changed: ReadonlySet<string>): Promise<boolean> {
        const rows = await database.select({ reach: layer.reach }).from(layer);

        return rows.some((row) =>
            (JSON.parse(row.reach) as string[]).some((table) => changed.has(table)),
        );
    }

    /** Read the predicted changes in the order applied. */
    async predicted(database: DatabaseConnection): Promise<RowChange[]> {
        const rows = await database
            .select({ changes: layer.changes })
            .from(layer)
            .orderBy(asc(layer.position));

        return rows.flatMap((row) => JSON.parse(row.changes) as RowChange[]);
    }

    /** Revert every layer, latest first. */
    async revert(transaction: DatabaseConnection): Promise<void> {
        await this.#revertFrom(transaction, 1);
    }

    /** Settle the queue a page answered, then predict the stack again over the reverted copy. */
    async replay(
        transaction: DatabaseConnection,
        outcomes: readonly MutationOutcome[],
        snapshot?: LogPosition,
    ): Promise<void> {
        // settle the queue
        await this.outbox.settle(transaction, outcomes, snapshot);

        // show the main line once the checked-out branch closes
        const branch = await this.checkedOut(transaction);
        if (
            branch !== undefined &&
            !((await this.#branches?.isOpen(transaction, branch)) ?? true)
        ) {
            await transaction.delete(checkout);
        }

        // predict every layer in order
        await this.#apply(transaction, await this.#stack(transaction), 1, undefined);
    }

    /** List the layers in the order they apply: the main line, then the checked-out branch's rows and edits. */
    async #stack(transaction: DatabaseConnection): Promise<Layer[]> {
        // read the branch shown and the queue
        const branch = await this.checkedOut(transaction);
        const queued = await this.outbox.queued(transaction);
        const layered = (entry: (typeof queued)[number]): Layer => ({
            kind: "mutation",
            mutation: entry,
        });

        return [
            ...queued.filter((entry) => entry.branch === null).map(layered),
            ...(branch === undefined
                ? []
                : [
                      { kind: "branch" as const, branch },
                      ...queued.filter((entry) => entry.branch === branch).map(layered),
                  ]),
        ];
    }

    /** Predict layers in order from a position, continuing each one's changes from the last. */
    async #apply(
        transaction: DatabaseConnection,
        layers: readonly Layer[],
        position: number,
        after: number | undefined,
    ): Promise<void> {
        let reached = after;
        for (const [index, entry] of layers.entries()) {
            // predict the layer
            const recorded = await this.#record(
                transaction,
                async (inner) => {
                    if (entry.kind === "mutation") {
                        await this.#predict(inner, entry.mutation);
                    } else {
                        await this.#branches?.apply(inner, entry.branch);
                    }

                    return {
                        calls: entry.kind === "mutation" ? entry.mutation.calls : [],
                        result: undefined,
                    };
                },
                reached,
            );
            reached = recorded.sequence;

            // keep its changes and reach
            await transaction.insert(layer).values({
                key: entry.kind === "mutation" ? entry.mutation.id : entry.branch,
                position: position + index,
                changes: JSON.stringify(recorded.changes),
                reach: this.#reached(recorded.calls, recorded.changes, entry.kind === "branch"),
            });
        }
    }

    /** Revert the layers from a position up, latest first. */
    async #revertFrom(transaction: DatabaseConnection, position: number): Promise<void> {
        const rows = await transaction
            .select({ changes: layer.changes })
            .from(layer)
            .where(gte(layer.position, position))
            .orderBy(desc(layer.position));
        for (const row of rows) {
            for (const change of (JSON.parse(row.changes) as RowChange[]).reverse()) {
                await this.#invert(transaction, change);
            }
        }
        await transaction.delete(layer).where(gte(layer.position, position));
    }

    /** List the tables a layer reads or writes, with the branch rows' table for a branch. */
    #reached(calls: readonly Call[], changes: readonly RowChange[], isBranch = false): string {
        const tables = new Set([
            ...calls.flatMap((call) => this.#reach(call)),
            ...changes.map((change) => change.table),
            ...(isBranch && this.#branches !== undefined ? this.#branches.tables : []),
        ]);

        return JSON.stringify([...tables].sort());
    }

    /** Run a prediction and read the changes it logged, with the sequence they reach. */
    async #record<Value>(
        transaction: DatabaseConnection,
        predict: (
            transaction: DatabaseConnection,
        ) => Promise<{ readonly calls: readonly Call[]; readonly result: Value }>,
        after?: number,
    ): Promise<{
        readonly calls: readonly Call[];
        readonly value: Value;
        readonly changes: RowChange[];
        readonly sequence: number;
    }> {
        // require SQLite, whose log sequences changes within a transaction
        if (transaction.dialect !== "sqlite") {
            throw new TypeError(
                "a prediction runs on SQLite, whose log sequences changes within a transaction",
            );
        }

        // run the prediction after the latest change
        const tables = [...this.tables.values()];
        let sequence = after ?? (await transaction.log.reached()).sequence;
        const { calls, result } = await predict(transaction);

        // read every logged change
        const changes: RowChange[] = [];
        for (;;) {
            const page = await transaction.log.read({
                tables,
                after: sequence,
                limit: RECORD_CHANGES,
            });
            changes.push(
                ...page.changes.map((change) => ({
                    table: change.table[TABLE].sqlName,
                    operation: change.operation,
                    row: encodeRow(change.table, (change.after ?? change.before)!),
                    ...(change.operation === "update"
                        ? { before: encodeRow(change.table, change.before!) }
                        : {}),
                })),
            );
            sequence = page.sequence;
            if (page.changes.length < RECORD_CHANGES) {
                break;
            }
        }

        return { calls, value: result, changes, sequence };
    }

    /** Undo one predicted change. */
    async #invert(transaction: DatabaseConnection, change: RowChange): Promise<void> {
        // match the row by its key
        const table = this.tables.get(change.table)!;
        const row = decodeRow(table, change.row);
        const matched = Key.match(table, row);

        // remove an inserted row
        if (change.operation === "insert") {
            await transaction.delete(table).where(matched);
        }
        // restore the row before an update
        else if (change.operation === "update") {
            await transaction
                .update(table)
                .set(decodeRow(table, change.before!) as never)
                .where(matched);
        }
        // reinsert a deleted row
        else {
            await transaction.insert(table).values(row as never);
        }
    }
}
