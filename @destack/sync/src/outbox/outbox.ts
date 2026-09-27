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

/** One method call within a mutation, named `object.method`. */
export const Call = defineSchema(
    schema.object({
        /** The object type and method, such as page.create. */
        method: schema.string().min(1),
        /** The method's input, its scope and target included. */
        input: schema.record(schema.string(), schema.json()),
    }),
);
/** One method call within a mutation, named `object.method`. */
export type Call = schema.Infer<typeof Call>;

/** Calls a client makes atomically: one server transaction and one journal entry. */
export const Mutation = defineSchema(
    schema.object({
        /** The request identifier, minted once on the client. */
        id: schema.uuidv7(),
        /** The calls in order. */
        calls: schema.array(Call).min(1),
    }),
);
/** Calls a client makes atomically: one server transaction and one journal entry. */
export type Mutation = schema.Infer<typeof Mutation>;

/** The mutations in a client's outbox, in order, with the local changes predicting them. */
export const mutation = defineTable("mutation", {
    /** The request identifier. */
    id: text("id").primaryKey().validate(schema.uuidv7()),
    /** The party that queued the mutation, such as a browser tab, which reads its outcome. */
    origin: text("origin").notNull(),
    /** The mutation's place in the outbox. */
    position: integer("position").notNull().unique(),
    /** The calls. */
    calls: json("calls", schema.array(Call)).notNull(),
    /** The local changes predicting the mutation, reverted before each rebase. */
    changes: json("changes", schema.array(RowChange)).notNull(),
    /** The server log's epoch in which the server executed the mutation, absent while pending. */
    epoch: text("epoch"),
    /** The server log sequence holding the mutation's changes within the epoch. */
    sequence: integer("sequence"),
    /** The failure the server recorded, once it rejected the mutation; its origin reads and removes it. */
    error: json("error", schema.json()),
});

/** A client's durable, ordered outbox of mutations on SQLite, predicted locally and confirmed by its replica. */
export class Outbox {
    /** The tables predictions change, which must log every column. */
    readonly tables: ReadonlyMap<string, Table>;
    /** Predict a mutation again inside a transaction, as its calls first did. */
    readonly #predict: (transaction: DatabaseConnection, mutation: Mutation) => Promise<unknown>;

    /** Hold mutations predicted over tables. */
    constructor(
        tables: readonly Table[],
        predict: (transaction: DatabaseConnection, mutation: Mutation) => Promise<unknown>,
    ) {
        this.tables = new Map(tables.map((table) => [table[TABLE].sqlName, table]));
        this.#predict = predict;
    }

    /** Predict a mutation in one local transaction and add it, returning what its calls returned. */
    async add<Result>(
        database: DatabaseConnection,
        id: string,
        origin: string,
        predict: (
            transaction: DatabaseConnection,
        ) => Promise<{ readonly calls: readonly Call[]; readonly result: Result }>,
    ): Promise<Result> {
        // predict and append the mutation in one local transaction
        return database.transaction(async (transaction) => {
            // predict the calls, noting the local changes they make
            const { calls, result } = await this.#record(transaction, predict);

            // append the mutation to the outbox
            const [last] = await transaction
                .select({ position: max(mutation.position) })
                .from(mutation);
            await transaction.insert(mutation).values({
                id,
                origin,
                position: (last?.position ?? 0) + 1,
                calls: [...calls],
                changes: result.changes,
                sequence: null,
                epoch: null,
                error: null,
            });

            return result.value;
        });
    }

    /** Count the mutations for inspection: those waiting for the server, those it executed that the copy does not hold yet, and those it rejected. */
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
            .where(and(isNull(mutation.epoch), isNull(mutation.error)))
            .orderBy(asc(mutation.position));

