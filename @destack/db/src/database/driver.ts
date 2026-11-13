import type { TransactionState } from "./transaction.ts";
import type { ReadCache } from "../log/cache.ts";
import type { ConnectionState, WriteOperation } from "./connection.ts";
import { classifyError, DatabaseError } from "../error/error.ts";
import { CallErrorCode } from "@destack/error";
import type { Dialect } from "../dialect/dialect.ts";
import type { DriverStatement } from "../sql/index.ts";
import type { Session } from "./session.ts";
import { TelemetryScope } from "@destack/telemetry";
import { type Environment, SpanPath } from "@destack/environment";
import type {} from "@destack/package/import-meta";

/** The statement verbs the database runs, as OpenTelemetry's `db.operation.name` reports them. */
const OPERATIONS = [
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
];

/** The log operations a database measures. */
const LOG_OPERATIONS = ["read", "changed", "position"] as const;

/** The span path of a statement run outside any span. */
const NO_PATH = "(none)";

/** The offset basis of 32-bit FNV-1a. */
const FNV_OFFSET = 0x81_1c_9d_c5;

/** The prime of 32-bit FNV-1a. */
const FNV_PRIME = 0x01_00_01_93;

/** The milliseconds in a second, as OpenTelemetry records durations in seconds. */
const MILLISECONDS_PER_SECOND = 1000;

/** The transactions a database measures: optimistic ones over its log, and its own. */
const TRANSACTION_KINDS = ["log", "database"] as const;

/** The blob operations a database measures. */
const BLOB_OPERATIONS = ["put", "write", "fetch", "collect"] as const;

/** One statement text's totals under one span path, as `pg_stat_statements` keeps them. */
export interface StatementRecord {
    /** The span path that ran it, such as `label/create>destack.transaction[main:database]`. */
    readonly path: string;
    /** The statement's identity: an FNV-1a hash of its text, in hexadecimal. */
    readonly query: string;
    /** Its operation and tables, as OpenTelemetry's `db.query.summary` reports them. */
    readonly summary: string;
    /** The runs. */
    count: number;
    /** The host seconds the runs took. */
    seconds: number;
    /** The rows the runs returned or changed. */
    rows: number;
}

/** A database's telemetry: statements, transactions, commits, log reads, blobs and wakes. */
export class DatabaseTelemetry {
    /** The spans and measures of the database's package. */
    readonly #scope: TelemetryScope;
    /** The duration of each statement. */
    readonly #operations;
    /** The duration of each transaction run, its lost races and reruns among it. */
    readonly #transactions;
    /** The attempts of each transaction run, a lost race rerunning it. */
    readonly #attempts;
    /** The duration of each commit. */
    readonly #commits;
    /** The duration of each log operation. */
    readonly #logOperations;
    /** The duration of each blob operation. */
    readonly #blobOperations;
    /** The bytes of blobs read and written. */
    readonly #blobBytes;
    /** The wakes of the commit watch, one per commit it announces to its watchers. */
    readonly #wakes;
    /** The SQL names of the tables a statement may report as its collection. */
    readonly #collections: ReadonlySet<string>;
    /** The statement records by span path, then by statement text. */
    readonly #statements = new Map<string, Map<string, StatementRecord>>();
    /** The context the span paths are read from. */
    readonly #context: Environment["telemetry"]["context"];
    /** The host clock statements are timed on. */
    readonly #monotonic: Environment["telemetry"]["monotonic"];
    /** The database's name, as OpenTelemetry's `db.namespace` reports it. */
    readonly namespace: string;

