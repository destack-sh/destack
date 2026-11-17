import {
    and,
    asc,
    count,
    defineTable,
    desc,
    eq,
    integer,
    inArray,
    json,
    gte,
    not,
    lte,
    max,
    ne,
    sql,
    TABLE,
    text,
    textEnum,
    type DatabaseConnection,
    type SQL,
    type LogPosition,
    type LogId,
    Change,
    ChangeSet,
    Table,
    type RowChange,
} from "@destack/db";
import { present, schema } from "@destack/schema";
import { Replica } from "../replica/replica.ts";
import { Failure } from "@destack/error";
import { Call, type Mutation } from "../call/index.ts";

/** A layer's predicted rows before and after, as its JSON text has them. */
const LayerChanges = schema.array(
    schema.object({
        /** The row's table, by SQL name. */
        table: schema.string().min(1),
        /** The row before the layer, null for a row it creates. */
        before: schema.jsonObject().nullable(),
        /** The row after the layer, null for a row it removes. */
        after: schema.jsonObject().nullable(),
    }),
);

/** The tables a layer reads or writes, as its JSON text has them. */
const LayerTables = schema.array(schema.string());

/** The states a mutation passes through in the outbox, in order. */
export const MUTATION_STATES = ["queued", "sent", "executed", "rejected"] as const;

/** The state of a mutation, settled once its replica reflects its execution and drops it. */
export type MutationState =
    | {
          /** Queued, sent, executed on the server, or settled and dropped from the outbox. */
          readonly kind: "queued" | "sent" | "executed" | "settled";
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
    /** The mutation's state; its calls change only while queued. */
    state: textEnum("state", MUTATION_STATES).notNull().default("queued"),
    /** The server log of the executed mutation. */
    log: text("log"),
    /** The server log's epoch of the executed mutation. */
    epoch: text("epoch"),
    /** The server log sequence of the mutation's changes. */
    sequence: integer("sequence"),
    /** The failure the server recorded. */
    error: json("error", Failure),
});

/** The predictions applied over a client's replica, one row per layer in the order applied. */
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

/** The branch a client's replica shows over the main line. */
export const checkout = defineTable("checkout", {
    /** The one row's key. */
    id: integer("id").primaryKey(),
    /** The identifier of the checked-out branch. */
    branch: text("branch").notNull(),
});

/** The tables of a client's queue and predictions. */
export const predictionTables = [mutation, layer, checkout] as const;

