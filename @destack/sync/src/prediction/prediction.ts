import {
    and,
    asc,
    count,
    defineTable,
    desc,
    eq,
    integer,
    inArray,
    isNotNull,
    isNull,
    json,
    Key,
    gte,
    lt,
    lte,
    max,
    sql,
    TABLE,
    text,
    type DatabaseConnection,
    type SQL,
    type Table,
    type LogPosition,
    Change,
} from "@destack/db";
import { schema } from "@destack/schema";
import { RowChange } from "../query/page.ts";
import { Call, Failure, type Mutation } from "../call/index.ts";

/** The most changes one read of a prediction takes. */
const RECORD_CHANGES = 1000;

/** A layer's predicted changes, as its JSON text has them. */
const LayerChanges = schema.array(RowChange);

/** The tables a layer reads or writes, as its JSON text has them. */
const LayerTables = schema.array(schema.string());

/** The state of a mutation. */
export type MutationState =
    | {
          /** Waiting for the server, or executed. */
          readonly kind: "pending" | "executed";
      }
    | {
          /** Rejected by the server. */
          readonly kind: "rejected";
          /** The failure the server recorded. */
          readonly error: Failure;
      };

/** The mutations of a client's outbox, in order. */
export const mutation = defineTable("mutation", {
    /** The request identifier. */
    id: text("id").primaryKey().validate(schema.uuidv7()),
    /** The party that queued the mutation, such as a browser tab. */
    origin: text("origin").notNull(),
    /** The mutation's place in the outbox. */
    position: integer("position").notNull().unique(),
    /** The calls. */
    calls: json("calls", schema.array(Call)).notNull(),
    /** The identifier of the branch the calls are pushed to, absent on the main line. */
    branch: text("branch"),
    /** The server log's epoch of the executed mutation. */
    epoch: text("epoch"),
    /** The server log sequence of the mutation's changes. */
    sequence: integer("sequence"),
    /** The failure the server recorded. */
    error: json("error", Failure),
});

/** The predictions applied over a client's copy, one row per layer in the order applied. */
export const layer = defineTable("layer", {
    /** The layer: a queued mutation's identifier, or the checked-out branch's. */
    key: text("key").primaryKey(),
    /** The layer's place in the order applied. */
    position: integer("position").notNull().unique(),
    /** The predicted changes, as JSON text. */
    changes: text("changes").notNull(),
    /** The tables the prediction reads or writes, as a JSON array of SQL names. */
    tables: text("tables").notNull(),
});

