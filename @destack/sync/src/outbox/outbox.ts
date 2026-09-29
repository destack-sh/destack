import {
    and,
    asc,
    defineTable,
    eq,
    integer,
    isNull,
    json,
    Key,
    lt,
    lte,
    max,
    ne,
    or,
    inArray,
    isNotNull,
    text,
    TABLE,
    type DatabaseConnection,
    type Table,
    encodeRow,
    decodeRow,
} from "@destack/db";
import { Condition } from "@destack/db/query";
import { defineSchema, schema } from "@destack/schema";
import type { LogPosition } from "@destack/db/log";
import { RowChange, type MutationOutcome } from "../query/page.ts";

/** The most changes one read of a prediction takes. */
const RECORD_CHANGES = 1000;

/** One method call within a mutation. */
export const Call = defineSchema(
    schema.object({
        /** The object type and method, such as page.create. */
        method: schema.string().min(1),
        /** The method's input, with its scope and target. */
        input: schema.record(schema.string(), schema.json()),
        /** The object type version of the call, the first when absent. */
        version: schema.number().int().min(2).optional(),
    }),
);
/** One method call within a mutation. */
export type Call = schema.Infer<typeof Call>;

/** Calls a client makes atomically. */
export const Mutation = defineSchema(
    schema.object({
        /** The request identifier. */
        id: schema.uuidv7(),
        /** The calls in order. */
        calls: schema.array(Call).min(1),
    }),
);
/** Calls a client makes atomically. */
export type Mutation = schema.Infer<typeof Mutation>;

/** The state of a mutation. */
export type Outcome =
    | {
          /** Waiting for the server, or executed. */
          readonly kind: "pending" | "executed";
      }
    | {
          /** Rejected by the server. */
          readonly kind: "rejected";
          /** The failure the server recorded. */
          readonly error: unknown;
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
    /** The mutation's predicted changes, as JSON text. */
    changes: text("changes").notNull(),
    /** The tables its prediction reads or writes, as a JSON array of SQL names. */
    reach: text("reach").notNull(),
    /** The server log's epoch of the executed mutation. */
    epoch: text("epoch"),
    /** The server log sequence of the mutation's changes. */
    sequence: integer("sequence"),
    /** The failure the server recorded. */
    error: json("error", schema.json()),
    /** The branch holding the mutation, absent on the main line. */
    branch: text("branch"),
});

/** The branch a client's copy shows over the main line. */
export const checkout = defineTable("checkout", {
    /** The one row's key. */
    slot: integer("slot").primaryKey(),
    /** The checked-out branch. */
    branch: text("branch").notNull(),
});

/** The tables of a client's outbox. */
export const outboxTables = [mutation, checkout] as const;

/** A client's durable, ordered outbox of mutations, predicted locally. */
export class Outbox {
    /** The tables predictions change. */
    readonly tables: ReadonlyMap<string, Table>;
    /** Predict a mutation inside a transaction. */
    readonly #predict: (transaction: DatabaseConnection, mutation: Mutation) => Promise<unknown>;
    /** Name the tables a call's prediction may read. */
    readonly #reach: (call: Call) => readonly string[];

    /** Create the outbox. */
    constructor(
        tables: readonly Table[],
        predict: (transaction: DatabaseConnection, mutation: Mutation) => Promise<unknown>,
        reach: (call: Call) => readonly string[],
    ) {
        this.tables = new Map(tables.map((table) => [table[TABLE].sqlName, table]));
        this.#predict = predict;
        this.#reach = reach;
    }

    /** Decide whether a page changing some tables reaches a pending prediction. */
    async reaches(database: DatabaseConnection, changed: ReadonlySet<string>): Promise<boolean> {
        const rows = await database
            .select({ reach: mutation.reach })
            .from(mutation)
            .where(isNull(mutation.error));

        return rows.some((row) =>
            (JSON.parse(row.reach) as string[]).some((table) => changed.has(table)),
        );
    }

    /** Name the tables a mutation's prediction reads or writes. */
    #reached(calls: readonly Call[], changes: readonly RowChange[]): string {
        const tables = new Set([
            ...calls.flatMap((call) => this.#reach(call)),
            ...changes.map((change) => change.table),
        ]);

        return JSON.stringify([...tables].sort());
    }

    /** Predict and add a mutation, returning its calls' result. */
    async add<Result>(
        database: DatabaseConnection,
        id: string,
        origin: string,
        predict: (
            transaction: DatabaseConnection,
        ) => Promise<{ readonly calls: readonly Call[]; readonly result: Result }>,
    ): Promise<Result> {
        // predict and append in one transaction
        return database.transaction(async (transaction) => {
            // predict the calls and note their changes
            const { calls, result } = await this.#record(transaction, predict);
            const branch = await this.checkedOut(transaction);

            // append the mutation to the outbox
            const [last] = await transaction
                .select({ position: max(mutation.position) })
                .from(mutation);
            await transaction.insert(mutation).values({
                id,
                origin,
                position: (last?.position ?? 0) + 1,
                calls: [...calls],
                changes: JSON.stringify(result.changes),
                reach: this.#reached(calls, result.changes),
                sequence: null,
                epoch: null,
                error: null,
                branch: branch ?? null,
            });

            return result.value;
        });
    }

