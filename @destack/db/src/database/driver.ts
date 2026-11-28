import type { TransactionState } from "./transaction.ts";
import type { ReadCache } from "../log/cache.ts";
import type { ConnectionState, Logging, WriteOperation } from "./connection.ts";
import { classifyError, DatabaseError } from "../error/error.ts";
import type { Dialect } from "../dialect/dialect.ts";
import type { DriverStatement } from "../sql/index.ts";
import type { Session } from "./session.ts";
import { SpanPath } from "@destack/environment";
import type { Context } from "@destack/telemetry";
import { LOG_TABLES } from "../log/schema.ts";
import type {} from "@destack/package/import-meta";

/** The operations that change rows. */
const WRITE_OPERATIONS: ReadonlySet<string> = new Set(["INSERT", "UPDATE", "DELETE"]);

/** The span total counting the statements that change rows. */
const WRITES = "db.client.operation.write.count";

/** The statement verbs, as OpenTelemetry's `db.operation.name`. */
const OPERATIONS: ReadonlySet<string> = new Set([
    "SELECT",
    "INSERT",
    "UPDATE",
    "DELETE",
    "WITH",
    "PRAGMA",
    "CREATE",
    "DROP",
    "SCRIPT",
    "TRANSACTION",
]);

/** The tables a plain write passes. */
const UNSCRIPTED: ReadonlySet<string> = new Set();

/** A session running measured statements. */
export class DatabaseDriver {
    /** The session running statements. */
    readonly session: Session;
    /** The shared connection lifecycle. */
    readonly state: ConnectionState;
    /** The active transaction's state. */
    readonly transaction: TransactionState | undefined;
    /** The SQL names of the tables the driver's reads touch, absent when unrecorded. */
    readonly reads: Set<string> | undefined;

    /** Create the driver. */
    constructor(
        session: Session,
        state: ConnectionState,
        transaction?: TransactionState,
        reads?: Set<string>,
    ) {
        // keep the session, the connection's state, the transaction and the recorded reads
        this.session = session;
        this.state = state;
        this.transaction = transaction;
        this.reads = reads;
    }

    /** Who writes in the driver's transaction: a replica copying rows, or the database's home. */
    get writer(): "home" | "replica" {
        return this.transaction?.replicaMark === undefined ? "home" : "replica";
    }

    /** The SQL dialect. */
    get dialect(): Dialect {
        return this.session.dialect;
    }

