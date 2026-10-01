import {
    and,
    asc,
    count,
    defineTable,
    eq,
    integer,
    isNull,
    json,
    lt,
    lte,
    max,
    inArray,
    isNotNull,
    text,
    sql,
    type DatabaseConnection,
    type SQL,
} from "@destack/db";
import { Condition } from "@destack/db/query";
import { schema } from "@destack/schema";
import type { LogPosition } from "@destack/db/log";
import type { MutationOutcome } from "../query/page.ts";
import { Call, Failure, Mutation } from "../call/index.ts";

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

/** A client's durable, ordered queue of mutations waiting for the server. */
export class Outbox {
    /** Append a mutation after every other. */
    async append(
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
    async queued(
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

    /** Count the pending, unconfirmed and rejected mutations. */
    async inspect(
        database: DatabaseConnection,
    ): Promise<{ readonly pending: number; readonly executed: number; readonly rejected: number }> {
        // count each state in one pass, pending as the push reads it
        const when = (condition: SQL | undefined) => count(sql`CASE WHEN ${condition} THEN 1 END`);
        const [counts] = await database
            .select({
                pending: when(and(isNull(mutation.epoch), isNull(mutation.error))),
                executed: when(and(isNotNull(mutation.epoch), isNull(mutation.error))),
                rejected: when(isNotNull(mutation.error)),
            })
            .from(mutation);

        return counts!;
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

    /** Settle the mutations a page answered or a snapshot holds. */
    async settle(
        transaction: DatabaseConnection,
        outcomes: readonly MutationOutcome[],
        snapshot?: LogPosition,
    ): Promise<void> {
        // drop executed mutations
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

        // keep rejected mutations with their failure
        for (const { id, error } of outcomes) {
            if (error !== undefined) {
                await this.reject(transaction, id, error);
            }
        }

        // drop what a snapshot holds, and push again what an earlier epoch executed
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
            await transaction
                .update(mutation)
                .set({ epoch: null, sequence: null })
                .where(and(isNotNull(mutation.epoch), lt(mutation.epoch, snapshot.epoch)));
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
        const held = new Map(rows.map((row) => [row.id, row.error]));

        return new Map(
            ids.map((id): [string, MutationState] => {
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