    /** Count the pending, unconfirmed and rejected mutations. */
    async inspect(
        database: DatabaseConnection,
    ): Promise<{ readonly pending: number; readonly executed: number; readonly rejected: number }> {
        const rows = await database
            .select({ epoch: mutation.epoch, error: mutation.error })
            .from(mutation);

        return {
            pending: rows.filter((row) => row.epoch === null && row.error === null).length,
            executed: rows.filter((row) => row.epoch !== null && row.error === null).length,
            rejected: rows.filter((row) => row.error !== null).length,
        };
    }

    /** Read the mutations waiting for the server, in order. */
    async pending(database: DatabaseConnection): Promise<Mutation[]> {
        const rows = await database
            .select({ id: mutation.id, calls: mutation.calls })
            .from(mutation)
            .where(and(isNull(mutation.epoch), isNull(mutation.error), isNull(mutation.branch)))
            .orderBy(asc(mutation.position));

        return rows.map((row) => ({ id: row.id, calls: row.calls }));
    }

    /** Read the checked-out branch. */
    async checkedOut(database: DatabaseConnection): Promise<string | undefined> {
        const [row] = await database.select({ branch: checkout.branch }).from(checkout);

        return row?.branch;
    }

    /** List the branches holding mutations. */
    async branches(database: DatabaseConnection): Promise<string[]> {
        const rows = await database
            .selectDistinct({ branch: mutation.branch })
            .from(mutation)
            .where(isNotNull(mutation.branch))
            .orderBy(asc(mutation.branch));

        return rows.map((row) => row.branch!);
    }

