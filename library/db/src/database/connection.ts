import { assertNever, DatabaseError } from "../error/error.ts";
import { DatabaseDriver, type NativeDatabase } from "./driver.ts";
import type { SchemaCompiler } from "../dialect/compiler.ts";
import type { Table } from "../table/table.ts";
import { SelectBuilder, type SelectedSubquery, type SelectQuery } from "../query/select.ts";
import type { SQL, WithSubquery } from "drizzle-orm";
import { MutationQuery } from "../query/mutation.ts";
import type { Selection } from "../query/selection.ts";
import { type TransactionOptions, TransactionState } from "./transaction.ts";
import type { Dialect } from "../dialect/dialect.ts";
import type { TableRelations } from "../schema/relation.ts";
import type { DatabaseSchema } from "../schema/schema.ts";
import { bindRelations, type RelationalQueries } from "../query/relation.ts";

/** Portable queries over a materialized database schema. */
export class DatabaseConnection<
    Driver extends Dialect = Dialect,
    Relations extends Record<string, TableRelations> = {},
> {
    /** The native Drizzle database and its query lifetime. */
    readonly connection: DatabaseDriver;
    /** The concrete schema used by queries and migration tooling. */
    readonly schema: SchemaCompiler<Driver>;
    /** Logical relations shared by root and transactional queries. */
    readonly relations: Relations;
    /** Relational query builders using this connection's current session. */
    readonly query: RelationalQueries<Relations>;
    /** The physical connection's submission and shutdown state. */
    readonly state: ConnectionState;
    /** Schema-specific queries sharing this connection's physical tables. */
    readonly #bindings = new WeakMap<DatabaseSchema, DatabaseConnection>();

    /** Bind logical tables to a physical connection. */
    constructor(
        connection: DatabaseDriver,
        declarations: SchemaCompiler<Driver>,
        relations: Relations = {} as Relations,
    ) {
        this.state = connection.state;
        this.connection = connection;
        this.schema = declarations;
        this.relations = relations;
        this.query = bindRelations(this.connection, this.schema, relations);
    }

    /** Bind a declared schema to this connection and reuse its query builders. */
    bind<BoundRelations extends Record<string, TableRelations>>(
        definition: DatabaseSchema<Record<string, Table>, BoundRelations>,
    ): DatabaseConnection<Driver, BoundRelations> {
        this.connection.transaction?.assertActive();

        // require each bound table to belong to the prepared physical schema
        for (const table of Object.values(definition.tables)) this.schema.table(table);

        // retain one binding per declaration and physical connection
        let bound = this.#bindings.get(definition);
        if (!bound) {
            bound = new DatabaseConnection<Dialect>(
                this.connection,
                this.schema,
                definition.relations ?? {},
            );
            this.#bindings.set(definition, bound);
        }

        return bound as DatabaseConnection<Driver, BoundRelations>;
    }

    /** Select application records or explicit fields. */
    select<Fields extends Selection | undefined = undefined>(
        fields?: Fields,
    ): SelectBuilder<Fields> {
        return new SelectBuilder(this.connection, this.schema, fields as Fields);
    }

    /** Select distinct application records or explicit fields. */
    selectDistinct<Fields extends Selection | undefined = undefined>(
        fields?: Fields,
    ): SelectBuilder<Fields> {
        return new SelectBuilder(this.connection, this.schema, fields as Fields, true);
    }

    /** Declare a named common table expression using Drizzle's query description. */
    $with<const Alias extends string>(alias: Alias) {
        return {
            as: <
                Result,
                Fields extends Selection,
                Nullable extends string,
                Automatic extends boolean,
            >(
                query: SelectQuery<Result, Fields, Nullable, Automatic>,
            ): SelectedSubquery<Result, Alias> => {
                // select the native CTE constructor for this connection
                if (this.connection.native.dialect === "sqlite") {
                    return this.connection.native.database.$with(alias).as(
                        query,
                    ) as unknown as SelectedSubquery<Result, Alias>;
                } else if (this.connection.native.dialect === "postgresql") {
                    return this.connection.native.database.$with(alias).as(
                        query,
                    ) as unknown as SelectedSubquery<Result, Alias>;
                } else {
                    return assertNever(this.connection.native);
                }
            },
        };
    }

    /** Include named common table expressions in the next selection. */
    with(...queries: WithSubquery[]) {
        return {
            select: <Fields extends Selection | undefined = undefined>(fields?: Fields) =>
                new SelectBuilder(this.connection, this.schema, fields as Fields, false, queries),
            selectDistinct: <Fields extends Selection | undefined = undefined>(fields?: Fields) =>
                new SelectBuilder(this.connection, this.schema, fields as Fields, true, queries),
        };
    }

    /** Insert application records. */
    insert<Definition extends Table>(table: Definition): MutationQuery<Definition, void, "insert"> {
        return new MutationQuery(this.connection, this.schema, table, "insert");
    }

    /** Update application records. */
    update<Definition extends Table>(table: Definition): MutationQuery<Definition, void, "update"> {
        return new MutationQuery(this.connection, this.schema, table, "update");
    }

    /** Delete application records. */
    delete<Definition extends Table>(table: Definition): MutationQuery<Definition, void, "delete"> {
        return new MutationQuery(this.connection, this.schema, table, "delete");
    }

    /** Execute explicit SQL and return its driver rows. */
    async execute<Row extends Record<string, unknown> = Record<string, unknown>>(
        statement: SQL,
    ): Promise<Row[]> {
        return await this.connection.run(async () => {
            const query = this.schema.expression(statement);
            if (this.connection.native.dialect === "sqlite") {
                return await this.connection.native.database.all<Row>(query);
            } else if (this.connection.native.dialect === "postgresql") {
                return Array.from(
                    await this.connection.native.database.execute<Row>(query),
                ) as Row[];
            } else {
                return assertNever(this.connection.native);
            }
        });
    }

    /** Commit a callback once, or roll back all its changes on failure. */
    async transaction<Value>(
        operation: (transaction: DatabaseConnection<Dialect, Relations>) => Promise<Value>,
        options: TransactionOptions = {},
    ): Promise<Value> {
        this.connection.transaction?.assertActive();

        // propagate cancellation from both the caller and an enclosing transaction
        const signals = [this.connection.transaction?.signal, options.signal].filter(
            (signal): signal is AbortSignal => signal !== undefined,
        );
        const signal = signals.length ? AbortSignal.any(signals) : undefined;
        signal?.throwIfAborted();

        // track nested transactions until their enclosing callback can finish
        const execute = async () => {
            if (this.connection.native.dialect === "sqlite") {
                return await this.connection.native.database.transaction(
                    (transaction) =>
                        this.transact(
                            { dialect: "sqlite", database: transaction },
                            operation,
                            signal,
                        ),
                    { behavior: "immediate" },
                );
            } else if (this.connection.native.dialect === "postgresql") {
                return await this.connection.native.database.transaction(
                    (transaction) =>
                        this.transact(
                            { dialect: "postgresql", database: transaction },
                            operation,
                            signal,
                        ),
                    { isolationLevel: options.isolationLevel ?? "repeatable read" },
                );
            } else {
                return assertNever(this.connection.native);
            }
        };

        return this.connection.transaction
            ? await this.connection.transaction.run(execute, false)
            : await this.state.run(execute);
    }

    /** Bind a transaction session and drain its submitted queries before completion. */
    private transact<Value>(
        connection: NativeDatabase,
        operation: (transaction: DatabaseConnection<Dialect, Relations>) => Promise<Value>,
        signal?: AbortSignal,
    ): Promise<Value> {
        const state = new TransactionState(signal);
        const transaction = new DatabaseConnection<Dialect, Relations>(
            new DatabaseDriver(connection, this.state, state),
            this.schema,
            this.relations,
        );

        return state.execute(() => operation(transaction));
    }
}

/** Submitted operations and shutdown of one physical connection. */
export class ConnectionState {
    /** Operations that must finish before the client closes. */
    readonly #pending = new Set<Promise<void>>();
    /** The shared shutdown operation once closure starts. */
    #closing?: Promise<void>;

    /** Submit an operation while this connection accepts work. */
    run<Value>(operation: () => PromiseLike<Value>): Promise<Value> {
        if (this.#closing) {
            throw new DatabaseError("CONNECTION_CLOSED", "The connection is closing or closed.");
        }

        // retain settlement separately from the result returned to the caller
        const result = Promise.resolve().then(operation);
        const settled = result.then(
            () => {
                this.#pending.delete(settled);
            },
            () => {
                this.#pending.delete(settled);
            },
        );
        this.#pending.add(settled);

        return result;
    }

    /** Stop submissions, drain pending work, and close the client once. */
    close(operation: () => Promise<void>): Promise<void> {
        this.#closing ??= Promise.all(this.#pending).then(operation);

        return this.#closing;
    }
}