/** The branch a client's copy shows over the main line. */
export const checkout = defineTable("checkout", {
    /** The one row's key. */
    id: integer("id").primaryKey(),
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
    /** The tables predictions change. */
    readonly tables: ReadonlyMap<string, Table>;
    /** Predict a mutation inside a transaction. */
    readonly #predict: (transaction: DatabaseConnection, mutation: Mutation) => Promise<unknown>;
    /** Read the tables a call's prediction may read. */
    readonly #reads: (call: Call) => readonly string[];
    /** The rows branches change, absent for a client without branches. */
    readonly #branches: BranchSource | undefined;

    /** Predict a queue over some tables. */
    constructor(definition: {
        /** The tables predictions change. */
        readonly tables: readonly Table[];
        /** Predict a mutation inside a transaction. */
        readonly predict: (transaction: DatabaseConnection, mutation: Mutation) => Promise<unknown>;
        /** Read the tables a call's prediction may read. */
        readonly reads: (call: Call) => readonly string[];
        /** The rows branches change, absent for a client without branches. */
        readonly branches?: BranchSource;
    }) {
        // keep the tables and how calls and branches predict
        this.tables = new Map(definition.tables.map((table) => [table[TABLE].sqlName, table]));
        this.#predict = definition.predict;
        this.#reads = definition.reads;
        this.#branches = definition.branches;
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
            await transaction.log.asReplica(() => this.#revertFrom(transaction, index + 1));
            const recorded = await this.#record(transaction, predict);
            await this.#append(transaction, { id, origin, calls: recorded.calls, branch });
            await transaction.insert(layer).values({
                key: id,
                position: index + 1,
                changes: JSON.stringify(recorded.changes),
                tables: this.#tables(recorded.calls, recorded.changes),
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
            await transaction.log.asReplica(() => this.revert(transaction));
            await transaction.delete(checkout);
            if (branch !== undefined) {
                await transaction.insert(checkout).values({ id: 1, branch });
            }
            await this.replay(transaction);
        });
    }

    /** Decide whether a page changing some tables touches a layer. */
    async touches(database: DatabaseConnection, changed: ReadonlySet<string>): Promise<boolean> {
        const rows = await database.select({ tables: layer.tables }).from(layer);

        return rows.some((row) =>
            LayerTables.parse(JSON.parse(row.tables)).some((table) => changed.has(table)),
        );
    }

    /** Read the predicted changes in the order applied. */
    async predicted(database: DatabaseConnection): Promise<RowChange[]> {
        const rows = await database
            .select({ changes: layer.changes })
            .from(layer)
            .orderBy(asc(layer.position));

        return rows.flatMap((row) => LayerChanges.parse(JSON.parse(row.changes)));
    }

    /** Revert every layer, latest first. */
    async revert(transaction: DatabaseConnection): Promise<void> {
        await this.#revertFrom(transaction, 1);
    }

    /** Settle the queue a copy's completed page reaches, then predict the stack again over the reverted copy. */
    async replay(
        transaction: DatabaseConnection,
        reached?: { readonly position: LogPosition; readonly isSnapshot: boolean },
    ): Promise<void> {
        // settle the queue
        if (reached !== undefined) {
            await this.#settle(transaction, reached.position, reached.isSnapshot);
        }

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
        const queued = await this.#queued(transaction);

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
                async (nested) => {
                    if (entry.kind === "mutation") {
                        await this.#predict(nested, entry.mutation);
                    } else {
                        await this.#branches?.apply(nested, entry.branch);
                    }

                    return {
                        calls: entry.kind === "mutation" ? entry.mutation.calls : [],
                        result: undefined,
                    };
                },
                reached,
            );
            reached = recorded.sequence;

            // keep its changes and tables
            await transaction.insert(layer).values({
                key: entry.kind === "mutation" ? entry.mutation.id : entry.branch,
                position: position + index,
                changes: JSON.stringify(recorded.changes),
                tables: this.#tables(recorded.calls, recorded.changes, entry.kind === "branch"),
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
            for (const change of LayerChanges.parse(JSON.parse(row.changes)).toReversed()) {
                await this.#invert(transaction, change);
            }
        }
        await transaction.delete(layer).where(gte(layer.position, position));
    }

    /** List the tables a layer reads or writes, with the branch rows' table for a branch. */
    #tables(calls: readonly Call[], changes: readonly RowChange[], isBranch = false): string {
        const tables = new Set([
            ...calls.flatMap((call) => this.#reads(call)),
            ...changes.map((change) => change.table),
            ...(isBranch && this.#branches !== undefined ? this.#branches.tables : []),
        ]);

        return JSON.stringify([...tables].toSorted());
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
            changes.push(...page.changes.map((change) => predictedChange(change)));
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
        const table = this.tables.get(change.table);
        if (table === undefined) {
            throw new TypeError(`a prediction changed an unknown table: ${change.table}`);
        }
        const row = table[TABLE].decode(change.row);
        const matched = Key.match(table, row);

        // remove an inserted row
        if (change.operation === "insert") {
            await transaction.delete(table).where(matched);
        }
        // restore the row before an update
        else if (change.operation === "update") {
            if (change.before === undefined) {
                throw new TypeError(`a predicted update of ${change.table} lacks its before`);
            }
            await transaction.update(table).set(table[TABLE].decode(change.before)).where(matched);
        }
        // reinsert a deleted row
        else {
            await transaction.insert(table).values(row);
        }
    }

    /** Append a mutation after every other. */
    async #append(
        transaction: DatabaseConnection,
        entry: {
            /** The request identifier. */
            readonly id: string;
            /** The party queueing it. */
            readonly origin: string;
            /** The calls. */
            readonly calls: readonly Call[];
            /** The branch the calls are pushed to, absent on the main line. */
            readonly branch: string | undefined;
        },
    ): Promise<void> {
        const [last] = await transaction
            .select({ position: max(mutation.position) })
            .from(mutation);
        await transaction.insert(mutation).values({
            id: entry.id,
            origin: entry.origin,
            position: (last?.position ?? 0) + 1,
            calls: [...entry.calls],
            branch: entry.branch ?? null,
            epoch: null,
            sequence: null,
            error: null,
        });
    }

    /** Read the mutations the server has not rejected, in order. */
    async #queued(
        database: DatabaseConnection,
    ): Promise<{ readonly id: string; readonly calls: Call[]; readonly branch: string | null }[]> {
        return database
            .select({ id: mutation.id, calls: mutation.calls, branch: mutation.branch })
            .from(mutation)
            .where(isNull(mutation.error))
            .orderBy(asc(mutation.position));
    }

    /** Read the mutations waiting for the server, in order, with the branch each pushes to. */
    async pending(
        database: DatabaseConnection,
        options: {
            /** The most mutations to read, every pending one when absent. */
            readonly limit?: number;
        } = {},
    ): Promise<(Mutation & { readonly branch?: string })[]> {
        const query = database
            .select({ id: mutation.id, calls: mutation.calls, branch: mutation.branch })
            .from(mutation)
            .where(and(isNull(mutation.epoch), isNull(mutation.error)))
            .orderBy(asc(mutation.position));
        const rows = await (options.limit === undefined ? query : query.limit(options.limit));

        return rows.map((row) => ({
            id: row.id,
            calls: row.calls,
            ...(row.branch === null ? {} : { branch: row.branch }),
        }));
    }

    /** Count the pending, executed and rejected mutations. */
    async inspect(
        database: DatabaseConnection,
    ): Promise<{ readonly pending: number; readonly executed: number; readonly rejected: number }> {
        // count each state in one pass with pending as the push reads it
        const [counts] = await database
            .select({
                pending: countWhen(and(isNull(mutation.epoch), isNull(mutation.error))),
                executed: countWhen(and(isNotNull(mutation.epoch), isNull(mutation.error))),
                rejected: countWhen(isNotNull(mutation.error)),
            })
            .from(mutation);
        if (counts === undefined) {
            throw new TypeError("a count read returned no row");
        }

        return counts;
    }

    /** Wait until a committed mutation is pending, returning false once the signal aborts. */
    wait(database: DatabaseConnection, signal: AbortSignal): Promise<boolean> {
        return database.log.until(
            async () => (await this.pending(database, { limit: 1 })).length > 0,
            signal,
        );
    }

    /** Record the log position of an executed mutation. */
    async acknowledge(
        database: DatabaseConnection,
        id: string,
        position: LogPosition,
    ): Promise<void> {
        await database
            .update(mutation)
            .set({ epoch: position.epoch, sequence: position.sequence })
            .where(eq(mutation.id, id));
    }

    /** Record the server's rejection of a mutation. */
    async reject(database: DatabaseConnection, id: string, error: Failure): Promise<void> {
        await database.update(mutation).set({ error }).where(eq(mutation.id, id));
    }

    /** Report whether a position reaches an executed mutation's changes. */
    async isReached(transaction: DatabaseConnection, position: LogPosition): Promise<boolean> {
        const [reached] = await transaction
            .select({ id: mutation.id })
            .from(mutation)
            .where(acknowledged(position))
            .limit(1);

        return reached !== undefined;
    }

    /** Drop the executed mutations a copy's position reaches, and push again what a snapshot of a later epoch lost. */
    async #settle(
        transaction: DatabaseConnection,
        position: LogPosition,
        isSnapshot: boolean,
    ): Promise<void> {
        // drop the mutations whose changes the copy has
        await transaction.delete(mutation).where(acknowledged(position));

        // push again what an earlier epoch executed
        if (isSnapshot) {
            await transaction
                .update(mutation)
                .set({ epoch: null, sequence: null })
                .where(and(isNotNull(mutation.epoch), lt(mutation.epoch, position.epoch)));
        }
    }

    /** Read mutations' outcomes in one statement. */
    async outcomes(
        database: DatabaseConnection,
        ids: readonly string[],
    ): Promise<ReadonlyMap<string, MutationState>> {
        const rows = await database
            .select({ id: mutation.id, error: mutation.error })
            .from(mutation)
            .where(inArray(mutation.id, [...ids]));
        const errors = new Map(rows.map((row) => [row.id, row.error]));

        return new Map(ids.map((id) => [id, mutationState(errors.get(id))]));
    }

    /** Read the origins with queued mutations. */
    async origins(database: DatabaseConnection): Promise<Set<string>> {
        const rows = await database.selectDistinct({ origin: mutation.origin }).from(mutation);

        return new Set(rows.map((row) => row.origin));
    }

    /** Remove the rejected mutations of an origin. */
    async forget(database: DatabaseConnection, origin: string, id?: string): Promise<void> {
        await database
            .delete(mutation)
            .where(
                and(
                    eq(mutation.origin, origin),
                    isNotNull(mutation.error),
                    id === undefined ? undefined : eq(mutation.id, id),
                ),
            );
    }
}

