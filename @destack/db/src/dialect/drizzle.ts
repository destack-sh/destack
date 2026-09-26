import type * as drizzle from "drizzle-orm";
import type { Query, SQL, SQLWrapper, Subquery } from "drizzle-orm";
import type { PgTable } from "drizzle-orm/pg-core";
import type { SQLiteTable } from "drizzle-orm/sqlite-core";
import type { PreparedQuery } from "../query/select.ts";
import type { Selection } from "../query/selection.ts";

/** A Drizzle database, as SQLite and PostgreSQL build queries on it alike. */
export interface DrizzleDatabase {
    /** Name a query for later selections. */
    $with(alias: string): { as(query: unknown): unknown };
    /** Start an insertion. */
    insert(table: unknown): { values(records: Record<string, unknown>[]): DrizzleInsert };
    /** Start an update. */
    update(table: unknown): { set(values: Record<string, unknown>): DrizzleMutation };
    /** Start a deletion. */
    delete(table: unknown): DrizzleMutation;
}

/** A Drizzle insertion before it becomes dynamic. */
export interface DrizzleInsert {
    /** Allow chaining conflict handling and returned fields. */
    $dynamic(): DrizzleMutation;
}

/** A Drizzle mutation, as SQLite and PostgreSQL build it alike. */
export interface DrizzleMutation extends PromiseLike<unknown> {
    /** Describe SQL and positional parameters. */
    toSQL(): Query;
    /** Prepare a reusable mutation. */
    prepare(): { execute(parameters?: Record<string, unknown>): Promise<unknown> };
    /** Restrict the mutated rows. */
    where(predicate: SQL | undefined): DrizzleMutation;
    /** Skip rows that conflict with a unique target. */
    onConflictDoNothing(config: {
        /** The conflicting columns. */
        target?: unknown[];
        /** The predicate of a partial unique target. */
        where?: SQL;
    }): DrizzleMutation;
    /** Update rows that conflict with a unique target. */
    onConflictDoUpdate(config: {
        /** The conflicting columns. */
        target: unknown[];
        /** The values written to conflicting rows. */
        set: Record<string, unknown>;
        /** The predicate of a partial unique target. */
        targetWhere?: SQL;
        /** The predicate restricting which conflicting rows update. */
        setWhere?: SQL;
    }): DrizzleMutation;
    /** Return fields of the mutated rows. */
    returning(fields: object): DrizzleMutation;
}

/** A Drizzle selection, as SQLite and PostgreSQL build it alike. */
export interface DrizzleSelect extends PromiseLike<unknown[]>, SQLWrapper {
    /** Describe SQL and positional parameters. */
    toSQL(): Query;
    /** Prepare a reusable selection. */
    prepare(): PreparedQuery<unknown[]>;
    /** Name this selection. */
    as(alias: string): Subquery;
    /** Read selected fields and their native decoders. */
    getSelectedFields(): Selection;
    /** Join matching rows. */
    innerJoin(table: SQLiteTable | PgTable | Subquery, on?: SQL): DrizzleSelect;
    /** Join matching or null rows. */
    leftJoin(table: SQLiteTable | PgTable | Subquery, on?: SQL): DrizzleSelect;
    /** Join all right rows. */
    rightJoin(table: SQLiteTable | PgTable | Subquery, on?: SQL): DrizzleSelect;
    /** Join all rows on both sides. */
    fullJoin(table: SQLiteTable | PgTable | Subquery, on?: SQL): DrizzleSelect;
    /** Join every combination of rows. */
    crossJoin(table: SQLiteTable | PgTable | Subquery): DrizzleSelect;
    /** Filter selected rows. */
    where(predicate: SQL): DrizzleSelect;
    /** Group selected rows. */
    groupBy(...expressions: SQLWrapper[]): DrizzleSelect;
    /** Filter grouped rows. */
    having(predicate: SQL): DrizzleSelect;
    /** Order selected rows. */
    orderBy(...expressions: SQLWrapper[]): DrizzleSelect;
    /** Limit selected rows. */
    limit(count: number): DrizzleSelect;
    /** Skip selected rows. */
    offset(count: number): DrizzleSelect;
}

/** A selection as Drizzle fields. */
export interface DrizzleSelection {
    /** A physical column, SQL expression, or nested group. */
    [property: string]: drizzle.Column | SQL | SQL.Aliased | DrizzleSelection;
}