    /** Declare the instruments on a telemetry, over a named database's tables. */
    constructor(
        telemetry: Environment["telemetry"],
        namespace: string,
        collections: readonly string[],
    ) {
        // store the namespace, the tables, and the context and clock statements are recorded on
        this.#collections = new Set(collections);
        this.namespace = namespace;
        this.#context = telemetry.context;
        this.#monotonic = telemetry.monotonic;

        // declare the statement, transaction and commit durations
        const scope = TelemetryScope.of(import.meta.destack.package, telemetry);
        this.#scope = scope;
        const failed = { "error.type": CallErrorCode.codes };
        this.#operations = scope.metric.histogram("db.client.operation.duration", {
            unit: "s",
            attributes: {
                "db.system.name": ["sqlite", "postgresql"],
                "db.namespace": [namespace],
                "db.operation.name": OPERATIONS,
                "db.collection.name": collections,
                ...failed,
            },
        });
        const transacted = {
            "db.namespace": [namespace],
            "destack.transaction.kind": TRANSACTION_KINDS,
        };
        this.#transactions = scope.metric.histogram("destack.transaction.duration", {
            unit: "s",
            attributes: { ...transacted, ...failed },
        });
        this.#commits = scope.metric.histogram("destack.transaction.commit.duration", {
            unit: "s",
            attributes: { "db.namespace": [namespace], ...failed },
        });

        // declare the log read, blob operation and watch wake measures
        this.#logOperations = scope.metric.histogram("destack.log.operation.duration", {
            unit: "s",
            attributes: {
                "db.namespace": [namespace],
                "destack.log.operation.name": LOG_OPERATIONS,
                ...failed,
            },
        });
        this.#blobOperations = scope.metric.histogram("destack.blob.operation.duration", {
            unit: "s",
            attributes: {
                "db.namespace": [namespace],
                "destack.blob.operation.name": BLOB_OPERATIONS,
                ...failed,
            },
        });
        this.#blobBytes = scope.metric.counter("destack.blob.io", {
            unit: "By",
            attributes: {
                "db.namespace": [namespace],
                "destack.blob.io.direction": ["read", "write"],
            },
        });
        this.#attempts = scope.metric.histogram("destack.transaction.attempts", {
            unit: "{attempt}",
            attributes: transacted,
        });
        this.#wakes = scope.metric.counter("destack.watch.wakes", {
            unit: "{wake}",
            attributes: { "db.namespace": [namespace] },
        });
    }

    /** Run a statement on a dialect, recording its duration by its operation and first table. */
    statement<Value>(
        dialect: Dialect,
        statement: DriverStatement,
        run: () => Promise<Value>,
    ): Promise<Value> {
        // report the operation and the first table, when the database declares them
        const operation = OPERATIONS.includes(statement.operation)
            ? statement.operation
            : undefined;
        const table = statement.tables[0];
        const collection = table !== undefined && this.#collections.has(table) ? table : undefined;
        const attributes = {
            "db.system.name": dialect,
            "db.namespace": this.namespace,
            ...(operation === undefined ? {} : { "db.operation.name": operation }),
            ...(collection === undefined ? {} : { "db.collection.name": collection }),
        };

        return this.#scope.measure(this.#operations, attributes, async () => {
            // time the run, recording it under its work and text whether it succeeds or fails
            const started = this.#monotonic.now();
            let rows = 0;
            try {
                const value = await run();
                rows = Array.isArray(value) ? value.length : typeof value === "number" ? value : 0;

                return value;
            } finally {
                const seconds = (this.#monotonic.now() - started) / MILLISECONDS_PER_SECOND;
                this.#record(statement, seconds, rows);
            }
        });
    }

    /** List the statement records, the costliest first. */
    statements(): readonly StatementRecord[] {
        const records = [...this.#statements.values()].flatMap((texts) => [...texts.values()]);

        return records.toSorted((left, right) => right.seconds - left.seconds);
    }

    /** Add a statement's run to its record under the span path running it. */
    #record(statement: DriverStatement, seconds: number, rows: number): void {
        // find the span path's records
        const path = SpanPath.of(this.#context.active()) ?? NO_PATH;
        const texts = this.#statements.get(path) ?? new Map<string, StatementRecord>();
        this.#statements.set(path, texts);

        // add the run to the text's record, made at its first run
        const { text } = statement;
        const record = texts.get(text) ?? {
            path,
            query: fnv1a(text),
            summary: [statement.operation, ...statement.tables].join(" "),
            count: 0,
            seconds: 0,
            rows: 0,
        };
        texts.set(text, record);
        record.count += 1;
        record.seconds += seconds;
        record.rows += rows;
    }

    /** Run a log or database transaction in its span, recording its attempts and duration. */
    transaction<Value>(
        kind: (typeof TRANSACTION_KINDS)[number],
        run: (attempted: () => void) => Promise<Value>,
    ): Promise<Value> {
        // count the attempts the run makes, lost races rerunning it
        let attempts = 0;
        const attempted = () => {
            attempts += 1;
        };

        // run it in its span, recording how long it took
        const attributes = { "db.namespace": this.namespace, "destack.transaction.kind": kind };

        return this.#scope.measure(this.#transactions, attributes, () =>
            this.#scope.span("destack.transaction", attributes, async (span) => {
                try {
                    return await run(attempted);
                } finally {
                    span.setAttribute("destack.transaction.attempts", attempts);
                    this.#attempts.record(attempts, attributes);
                }
            }),
        );
    }

    /** Run a commit, recording its duration. */
    commit<Value>(run: () => Promise<Value>): Promise<Value> {
        return this.#scope.measure(this.#commits, { "db.namespace": this.namespace }, run);
    }

    /** Run a log operation, recording its duration. */
    log<Value>(
        operation: (typeof LOG_OPERATIONS)[number],
        run: () => Promise<Value>,
    ): Promise<Value> {
        const attributes = {
            "db.namespace": this.namespace,
            "destack.log.operation.name": operation,
        };

        return this.#scope.measure(this.#logOperations, attributes, run);
    }

    /** Run a blob operation, recording its duration. */
    blob<Value>(
        operation: (typeof BLOB_OPERATIONS)[number],
        run: () => Promise<Value>,
    ): Promise<Value> {
        const attributes = {
            "db.namespace": this.namespace,
            "destack.blob.operation.name": operation,
        };

        return this.#scope.measure(this.#blobOperations, attributes, run);
    }

    /** Count the bytes of blobs read or written. */
    blobIo(direction: "read" | "write", bytes: number): void {
        this.#blobBytes.add(bytes, {
            "db.namespace": this.namespace,
            "destack.blob.io.direction": direction,
        });
    }

    /** Count a wake of the commit watch. */
    wake(): void {
        this.#wakes.add(1, { "db.namespace": this.namespace });
    }
}