/** The rows branches change on the server, as the client's replica has them. */
export interface BranchSource {
    /** The SQL names of the tables that store branches and their rows. */
    readonly tables: readonly string[];
    /** Report whether a branch is open, as the replica has it. */
    readonly isOpen: (transaction: DatabaseConnection, branch: string) => Promise<boolean>;
    /** Write a branch's rows over the replica's. */
    readonly apply: (transaction: DatabaseConnection, branch: string) => Promise<void>;
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

/** A prediction's calls: a new mutation's, or the latest mutation's calls composed with it. */
export type Predicted<Result> = {
    /** Whether the calls queue as a new mutation or replace the latest mutation's. */
    readonly kind: "add" | "compose";
    /** The calls to queue. */
    readonly calls: readonly Call[];
    /** The prediction's result. */
    readonly result: Result;
};

/** What a client's replica shows over the server's rows: queued mutations and the branch. */
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
        // store the tables and how calls and branches predict
        this.tables = new Map(definition.tables.map((table) => [table[TABLE].sqlName, table]));
        this.#predict = definition.predict;
        this.#reads = definition.reads;
        this.#branches = definition.branches;
    }

    /** Predict calls on the checked-out branch or main line, queued or composed into the latest. */
    async add<Result>(
        database: DatabaseConnection,
        id: string,
        origin: string,
        predict: (
            transaction: DatabaseConnection,
            latest: Mutation | undefined,
        ) => Promise<Predicted<Result>>,
        options: {
            /** Whether the mutation goes on the main line even while a branch is checked out. */
            readonly isMainLine?: boolean;
        } = {},
    ): Promise<{ readonly id: string; readonly result: Result }> {
        return database.transaction(
            async (transaction) => {
                // place the layer on top of the main line with no branch shown, reading only the top
                const checkedOut = await this.checkedOut(transaction);
                if (checkedOut === undefined) {
                    return await this.#addOnTop(transaction, id, origin, predict);
                }

                // place it on top, or after the main line's below a checked-out branch
                const branch = options.isMainLine === true ? undefined : checkedOut;
                const stack = await this.#stack(transaction);
                const index =
                    branch === undefined && checkedOut !== undefined
                        ? stack.findIndex((entry) => entry.kind === "branch")
                        : stack.length;

                // offer the top mutation to compose into while it is unsent, the origin's and on the line
                const latest =
                    index === stack.length
                        ? await this.#composable(transaction, stack.at(-1), origin, branch)
                        : undefined;

                // revert the layers above it and predict it
                await transaction.log.asReplica(() => this.#revertFrom(transaction, index + 1));
                const recorded = await this.#record(transaction, async (nested) => {
                    const predicted = await predict(nested, latest);

                    return { calls: predicted.calls, result: predicted };
                });
                const { kind, result } = recorded.value;

                // compose it into the latest mutation, extending its layer
                if (kind === "compose") {
                    const composed = present(latest, "the mutation a prediction composes into");
                    await this.#compose(transaction, composed.id, recorded.calls, recorded.changes);

                    return { id: composed.id, result };
                }

                // queue it as a new mutation, and predict the layers above again on top
                await this.#append(transaction, { id, origin, calls: recorded.calls, branch });
                await transaction.insert(layer).values({
                    key: id,
                    position: index + 1,
                    changes: this.#encode(recorded.changes),
                    tables: this.#tables(recorded.calls, recorded.changes),
                });
                await this.#apply(transaction, stack.slice(index), index + 2, recorded.after);

                return { id, result };
            },
            { constraints: "deferred" },
        );
    }

    /** Read the checked-out branch, absent on the main line and for a client without branches. */
    async checkedOut(database: DatabaseConnection): Promise<string | undefined> {
        if (this.#branches === undefined) {
            return undefined;
        }
        const [row] = await database.select({ branch: checkout.branch }).from(checkout);

        return row?.branch;
    }

    /** Predict a mutation on top of the main line, composed into the top one or queued after it. */
    async #addOnTop<Result>(
        transaction: DatabaseConnection,
        id: string,
        origin: string,
        predict: (
            transaction: DatabaseConnection,
            latest: Mutation | undefined,
        ) => Promise<Predicted<Result>>,
    ): Promise<{ readonly id: string; readonly result: Result }> {
        // read the top mutation the server has not rejected, offered to compose into while unsent and the origin's
        const [top] = await transaction
            .select({
                id: mutation.id,
                calls: mutation.calls,
                branch: mutation.branch,
                origin: mutation.origin,
                state: mutation.state,
            })
            .from(mutation)
            .where(ne(mutation.state, "rejected"))
            .orderBy(desc(mutation.position))
            .limit(1);
        const isComposable =
            top !== undefined &&
            top.state === "queued" &&
            top.origin === origin &&
            top.branch === null;
        const latest = isComposable ? { id: top.id, calls: top.calls, branch: null } : undefined;

        // predict it over every layer
        const recorded = await this.#record(transaction, async (nested) => {
            const predicted = await predict(nested, latest);

            return { calls: predicted.calls, result: predicted };
        });
        const { kind, result } = recorded.value;

        // compose it into the top mutation, extending its layer
        if (kind === "compose") {
            const composed = present(latest, "the mutation a prediction composes into");
            await this.#compose(transaction, composed.id, recorded.calls, recorded.changes);

            return { id: composed.id, result };
        }

        // queue it as a new mutation, its layer on top
        await this.#append(transaction, { id, origin, calls: recorded.calls, branch: undefined });
        const [layered] = await transaction.select({ position: max(layer.position) }).from(layer);
        await transaction.insert(layer).values({
            key: id,
            position: (layered?.position ?? 0) + 1,
            changes: this.#encode(recorded.changes),
            tables: this.#tables(recorded.calls, recorded.changes),
        });

        return { id, result };
    }

    /** Show a branch over the main line, or only the main line. */
    async checkout(database: DatabaseConnection, branch: string | undefined): Promise<void> {
        await database.transaction(
            async (transaction) => {
                // revert, switch the branch shown, and predict again
                await transaction.log.asReplica(() => this.revert(transaction));
                await transaction.delete(checkout);
                if (branch !== undefined) {
                    await transaction.insert(checkout).values({ id: 1, branch });
                }
                await this.replay(transaction);
            },
            { constraints: "deferred" },
        );
    }

    /** List the tables each layer reads or writes, empty without predictions. */
    async layers(database: DatabaseConnection): Promise<string[][]> {
        const rows = await database.select({ tables: layer.tables }).from(layer);

        return rows.map((row) => LayerTables.parse(JSON.parse(row.tables)));
    }

    /** Read each layer's predicted rows before and after, in the order applied. */
    async predicted(database: DatabaseConnection): Promise<RowChange[]> {
        const rows = await database
            .select({ changes: layer.changes })
            .from(layer)
            .orderBy(asc(layer.position));

        return rows.flatMap((row) => this.#decode(row.changes));
    }

    /** Revert every layer, latest first, in a transaction deferring its constraints. */
    async revert(transaction: DatabaseConnection): Promise<void> {
        await this.#revertFrom(transaction, 1);
    }

    /** Settle the queue a completed page covers and predict the stack again over the replica. */
    async replay(transaction: DatabaseConnection, settled?: LogPosition): Promise<void> {
        // drop the mutations whose changes the replica has
        if (settled !== undefined) {
            await transaction.delete(mutation).where(Prediction.#acknowledged(settled));
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

    /** List the layers in apply order: the main line, then the branch's rows and edits. */
    async #stack(transaction: DatabaseConnection): Promise<Layer[]> {
        // read the branch shown and the queue
        const branch = await this.checkedOut(transaction);
        const queued = await this.#queued(transaction);

        return [
            ...queued
                .filter((entry) => entry.branch === null)
                .map((entry) => Prediction.#layered(entry)),
            ...(branch === undefined
                ? []
                : [
                      { kind: "branch" as const, branch },
                      ...queued
                          .filter((entry) => entry.branch === branch)
                          .map((entry) => Prediction.#layered(entry)),
                  ]),
        ];
    }

    /** Predict layers in order from a position, continuing each one's changes from the last. */
    async #apply(
        transaction: DatabaseConnection,
        layers: readonly Layer[],
        position: number,
        after: LogId | undefined,
    ): Promise<void> {
        let cursor = after;
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
                cursor,
            );
            cursor = recorded.after;

            // store its changes and tables
            await transaction.insert(layer).values({
                key: entry.kind === "mutation" ? entry.mutation.id : entry.branch,
                position: position + index,
                changes: this.#encode(recorded.changes),
                tables: this.#tables(recorded.calls, recorded.changes, entry.kind === "branch"),
            });
        }
    }

    /** Read the top layer's mutation when a prediction may compose into it, else absent. */
    async #composable(
        transaction: DatabaseConnection,
        top: Layer | undefined,
        origin: string,
        branch: string | undefined,
    ): Promise<Mutation | undefined> {
        // take only a mutation layer
        if (top?.kind !== "mutation") {
            return undefined;
        }

        // take it while queued, the origin's and on the same line
        const [row] = await transaction
            .select({ origin: mutation.origin, state: mutation.state, branch: mutation.branch })
            .from(mutation)
            .where(eq(mutation.id, top.mutation.id));
        const isComposable =
            row !== undefined &&
            row.state === "queued" &&
            row.origin === origin &&
            row.branch === (branch ?? null);

        return isComposable ? top.mutation : undefined;
    }

    /** Replace a queued mutation's calls with composed ones, and extend its layer by changes. */
    async #compose(
        transaction: DatabaseConnection,
        id: string,
        calls: readonly Call[],
        changes: readonly RowChange[],
    ): Promise<void> {
        // replace the calls
        await transaction
            .update(mutation)
            .set({ calls: [...calls] })
            .where(eq(mutation.id, id));

        // compose the changes into the layer's, each row before the first and after the last
        const [row] = await transaction
            .select({ changes: layer.changes })
            .from(layer)
            .where(eq(layer.key, id));
        const earlier = this.#decode(present(row, "a queued mutation's layer").changes);
        const composed = Prediction.#composed([...earlier, ...changes]);
        await transaction
            .update(layer)
            .set({ changes: this.#encode(composed), tables: this.#tables(calls, composed) })
            .where(eq(layer.key, id));
    }

    /** Write a layer's changes as JSON text. */
    #encode(changes: readonly RowChange[]): string {
        return JSON.stringify(
            changes.map(({ table, before, after }) => ({
                table: table[TABLE].sqlName,
                before: before === null ? null : table[TABLE].encode(before),
                after: after === null ? null : table[TABLE].encode(after),
            })),
        );
    }

    /** Read a layer's changes from JSON text. */
    #decode(changes: string): RowChange[] {
        return LayerChanges.parse(JSON.parse(changes)).map((change) => {
            const table = this.#table(change.table);

            return {
                table,
                before: change.before === null ? null : Table.decode(table, change.before),
                after: change.after === null ? null : Table.decode(table, change.after),
            };
        });
    }

    /** Find a predicted table by its SQL name. */
    #table(name: string): Table {
        const table = this.tables.get(name);
        if (table === undefined) {
            throw new TypeError(`a prediction changed an unknown table: '${name}'`);
        }

        return table;
    }

    /** Revert the layers from a position up, latest first. */
    async #revertFrom(transaction: DatabaseConnection, position: number): Promise<void> {
        // read the layers, latest first
        const rows = await transaction
            .select({ changes: layer.changes })
            .from(layer)
            .where(gte(layer.position, position))
            .orderBy(desc(layer.position));

        // restore each row as it was before the first layer, latest change first
        const inverse = new ChangeSet();
        for (const row of rows) {
            for (const { table, before, after } of this.#decode(row.changes).toReversed()) {
                inverse.write({ table, before: after, after: before });
            }
        }

        // write the restored rows and drop the layers
        await inverse.apply(transaction);
        await transaction.delete(layer).where(gte(layer.position, position));
    }

    /** List the tables a layer reads or writes, with the branch rows' table for a branch. */
    #tables(calls: readonly Call[], changes: readonly RowChange[], isBranch = false): string {
        const tables = new Set([
            ...calls.flatMap((call) => this.#reads(call)),
            ...changes.map((change) => change.table[TABLE].sqlName),
            ...(isBranch && this.#branches !== undefined ? this.#branches.tables : []),
        ]);

        return JSON.stringify([...tables].toSorted());
    }

    /** Run a prediction and read the changes it wrote, with the log id they end at. */
    async #record<Value>(
        transaction: DatabaseConnection,
        predict: (
            transaction: DatabaseConnection,
        ) => Promise<{ readonly calls: readonly Call[]; readonly result: Value }>,
        after?: LogId,
    ): Promise<{
        readonly calls: readonly Call[];
        readonly value: Value;
        readonly changes: RowChange[];
        readonly after: LogId;
    }> {
        // run the prediction after the transaction's earlier writes
        const tables = [...this.tables.values()];
        const before = after ?? (await transaction.log.readTransaction({ tables })).after;
        const { calls, result } = await predict(transaction);

        // read every change it wrote, in write order
        const written = await transaction.log.readTransaction({ tables, after: before });
        const changes = Prediction.#composed(
            written.changes.map((change) => ({
                table: change.table,
                before: Change.before(change),
                after: Change.after(change),
            })),
        );

        return { calls, value: result, changes, after: written.after };
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
            log: null,
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
            .where(ne(mutation.state, "rejected"))
            .orderBy(asc(mutation.position));
    }

    /** Read the mutations waiting for the server, in order, with the branch each pushes to. */
    async pending(
        database: DatabaseConnection,
        options: {
            /** The most mutations to read, every pending one when absent. */
            readonly limit?: number;
        } = {},
    ): Promise<(Mutation & { readonly origin: string; readonly branch?: string })[]> {
        const query = database
            .select({
                id: mutation.id,
                origin: mutation.origin,
                calls: mutation.calls,
                branch: mutation.branch,
            })
            .from(mutation)
            .where(Prediction.#pending())
            .orderBy(asc(mutation.position));
        const rows = await (options.limit === undefined ? query : query.limit(options.limit));

        return rows.map((row) => ({
            id: row.id,
            origin: row.origin,
            calls: row.calls,
            ...(row.branch === null ? {} : { branch: row.branch }),
        }));
    }

    /** Mark the pending mutations sent and read them in order, as a push sends them. */
    async send(
        database: DatabaseConnection,
        options: {
            /** The most mutations to send, every pending one when absent. */
            readonly limit?: number;
        } = {},
    ): Promise<(Mutation & { readonly origin: string; readonly branch?: string })[]> {
        return database.transaction(async (transaction) => {
            // read them and mark them sent in one transaction, so no prediction composes into them
            const pending = await this.pending(transaction, options);
            await transaction
                .update(mutation)
                .set({ state: "sent" })
                .where(
                    inArray(
                        mutation.id,
                        pending.map((entry) => entry.id),
                    ),
                );

            return pending;
        });
    }

    /** Count the pending, executed and rejected mutations. */
    async inspect(
        database: DatabaseConnection,
    ): Promise<{ readonly pending: number; readonly executed: number; readonly rejected: number }> {
        // count each state in one pass with pending as the push reads it
        const pending = Prediction.#pending();
        const [counts] = await database
            .select({
                pending: Prediction.#countWhen(pending),
                executed: Prediction.#countWhen(and(eq(mutation.state, "executed"), not(pending))),
                rejected: Prediction.#countWhen(eq(mutation.state, "rejected")),
            })
            .from(mutation);
        if (counts === undefined) {
            throw new TypeError("a count read returned no row");
        }

        return counts;
    }

    /** Wait until a committed mutation is pending, returning false once the signal aborts. */
    wait(database: DatabaseConnection, signal: AbortSignal): Promise<boolean> {
        return database.log.waitUntil(
            async () => (await this.pending(database, { limit: 1 })).length > 0,
            signal,
        );
    }

    /** Record an executed mutation's home position, settling it once a replica reflects it. */
    async acknowledge(
        transaction: DatabaseConnection,
        id: string,
        position: LogPosition,
    ): Promise<void> {
        // record the position for a replica's later page to settle
        await transaction
            .update(mutation)
            .set({
                state: "executed",
                log: position.log,
                epoch: position.epoch,
                sequence: position.sequence,
            })
            .where(eq(mutation.id, id));

        // find a replica of the predicted tables whose confirming page arrived first
        const [predicted] = await transaction
            .select({ tables: layer.tables })
            .from(layer)
            .where(eq(layer.key, id));
        const tables =
            predicted === undefined ? [] : LayerTables.parse(JSON.parse(predicted.tables));
        const reflections = await Replica.reflections(transaction);
        const reflecting = reflections.find(
            (reflection) =>
                reflection.position.log === position.log &&
                reflection.position.epoch === position.epoch &&
                reflection.position.sequence >= position.sequence &&
                reflection.tables.some((table) => tables.includes(table)),
        );

        // settle at its position
        if (reflecting !== undefined) {
            await transaction.log.asReplica(() => this.revert(transaction));
            await this.replay(transaction, reflecting.position);
        }
    }

    /** Settle executed mutations at once, as no replica would confirm their changes. */
    async settle(transaction: DatabaseConnection, ids: readonly string[]): Promise<void> {
        // revert every layer once, drop the mutations, and predict the rest again once
        if (ids.length === 0) {
            return;
        }
        await transaction.log.asReplica(() => this.revert(transaction));
        await transaction.delete(mutation).where(inArray(mutation.id, [...ids]));
        await this.replay(transaction);
    }

    /** Record the server's rejection of a mutation. */
    async reject(database: DatabaseConnection, id: string, error: Failure): Promise<void> {
        await database
            .update(mutation)
            .set({ state: "rejected", error })
            .where(eq(mutation.id, id));
    }

    /** Report whether a position includes an executed mutation's changes. */
    async includes(transaction: DatabaseConnection, position: LogPosition): Promise<boolean> {
        const [recorded] = await transaction
            .select({ id: mutation.id })
            .from(mutation)
            .where(Prediction.#acknowledged(position))
            .limit(1);

        return recorded !== undefined;
    }

    /** Read mutations' outcomes in one statement. */
    async outcomes(
        database: DatabaseConnection,
        ids: readonly string[],
    ): Promise<ReadonlyMap<string, MutationState>> {
        const rows = await database
            .select({ id: mutation.id, state: mutation.state, error: mutation.error })
            .from(mutation)
            .where(inArray(mutation.id, [...ids]));
        const states = new Map(rows.map((row) => [row.id, row]));

        return new Map(ids.map((id) => [id, Prediction.#mutationState(states.get(id))]));
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
                    eq(mutation.state, "rejected"),
                    id === undefined ? undefined : eq(mutation.id, id),
                ),
            );
    }

    /** Layer a queued mutation's calls. */
    static #layered(queued: Mutation): Layer {
        return { kind: "mutation", mutation: queued };
    }

    /** Compose row changes in order: each row before the first and after the last. */
    static #composed(changes: readonly RowChange[]): RowChange[] {
        // write each change into one set, folding rewrites of a row
        const set = new ChangeSet();
        for (const change of changes) {
            set.write(change);
        }

        return [...set.changes];
    }

    /** Match the mutations to push: unexecuted, or executed at an epoch the log's replicas left. */
    static #pending(): SQL {
        // push a queued or sent mutation, or an executed one at an epoch no replica of its log reflects
        const isUnexecuted = inArray(mutation.state, ["queued", "sent"]);
        const isLost = sql`${eq(mutation.state, "executed")} AND ${Replica.reflects(mutation.log)} AND NOT ${Replica.reflects(mutation.log, mutation.epoch)}`;

        return sql`(${isUnexecuted} OR (${isLost}))`;
    }

    /** Match the executed mutations whose changes a position includes. */
    static #acknowledged(position: LogPosition): SQL | undefined {
        return and(
            eq(mutation.state, "executed"),
            eq(mutation.log, position.log),
            eq(mutation.epoch, position.epoch),
            lte(mutation.sequence, position.sequence),
        );
    }

    /** Count the rows meeting a condition. */
    static #countWhen(condition: SQL | undefined): SQL<number> {
        return count(sql`CASE WHEN ${condition} THEN 1 END`);
    }

    /** Read a mutation's state from its row, settled once the outbox dropped it. */
    static #mutationState(
        row:
            | {
                  readonly state: (typeof MUTATION_STATES)[number];
                  readonly error: Failure | null;
              }
            | undefined,
    ): MutationState {
        // read a settled mutation
        if (row === undefined) {
            return { kind: "settled" };
        }
        // read a rejected one with its failure
        else if (row.state === "rejected") {
            return { kind: "rejected", error: present(row.error, "a rejected mutation's failure") };
        }
        // read any other state
        else {
            return { kind: row.state };
        }
    }
}