        return rows.map((row) => ({ id: row.id, calls: row.calls }));
    }

    /** Read the local changes predicting the mutations the replica does not hold yet, in order. */
    async predicted(database: DatabaseConnection): Promise<RowChange[]> {
        const rows = await database
            .select({ changes: mutation.changes })
            .from(mutation)
            .where(isNull(mutation.error))
            .orderBy(asc(mutation.position));

        return rows.flatMap((row) => row.changes);
    }

    /** Wait until a committed mutation waits for the server; false once the signal aborts. */
    wait(database: DatabaseConnection, signal: AbortSignal): Promise<boolean> {
        return database.log.until(async () => (await this.pending(database)).length > 0, signal);
    }

    /** Record the log position holding a mutation the server executed, so it stops pushing it. */
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

    /** Read a mutation's outcome: pending, executed once its row is gone, or the failure the server recorded. */
    async outcome(
        database: DatabaseConnection,
        id: string,
    ): Promise<
        | { readonly kind: "pending" | "executed" }
        | { readonly kind: "rejected"; readonly error: unknown }
    > {
        const [row] = await database
            .select({ error: mutation.error })
            .from(mutation)
            .where(eq(mutation.id, id));

        return row === undefined
            ? { kind: "executed" }
            : row.error === null
              ? { kind: "pending" }
              : { kind: "rejected", error: row.error };
    }

    /** Remove the rejected mutations of an origin, once it read them or went away. */
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

    /** Record the server's rejection of a mutation, whose prediction the next rebase drops. */
    async reject(database: DatabaseConnection, id: string, error: unknown): Promise<void> {
        await database
            .update(mutation)
            .set({ error: schema.json().parse(error) })
            .where(eq(mutation.id, id));
    }

    /** Revert every prediction the database holds, latest first. */
    async revert(transaction: DatabaseConnection): Promise<void> {
        const rows = await transaction
            .select({ changes: mutation.changes })
            .from(mutation)
            .orderBy(asc(mutation.position));
        for (const row of rows.reverse()) {
            for (const change of [...row.changes].reverse()) {
                await this.#invert(transaction, change);
            }
        }
    }

    /** Drop the mutations a page settled, a completed snapshot's position included, and predict the rest again. */
    async replay(
        transaction: DatabaseConnection,
        outcomes: readonly MutationOutcome[],
        snapshot?: LogPosition,
    ): Promise<void> {
        // drop what the server executed, and keep what it rejected for its origin without its prediction
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
        await transaction.update(mutation).set({ changes: [] }).where(isNotNull(mutation.error));

        // drop what a snapshot holds whose outcome the server pruned already
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

        // push again what an earlier epoch executed, since the restore that began the snapshot may have lost it
        if (snapshot !== undefined) {
            await transaction
                .update(mutation)
                .set({ epoch: null, sequence: null })
                .where(and(isNotNull(mutation.epoch), lt(mutation.epoch, snapshot.epoch)));
        }

        // predict the rest again in order, noting their new local changes
        const rows = await transaction
            .select({ id: mutation.id, calls: mutation.calls })
            .from(mutation)
            .where(isNull(mutation.error))
            .orderBy(asc(mutation.position));
        for (const row of rows) {
            const { result } = await this.#record(transaction, async (inner) => ({
                calls: row.calls,
                result: await this.#predict(inner, row),
            }));
            await transaction
                .update(mutation)
                .set({ changes: result.changes })
                .where(eq(mutation.id, row.id));
        }
    }

    /** Run a prediction and read the local changes it logged. */
    async #record<Value>(
        transaction: DatabaseConnection,
        predict: (
            transaction: DatabaseConnection,
        ) => Promise<{ readonly calls: readonly Call[]; readonly result: Value }>,
    ): Promise<{
        readonly calls: readonly Call[];
        readonly result: { readonly value: Value; readonly changes: RowChange[] };
    }> {
        // require SQLite, whose log stamps each change as it happens, so the transaction reads its own
        if (transaction.dialect !== "sqlite") {
            throw new TypeError(
                "an outbox predicts on SQLite, whose log sequences changes within a transaction",
            );
        }

        // run the prediction after the latest logged change
        const tables = [...this.tables.values()];
        const before = await transaction.log.latest();
        const { calls, result } = await predict(transaction);

        // read every change it logged, page by page
        const changes: RowChange[] = [];
        let after = before;
        for (;;) {
            const page = await transaction.log.read({ tables, after });
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
            if (page.changes.length === 0 || page.sequence === after) {
                break;
            }
            after = page.sequence;
        }

        return { calls, result: { value: result, changes } };
    }

    /** Undo one predicted change: remove an insertion, restore an update, reinsert a deletion. */
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