/** A connection pool's telemetry: its waits for a connection and its connections by state. */
export class PoolTelemetry {
    /** The measures of the database's package. */
    readonly #scope: TelemetryScope;
    /** The duration of each wait for a connection. */
    readonly #waits;
    /** The connections idle and in use. */
    readonly #connections;
    /** The pool's name, as OpenTelemetry's `db.client.connection.pool.name` reports it. */
    readonly #name: string;

    /** Declare the instruments on a telemetry, for a named pool of some idle connections. */
    constructor(telemetry: Environment["telemetry"], name: string, connections: number) {
        // declare the wait and the connections by state
        const scope = TelemetryScope.of(import.meta.destack.package, telemetry);
        const pool = { "db.client.connection.pool.name": [name] };
        this.#scope = scope;
        this.#name = name;
        this.#waits = scope.metric.histogram("db.client.connection.wait_time", {
            unit: "s",
            attributes: { ...pool, "error.type": CallErrorCode.codes },
        });
        this.#connections = scope.metric.upDownCounter("db.client.connection.count", {
            unit: "{connection}",
            attributes: { ...pool, "db.client.connection.state": ["idle", "used"] },
        });

        // count the connections idle
        this.#count(connections, "idle");
    }

    /** Wait for a connection, recording how long the wait took. */
    wait<Value>(run: () => Promise<Value>): Promise<Value> {
        return this.#scope.measure(
            this.#waits,
            { "db.client.connection.pool.name": this.#name },
            run,
        );
    }

    /** Count a connection idle until now as used. */
    use(): void {
        this.#count(-1, "idle");
        this.#count(1, "used");
    }

    /** Count a connection used until now as idle. */
    release(): void {
        this.#count(-1, "used");
        this.#count(1, "idle");
    }

    /** Add to the connections in a state. */
    #count(connections: number, state: "idle" | "used"): void {
        this.#connections.add(connections, {
            "db.client.connection.pool.name": this.#name,
            "db.client.connection.state": state,
        });
    }
}