    /** Submit work through the transaction or connection, run in its submitter's context. */
    run<Value>(operation: () => PromiseLike<Value>, statement: DriverStatement): Promise<Value> {
        // classify failures and measure the statement, bound to the submitter's context as its queue runs it from another flow
        const { context } = this.state.environment.telemetry;
        const reported = context.bind(this.#context(), () =>
            this.#measure(statement, async () => {
                try {
                    return await operation();
                } catch (error) {
                    throw classifyError(error, this.state.declared);
                }
            }),
        );

        return this.transaction ? this.transaction.run(reported) : this.state.run(reported);
    }

    /** Read the context a statement measures in, its transaction's unless nested within it. */
    #context(): Context {
        const active = this.state.environment.telemetry.context.active();
        const opened = this.transaction?.context;

        return opened === undefined || SpanPath.isWithin(active, opened) ? active : opened;
    }

    /** Run a statement measured on its owner's span, a transaction outside any on its own. */
    #measure<Value>(statement: DriverStatement, run: () => Promise<Value>): Promise<Value> {
        // name the operation and its first table
        const { state } = this;
        const operation = OPERATIONS.has(statement.operation) ? statement.operation : undefined;
        const table = statement.tables[0];
        const isDeclared =
            table !== undefined && (state.declared.has(table) || LOG_TABLES.includes(table));
        const attributes = {
            "db.system.name": this.dialect,
            "db.namespace": state.name,
            ...(operation === undefined ? {} : { "db.operation.name": operation }),
            ...(isDeclared ? { "db.collection.name": table } : {}),
        };

        // measure it, failed or not
        const { context } = state.environment.telemetry;
        let rows = 0;

        // measure the run, adding it to the totals of the spans running as it starts
        const measured = () => {
            const entered = context.active();

            return state.scope.measure(
                "db.client.operation.duration",
                attributes,
                async () => {
                    // run it, counting its rows, none for a transaction
                    const value = await run();
                    const isTransaction = operation === "TRANSACTION";
                    rows = isTransaction
                        ? 0
                        : Array.isArray(value)
                          ? value.length
                          : typeof value === "number"
                            ? value
                            : 0;

                    return value;
                },
                (seconds) => {
                    // add the run to the running spans' totals
                    SpanPath.add(entered, {
                        "db.client.operation.count": 1,
                        [WRITES]: WRITE_OPERATIONS.has(statement.operation) ? 1 : 0,
                        "db.client.operation.duration": seconds,
                        "db.response.returned_rows": rows,
                    });
                    state.rows.add(rows, attributes);
                },
            );
        };

        // run a transaction outside any span in a client span of its own, and each statement inside one
        const isRunning = SpanPath.of(context.active()) !== undefined;
        const isSpanned = operation === "TRANSACTION" ? !isRunning : isRunning;

        return isSpanned
            ? state.scope.span(operation ?? "QUERY", attributes, measured, {
                  kind: "client",
                  duration: false,
              })
            : measured();
    }

    /** Submit a statement writing its tables. */
    write<Value>(
        operation: (session: Session) => Promise<Value>,
        statement: DriverStatement,
        changed: (value: Value) => number,
    ): Promise<Value> {
        // cascade the written tables, every table for unknown ones
        const { tables } = statement;
        const written =
            tables.length === 0
                ? new Set(this.state.declared.keys())
                : this.state.cascade(tables, writeOperation(statement));

        // stamp a write that may log, of tables with triggers or of unknown tables
        const logged = this.state.logged(written);
        let logging: Logging = logged.length > 0 ? "logged" : "unlogged";

        // read a write of unknown tables as unknown
        if (tables.length === 0) {
            logging = "unknown";
        }

        return this.commit(
            () => this.#identified(operation, logging, logged),
            written,
            UNSCRIPTED,
            statement,
            () => undefined,
            changed,
        );
    }

    /** Run self-committing work, then announce its tables and last sequence. */
    async commit<Value>(
        operation: () => Promise<Value>,
        tables: ReadonlySet<string>,
        scripted: ReadonlySet<string>,
        statement: DriverStatement,
        sequence: () => number | undefined = () => undefined,
        changed: (value: Value) => number = () => 1,
    ): Promise<Value> {
        // record the tables in the enclosing transaction
        const transaction = this.transaction;
        if (transaction !== undefined) {
            const mark = transaction.replicaMark;
            if (mark !== undefined && this.#isDerived([...tables, ...scripted], statement)) {
                await mark();
            }
            const result = await this.run(operation, statement);
            for (const table of changed(result) > 0 ? tables : []) {
                transaction.written.add(table);
            }
            for (const table of scripted) {
                transaction.scripted.add(table);
            }

            return result;
        }

        // commit
        const result = await this.run(operation, statement);

        // announce a commit that changed rows
        if (tables.size > 0 && changed(result) > 0) {
            this.state.commits.notify(tables, scripted, sequence());
        }

        return result;
    }

    /** Read every row of a bound statement as an array of values. */
    values(statement: DriverStatement): Promise<unknown[][]> {
        this.#record(statement);
        const read = () =>
            this.#query(statement, (session) =>
                session.values(statement.text, statement.parameters),
            );

        return this.#isCacheable(statement)
            ? this.#cached(statement, this.state.valueCache, read)
            : read();
    }

    /** Read every row of a bound statement by column name. */
    all(statement: DriverStatement): Promise<Record<string, unknown>[]> {
        this.#record(statement);
        const read = () =>
            this.#query(statement, (session) => session.all(statement.text, statement.parameters));

        return this.#isCacheable(statement)
            ? this.#cached(statement, this.state.recordCache, read)
            : read();
    }

    /** Record the tables a read touches, every declared table for unknown ones. */
    #record(statement: DriverStatement): void {
        // record nothing on an unrecorded driver
        const { reads } = this;
        if (reads === undefined) {
            return;
        }

        // add each table the statement names, or every declared one
        const tables =
            statement.tables.length === 0 ? this.state.declared.keys() : statement.tables;
        for (const table of tables) {
            reads.add(table);
        }
    }

    /** Run a bound statement for its effect. */
    async execute(statement: DriverStatement): Promise<void> {
        // run a read
        this.transaction?.assertActive();
        const run = (session: Session) =>
            failing(statement, session.run(statement.text, statement.parameters), this.state);
        if (statement.isRead) {
            this.#record(statement);
            await this.run(() => run(this.session), statement);

            return;
        }

        // run a write
        await this.write(run, statement, (count) => count);
    }

    /** Run a statement returning rows. */
    #query<Value extends readonly unknown[]>(
        statement: DriverStatement,
        query: (session: Session) => Promise<Value>,
    ): Promise<Value> {
        this.transaction?.assertActive();

        // read, or announce every write
        return statement.isRead
            ? this.run(() => failing(statement, query(this.session), this.state), statement)
            : this.write(
                  (session) => failing(statement, query(session), this.state),
                  statement,
                  () => 1,
              );
    }

    /** Decide whether a read may be cached. */
    #isCacheable(statement: DriverStatement): boolean {
        const { commits } = this.state;

        return (
            statement.isRead &&
            this.transaction === undefined &&
            statement.tables.length > 0 &&
            statement.tables.every((table) => commits.isVersioned(table))
        );
    }

    /** Read a statement once per version of its tables, sharing frozen rows. */
    async #cached<Row>(
        statement: DriverStatement,
        cache: ReadCache<Row[]>,
        read: () => Promise<Row[]>,
    ): Promise<Row[]> {
        // read directly until the watch sees every commit
        const { commits } = this.state;
        if (!commits.isComplete) {
            void commits.listen();

            return await read();
        }

        // key the read by its text, parameters and table versions
        const versions = commits.versions(statement.tables);
        const key = `${this.state.textNumber(statement.text)} ${JSON.stringify(statement.parameters, encodeKey)}`;

        return await cache.get(key, versions, async () => freeze(await read()));
    }

    /** Decide whether a write fires derivations. */
    #isDerived(tables: readonly string[], statement: DriverStatement): boolean {
        const { aggregated, dependent, declared, unlogged } = this.state;
        const isDeleting = writeOperation(statement) === "DELETE";

        return tables.some(
            (table) =>
                aggregated.has(table) ||
                (isDeleting && dependent.has(table)) ||
                (!declared.has(table) && !unlogged.has(table)),
        );
    }

    /** Run a stamped write, refusing a logged write outside a transaction. */
    async #identified<Value>(
        operation: (session: Session) => Promise<Value>,
        logging: Logging,
        logged: readonly string[],
    ): Promise<Value> {
        // stamp a logging write in its transaction
        if (this.transaction !== undefined && logging !== "unlogged") {
            await this.transaction.identity.stamp();

            return await operation(this.session);
        }
        // refuse a known logged write outside a transaction
        else if (this.transaction === undefined && logging === "logged") {
            throw DatabaseError.of(
                "TRANSACTION_BROKEN",
                `a logged write of '${logged.join(", ")}' runs outside a transaction: run it in a transaction`,
            );
        }
        // run any other write as it is
        else {
            return await operation(this.session);
        }
    }
}

/** Report a driver failure with its statement. */
async function failing<Value>(
    statement: DriverStatement,
    pending: Promise<Value>,
    state: ConnectionState,
): Promise<Value> {
    try {
        return await pending;
    } catch (error) {
        // rethrow a classified failure, else report it with the statement
        const classified = classifyError(error, state.declared);
        if (DatabaseError.is(classified)) {
            throw classified;
        }

        throw DatabaseError.of("QUERY_BROKEN", `query failed: '${statement.text}'`, {
            cause: error,
        });
    }
}

/** Read a statement's write operation, a delete for any other. */
function writeOperation(statement: DriverStatement): WriteOperation {
    const { operation } = statement;

    return operation === "INSERT" || operation === "UPDATE" ? operation : "DELETE";
}

/** Encode a cached read's parameters as a key. */
function encodeKey(_key: string, value: unknown): unknown {
    return typeof value === "bigint" ? { bigint: String(value) } : value;
}

/** Freeze rows and each row. */
function freeze<Row>(rows: Row[]): Row[] {
    for (const row of rows) {
        Object.freeze(row);
    }

    Object.freeze(rows);

    return rows;
}