/** Layer a queued mutation's calls. */
function layered(queued: Mutation): Layer {
    return { kind: "mutation", mutation: queued };
}

/** Record a logged change as its predicted row change. */
function predictedChange(change: Change): RowChange {
    // take the row after the change or before a deletion
    const definition = change.table[TABLE];
    const row = Change.image(change);
    const written = {
        table: definition.sqlName,
        operation: change.operation,
        row: definition.encode(row),
    };

    // keep an update's row before
    if (change.operation !== "update") {
        return written;
    } else if (change.before === undefined) {
        throw new TypeError(`a logged update of ${definition.name} lacks its before`);
    }

    return { ...written, before: definition.encode(change.before) };
}

/** Match the executed mutations whose changes a position reaches. */
function acknowledged(position: LogPosition): SQL | undefined {
    return and(
        isNull(mutation.error),
        eq(mutation.epoch, position.epoch),
        lte(mutation.sequence, position.sequence),
    );
}

/** Count the rows meeting a condition. */
function countWhen(condition: SQL | undefined): SQL<number> {
    return count(sql`CASE WHEN ${condition} THEN 1 END`);
}

/** Read a mutation's state from its recorded error: executed without a row, pending without an error, else rejected. */
function mutationState(
    error: Extract<MutationState, { kind: "rejected" }>["error"] | null | undefined,
): MutationState {
    // read an executed mutation
    if (error === undefined) {
        return { kind: "executed" };
    }
    // read a pending one
    else if (error === null) {
        return { kind: "pending" };
    }
    // read a rejected one
    else {
        return { kind: "rejected", error };
    }
}