    /** Show a branch over the main line, or the main line alone. */
    async checkout(database: DatabaseConnection, branch: string | undefined): Promise<void> {
        await this.#rebase(database, async (transaction) => {
            await transaction.delete(checkout);
            if (branch !== undefined) {
                await transaction.insert(checkout).values({ slot: 1, branch });
            }
        });
    }

    /** Move a branch's mutations onto the main line after its pending ones. */
    async merge(database: DatabaseConnection, branch: string): Promise<void> {
        await this.#rebase(database, async (transaction) => {
            // number the branch's mutations after every other
            const [last] = await transaction
                .select({ position: max(mutation.position) })
                .from(mutation);
            const moved = await transaction
                .select({ id: mutation.id })
                .from(mutation)
                .where(eq(mutation.branch, branch))
                .orderBy(asc(mutation.position));
            for (const [index, row] of moved.entries()) {
                await transaction
                    .update(mutation)
                    .set({ position: (last?.position ?? 0) + index + 1, branch: null })
                    .where(eq(mutation.id, row.id));
            }

            // check out the main line after merging the checked-out branch
            await transaction.delete(checkout).where(eq(checkout.branch, branch));
        });
    }

    /** Remove a branch's mutations. */
    async discard(database: DatabaseConnection, branch: string): Promise<void> {
        await this.#rebase(database, async (transaction) => {
            await transaction.delete(mutation).where(eq(mutation.branch, branch));
            await transaction.delete(checkout).where(eq(checkout.branch, branch));
        });
    }

    /** Change the outbox between reverting and predicting again. */
    async #rebase(
        database: DatabaseConnection,
        change: (transaction: DatabaseConnection) => Promise<void>,
    ): Promise<void> {
        await database.transaction(async (transaction) => {
            await transaction.log.copying(() => this.revert(transaction));
            await change(transaction);
            await this.replay(transaction, []);
        });
    }

    /** Read the predicted changes of unconfirmed mutations, in order. */
    async predicted(database: DatabaseConnection): Promise<RowChange[]> {
        const rows = await database
            .select({ changes: mutation.changes })
            .from(mutation)
            .where(isNull(mutation.error))
            .orderBy(asc(mutation.position));

        return rows.flatMap((row) => JSON.parse(row.changes) as RowChange[]);
    }

    /** Wait until a committed mutation is pending, returning false once the signal aborts. */
    wait(database: DatabaseConnection, signal: AbortSignal): Promise<boolean> {
        return database.log.until(async () => (await this.pending(database)).length > 0, signal);
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

    /** Read mutations' outcomes in one statement. */
    async outcomes(
        database: DatabaseConnection,
        ids: readonly string[],
    ): Promise<ReadonlyMap<string, Outcome>> {
        const rows = await database
            .select({ id: mutation.id, error: mutation.error })
            .from(mutation)
            .where(inArray(mutation.id, [...ids]));
        const held = new Map(rows.map((row) => [row.id, row.error]));

        return new Map(
            ids.map((id): [string, Outcome] => {
                const error = held.get(id);

                return [
                    id,
                    error === undefined
                        ? { kind: "executed" }
                        : error === null
                          ? { kind: "pending" }
                          : { kind: "rejected", error },
                ];
            }),
        );
    }

    /** Read the origins holding mutations. */
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

    /** Record the server's rejection of a mutation. */
    async reject(database: DatabaseConnection, id: string, error: unknown): Promise<void> {
        await database
            .update(mutation)
            .set({ error: schema.json().parse(error) })
            .where(eq(mutation.id, id));
    }

    /** Revert every prediction, latest first. */
    async revert(transaction: DatabaseConnection): Promise<void> {
        const rows = await transaction
            .select({ changes: mutation.changes })
            .from(mutation)
            .orderBy(asc(mutation.position));
        for (const row of rows.reverse()) {
            for (const change of (JSON.parse(row.changes) as RowChange[]).reverse()) {
                await this.#invert(transaction, change);
            }
        }
    }

    /** Drop the mutations a page settled and predict the rest again. */
    async replay(
        transaction: DatabaseConnection,
        outcomes: readonly MutationOutcome[],
        snapshot?: LogPosition,
    ): Promise<void> {
        // drop executed mutations and keep rejected ones without their prediction
        const executed = outcomes.filter((entry) => entry.error === undefined);
        if (executed.length > 0) {
            await transaction.delete(mutation).where(
                Condition.render(
                    Condition.oneOf(
                        "id",
                        executed.map((entry) => entry.id),
                    ),
                    Condition.bind(mutation),
                ),
            );
        }
        for (const entry of outcomes.filter((outcome) => outcome.error !== undefined)) {
            await transaction
                .update(mutation)
                .set({ error: schema.json().parse(entry.error) })
                .where(eq(mutation.id, entry.id));
        }
        await transaction.update(mutation).set({ changes: "[]" }).where(isNotNull(mutation.error));

        // drop what a snapshot holds
        if (snapshot !== undefined) {
            await transaction
                .delete(mutation)
                .where(
                    and(
                        isNull(mutation.error),
                        eq(mutation.epoch, snapshot.epoch),
                        lte(mutation.sequence, snapshot.sequence),
                    ),
                );
        }

        // push again what an earlier epoch executed
        if (snapshot !== undefined) {
            await transaction
                .update(mutation)
                .set({ epoch: null, sequence: null })
                .where(and(isNotNull(mutation.epoch), lt(mutation.epoch, snapshot.epoch)));
        }

        // predict the main line, then the checked-out branch
        const branch = await this.checkedOut(transaction);
        await transaction
            .update(mutation)
            .set({ changes: "[]" })
            .where(
                and(
                    isNotNull(mutation.branch),
                    branch === undefined ? undefined : ne(mutation.branch, branch),
                ),
            );
        const rows = await transaction
            .select({ id: mutation.id, calls: mutation.calls, branch: mutation.branch })
            .from(mutation)
            .where(
                and(
                    isNull(mutation.error),
                    branch === undefined
                        ? isNull(mutation.branch)
                        : or(isNull(mutation.branch), eq(mutation.branch, branch)),
                ),
            )
            .orderBy(asc(mutation.position));
        const ordered = [
            ...rows.filter((row) => row.branch === null),
            ...rows.filter((row) => row.branch !== null),
        ];
        let reached: number | undefined;
        for (const row of ordered) {
            // continue each prediction's changes from the last one's
            const { result, sequence } = await this.#record(
                transaction,
                async (inner) => ({ calls: row.calls, result: await this.#predict(inner, row) }),
                reached,
            );
            reached = sequence;
            await transaction
                .update(mutation)
                .set({
                    changes: JSON.stringify(result.changes),
                    reach: this.#reached(row.calls, result.changes),
                })
                .where(eq(mutation.id, row.id));
        }
    }

    /** Run a prediction and read the changes it logged, returning the sequence they reach. */
    async #record<Value>(
        transaction: DatabaseConnection,
        predict: (
            transaction: DatabaseConnection,
        ) => Promise<{ readonly calls: readonly Call[]; readonly result: Value }>,
        after?: number,
    ): Promise<{
        readonly calls: readonly Call[];
        readonly result: { readonly value: Value; readonly changes: RowChange[] };
        readonly sequence: number;
    }> {
        // require SQLite
        if (transaction.dialect !== "sqlite") {
            throw new TypeError(
                "an outbox predicts on SQLite, whose log sequences changes within a transaction",
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

        return { calls, result: { value: result, changes }, sequence };
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
