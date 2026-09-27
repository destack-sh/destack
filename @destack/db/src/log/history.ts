import type { DatabaseConnection } from "../database/connection.ts";
import type { Dialect } from "../dialect/dialect.ts";
import { literal, quote } from "../dialect/quote.ts";
import { DatabaseError } from "../error/error.ts";
import { readState, type TableState } from "../migration/state.ts";
import { latestOf, selectHead, type LogPosition } from "./position.ts";
import { LOG } from "./schema.ts";

/** The statements a database as of a position runs: reads, never writes. */
const READ = /^\s*\(*\s*(select|with)\b/i;

/** The keywords of statements that write, found outside quoted names and literals. */
const WRITE = /\b(insert|update|delete|merge|create|drop|alter)\b/i;

/** The quoted names and string literals of a statement, which may hold any word. */
const QUOTED = /"(?:[^"]|"")*"|'(?:[^']|'')*'/g;

/** A statement's own common table expressions, which the past relations join at the front. */
const WITH = /^\s*with\s+(recursive\s+)?/i;

/**
 * The logged tables of a database as they were at a log position, as SQL every read statement runs over.
 *
 * Each logged table a statement names becomes a common table expression of its rows at the position: the current rows no later change touched, and the image each touched row had before its first later change.
 * It shows logged columns only, which the log records the images of.
 */
export class History {
    /** The position shown. */
    readonly position: LogPosition;
    /** The database's dialect. */
    readonly dialect: Dialect;
    /** The logged tables, by SQL name. */
    readonly #tables: ReadonlyMap<string, TableState>;

    /** Show the logged tables at a position. */
    constructor(position: LogPosition, dialect: Dialect, tables: readonly TableState[]) {
        this.position = position;
        this.dialect = dialect;
        this.#tables = new Map(
            tables.flatMap((state) => (state.log ? [[state.table.name, state]] : [])),
        );
    }

    /** Read the logged tables of a database, requiring the position in its current epoch and within its retained log. */
    static async read(database: DatabaseConnection, position: LogPosition): Promise<History> {
        // require the position's epoch and a sequence the log reached
        const [head] = await database.execute<{
            epoch: string;
            logged: number | string | null;
            horizon: number | string | null;
        }>(selectHead());
        if (head!.epoch !== position.epoch) {
            throw new DatabaseError(
                "STALE_EPOCH",
                `position of epoch ${position.epoch} is not in the log's epoch ${head!.epoch}`,
            );
        } else if (position.sequence > latestOf(head!.logged, head!.horizon)) {
            throw new DatabaseError(
                "INVALID_QUERY",
                `position ${position.sequence} is after the log's latest sequence`,
            );
        }

        // require the retained changes of every table whose changes compaction removes
        const tables = await readState(database);
        const horizon = head!.horizon === null ? 0 : Number(head!.horizon);
        const isCompacted = tables.some((state) => state.log?.tier === "window");
        if (isCompacted && position.sequence < horizon) {
            throw new DatabaseError(
                "CHANGES_COMPACTED",
                `changes after ${position.sequence} were compacted; read a later position`,
            );
        }

        return new History(position, database.dialect, tables);
    }

    /** Rewrite a read statement to read the logged tables it names as of the position, refusing writes. */
    rewrite(statement: string): string {
        // refuse writes, also those a read's common table expressions hold, which a past database cannot take
        if (!READ.test(statement) || WRITE.test(statement.replaceAll(QUOTED, ""))) {
            throw new DatabaseError("READ_ONLY", "a database as of a position only reads");
        }

        // name each logged table the statement reads by its past relation
        const named = [...this.#tables.keys()].filter((name) => statement.includes(quote(name)));
        if (named.length === 0) {
            return statement;
        }
        let rewritten = statement;
        for (const name of named) {
            rewritten = rewritten.replaceAll(quote(name), quote(this.#relation(name)));
        }

        // define the past relations before the statement's own common table expressions
        const relations = named.map((name) => this.#define(this.#tables.get(name)!)).join(", ");
        const own = WITH.exec(rewritten);

        return own === null
            ? `WITH ${relations} ${rewritten}`
            : `WITH ${own[1] ?? ""}${relations}, ${rewritten.slice(own[0].length)}`;
    }

    /** Name a table's relation at the position. */
    #relation(name: string): string {
        return `${name}__at_${this.position.sequence}`;
    }

    /** Define a table's relation at the position: its untouched current rows, and the images of the rows later changes touched. */
    #define(state: TableState): string {
        // select the logged columns of the current rows and of the images
        const description = state.log!;
        const table = quote(state.table.name);
        const types = new Map(state.table.columns.map((column) => [column.name, column.type]));
        const columns = description.columns.map(quote).join(", ");
        const images = description.columns
            .map(
                (name) =>
                    `${this.#image(name, types.get(name)!, description.exact)} AS ${quote(name)}`,
            )
            .join(", ");

        // find the first change of each touched row after the position
        const log = quote(LOG);
        const after = `${log}.sequence > ${this.position.sequence} AND ${log}."table" = ${literal(description.table)}`;
        const first = `SELECT min(sequence) FROM ${log} WHERE ${after} GROUP BY key`;
        const touched = `SELECT key FROM ${log} WHERE ${after}`;

        return `${quote(this.#relation(state.table.name))} AS (
            SELECT ${columns} FROM ${table}
                WHERE ${this.#key(table, description.key, description.exact)} NOT IN (${touched})
            UNION ALL
            SELECT ${images} FROM ${log}
                WHERE ${log}.sequence IN (${first}) AND ${log}.operation <> 'insert'
        )`;
    }

    /** Encode a current row's key as the log names rows, to compare it with the log's keys. */
    #key(table: string, key: readonly string[], exact: readonly string[]): string {
        const values = key.map((name) =>
            this.dialect === "sqlite" && exact.includes(name)
                ? `CAST(${table}.${quote(name)} AS TEXT)`
                : `${table}.${quote(name)}`,
        );

        return this.dialect === "sqlite"
            ? `json_array(${values.join(", ")})`
            : `jsonb_build_array(${values.join(", ")})`;
    }

    /** Read one column of the image before a change: the previous value an update replaced, else the recorded row's. */
    #image(name: string, type: string, exact: readonly string[]): string {
        // read the value from the previous values where they hold it, else from the row
        if (this.dialect === "sqlite") {
            const path = literal(`$."${name.replaceAll('"', '\\"')}"`);
            const value = `CASE WHEN json_type(previous, ${path}) IS NOT NULL THEN json_extract(previous, ${path}) ELSE json_extract("row", ${path}) END`;

            return exact.includes(name) ? `CAST(${value} AS ${type})` : value;
        }

        // cast PostgreSQL's JSON text, keeping JSON columns as JSON
        const source = `(CASE WHEN previous ? ${literal(name)} THEN previous ELSE "row" END)`;
        const isJson = /^jsonb?$/i.test(type);

        return isJson
            ? `(${source} -> ${literal(name)})::${type}`
            : `(${source} ->> ${literal(name)})::${type}`;
    }
}
