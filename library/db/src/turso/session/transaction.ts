import { sql } from "drizzle-orm";
import { SQLiteAsyncTransaction } from "drizzle-orm/sqlite-core";
import type { AnyRelations } from "drizzle-orm/relations";
import type { DrizzleSQLiteConfig } from "drizzle-orm/sqlite-core/utils";
import { Session } from "./session.ts";
import type { QueryClient } from "./client.ts";

/** Drizzle queries scoped to one transaction or savepoint. */
export class Transaction<Result, Relations extends AnyRelations>
    extends SQLiteAsyncTransaction<"async", Result, Relations> {}

/** Queries and savepoints within one SQLite transaction. */
export class TransactionSession<Result, Relations extends AnyRelations>
    extends Session<Result, Relations> {
    /** The relation definitions available to nested transactions. */
    readonly relations: Relations;
    /** The current savepoint depth. */
    readonly depth: number;

    /** Retain the scoped transaction handle. */
    constructor(
        client: QueryClient<Result>,
        relations: Relations,
        options: DrizzleSQLiteConfig<Relations>,
        depth: number,
    ) {
        super(client, options);
        this.relations = relations;
        this.depth = depth;
    }

    /** Execute a nested transaction using a savepoint. */
    async transaction<Value>(
        operation: (transaction: Transaction<Result, Relations>) => Promise<Value>,
    ): Promise<Value> {
        const name = sql.identifier(`destack_savepoint_${this.depth}`);
        await this.run(sql`SAVEPOINT ${name}`);
        try {
            const session = new TransactionSession(
                this.client,
                this.relations,
                this.options,
                this.depth + 1,
            );
            const result = await operation(
                new Transaction("async", session.dialect, session, this.relations),
            );
            await this.run(sql`RELEASE SAVEPOINT ${name}`);

            return result;
        } catch (error) {
            // preserve both failures when the savepoint cannot be restored
            try {
                await this.run(sql`ROLLBACK TO SAVEPOINT ${name}`);
                await this.run(sql`RELEASE SAVEPOINT ${name}`);
            } catch (rollback) {
                throw new AggregateError(
                    [error, rollback],
                    "SQLite savepoint and rollback failed.",
                );
            }

            throw error;
        }
    }
}