/** Hash a text with 32-bit FNV-1a, in hexadecimal. */
function fnv1a(text: string): string {
    let hash = FNV_OFFSET;
    for (let index = 0; index < text.length; index += 1) {
        hash = Math.imul(hash ^ text.charCodeAt(index), FNV_PRIME) >>> 0;
    }

    return hash.toString(16).padStart(8, "0");
}

/** The tables no script wrote, which a plain write passes. */
const UNSCRIPTED: ReadonlySet<string> = new Set();

/** Whether a write logs: through tables with change triggers, through none, or unknown. */
type Logging = "logged" | "unlogged" | "unknown";

/** A session and its statements' lifetime, counting and failures. */
export class DatabaseDriver {
    /** The session running statements. */
    readonly session: Session;
    /** The shared connection lifecycle. */
    readonly state: ConnectionState;
    /** The active transaction state, when present. */
    readonly transaction: TransactionState | undefined;
    /** The operations submitted through this driver outside a transaction. */
    operations = 0;

    /** Create the driver. */
    constructor(session: Session, state: ConnectionState, transaction?: TransactionState) {
        // store the session, the connection's state and the transaction
        this.session = session;
        this.state = state;
        this.transaction = transaction;
    }

    /** The SQL dialect. */
    get dialect(): Dialect {
        return this.session.dialect;
    }

