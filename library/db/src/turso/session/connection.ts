import type { SQLiteTransactionConfig } from "drizzle-orm/sqlite-core";
import type { AnyRelations } from "drizzle-orm/relations";
import type { DrizzleSQLiteConfig } from "drizzle-orm/sqlite-core/utils";
import { Session } from "./session.ts";
import type { ConnectionClient } from "./client.ts";
import { Transaction, TransactionSession } from "./transaction.ts";

/** A query session that starts transactions through SQLite. */
export class ConnectionSession<Result, Relations extends AnyRelations> extends Session<
    Result,
    Relations
> {
    /** The physical connection used to create transaction handles. */
    readonly connection: ConnectionClient<Result>;
    /** The relations shared by queries and transactions. */
    readonly relations: Relations;

    /** Retain the physical connection and relation definitions. */
    constructor(
        client: ConnectionClient<Result>,
        relations: Relations,
        options: DrizzleSQLiteConfig<Relations>,
    ) {
        super(client, options);
        this.connection = client;
        this.relations = relations;
    }

    /** Commit or roll back a callback on a dedicated transaction handle. */
    transaction<Value>(
        operation: (transaction: Transaction<Result, Relations>) => Promise<Value>,
        configuration?: SQLiteTransactionConfig,
    ): Promise<Value> {
        const transaction = this.connection.transactionAsync(async (client) => {
            const session = new TransactionSession(client, this.relations, this.options, 0);
            return await operation(
                new Transaction("async", session.dialect, session, this.relations),
            );
        });

        return transaction[configuration?.behavior ?? "deferred"]();
    }
}