    /** Submit work through the transaction or connection. */
    run<Value>(operation: () => PromiseLike<Value>, statement: DriverStatement): Promise<Value> {
        // count the statement, and the operation the connection submits outside a transaction
        this.state.statements += 1;
        if (this.transaction === undefined) {
            this.operations += 1;
        }

        // classify failures, and record the duration, its statements counted in flight by the session
        const reported = () =>
            this.state.telemetry.statement(this.dialect, statement, async () => {
                try {
                    return await operation();
                } catch (error) {
                    throw classifyError(error, this.state.declared);
                }
            });

        return this.transaction ? this.transaction.run(reported) : this.state.run(reported);
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

    /** Run work that commits by itself, then announce the tables it wrote and its last sequence. */
    async commit<Value>(
        operation: () => Promise<Value>,
        tables: ReadonlySet<string>,
        scripted: ReadonlySet<string>,
        statement: DriverStatement,
        sequence: () => number | undefined = () => undefined,
        changed: (value: Value) => number = () => 1,
    ): Promise<Value> {
        // record the tables in the enclosing transaction, marking a replica's write that derives rows first
        const transaction = this.transaction;
        if (transaction !== undefined) {
            const mark = transaction.replicaMark;
            if (mark !== undefined && this.#isDerived([...tables, ...scripted], statement)) {
                transaction.replicaMark = undefined;
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

        // announce a commit that changed rows of tables, with the last sequence it logged
        if (tables.size > 0 && changed(result) > 0) {
            this.state.commits.notify(tables, scripted, sequence());
        }

        return result;
    }

    /** Read every row of a bound statement as an array of values. */
    values(statement: DriverStatement): Promise<unknown[][]> {
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
        const read = () =>
            this.#query(statement, (session) => session.all(statement.text, statement.parameters));

        return this.#isCacheable(statement)
            ? this.#cached(statement, this.state.recordCache, read)
            : read();
    }

    /** Run a bound statement for its effect: a write of its tables, or a read such as a lock. */
    async execute(statement: DriverStatement): Promise<void> {
        // run a read alone, writing no table
        this.transaction?.assertActive();
        const run = (session: Session) =>
            failing(statement, session.run(statement.text, statement.parameters), this.state);
        if (statement.isRead) {
            await this.run(() => run(this.session), statement);

            return;
        }

        // run a write of its tables
        await this.write(run, statement, (count) => count);
    }

    /** Run a statement returning rows, as a read or as a write of the tables it references. */
    #query<Value extends readonly unknown[]>(
        statement: DriverStatement,
        query: (session: Session) => Promise<Value>,
    ): Promise<Value> {
        this.transaction?.assertActive();

        return statement.isRead
            ? this.run(() => failing(statement, query(this.session), this.state), statement)
            : this.write(
                  (session) => failing(statement, query(session), this.state),
                  statement,
                  (rows) => rows.length,
              );
    }

    /** Decide whether a read may be cached: a live read of versioned tables only. */
    #isCacheable(statement: DriverStatement): boolean {
        const { commits } = this.state;

        return (
            statement.isRead &&
            this.transaction === undefined &&
            statement.tables.length > 0 &&
            statement.tables.every((table) => commits.isVersioned(table))
        );
    }

    /** Read a statement once per version of its tables, every caller sharing its frozen rows. */
    async #cached<Row>(
        statement: DriverStatement,
        cache: ReadCache<Row[]>,
        read: () => Promise<Row[]>,
    ): Promise<Row[]> {
        // read from the database until the watch receives every commit, listening from the first read
        const { commits } = this.state;
        if (!commits.isComplete) {
            void commits.listen();

            return await read();
        }

        // key the read by its text and parameters, at its tables' versions before it runs
        const versions = commits.versions(statement.tables);
        const key = `${statement.text} ${JSON.stringify(statement.parameters, encodeKey)}`;

        return await cache.get(key, versions, async () => freeze(await read()));
    }

    /** Decide whether a write fires derivations, dependents only on deletes. */
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

    /** Run a write stamped in its transaction, refusing a logged write outside one. */
    async #identified<Value>(
        operation: (session: Session) => Promise<Value>,
        logging: Logging,
        logged: readonly string[],
    ): Promise<Value> {
        // stamp a write that may log inside a transaction, in statement order
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
        // run any other write as it is, the triggers refusing an unknown table's logged change
        else {
            return await operation(this.session);
        }
    }
}

/** Report a driver failure with its statement, leaving classified failures as they are. */
async function failing<Value>(
    statement: DriverStatement,
    pending: Promise<Value>,
    state: ConnectionState,
): Promise<Value> {
    try {
        return await pending;
    } catch (error) {
        // rethrow a classified failure, and report another with its statement
        const classified = classifyError(error, state.declared);
        if (DatabaseError.is(classified)) {
            throw classified;
        }

        throw DatabaseError.of("QUERY_BROKEN", `query failed: '${statement.text}'`, {
            cause: error,
        });
    }
}

/** Read a statement's write operation, a delete for any other, as it cascades furthest. */
function writeOperation(statement: DriverStatement): WriteOperation {
    const { operation } = statement;

    return operation === "INSERT" || operation === "UPDATE" ? operation : "DELETE";
}

/** Encode a cached read's parameters as a key, bigints apart from strings. */
function encodeKey(_key: string, value: unknown): unknown {
    return typeof value === "bigint" ? { bigint: String(value) } : value;
}

/** Freeze rows and each row, so callers share them and a write to one fails loudly. */
function freeze<Row>(rows: Row[]): Row[] {
    for (const row of rows) {
        Object.freeze(row);
    }

    Object.freeze(rows);

    return rows;
}
