import type { Dialect } from "../dialect/dialect.ts";
import { quote } from "../dialect/quote.ts";
import type { Session } from "../database/session.ts";
import type { TableState } from "../migration/state.ts";
import { LOG, LOG_TRANSACTION } from "../log/schema.ts";
import { dialectSQL, inline, sql, type SQL, type SQLWrapper } from "../sql/index.ts";
import { boundedName, relation } from "../table/namespace.ts";
import { present, schema } from "@destack/schema";
import { TABLE, type Table } from "../table/table.ts";
import type { Row } from "../table/row.ts";
import { SearchText } from "./text.ts";

/** The suffix naming a table's full-text table in SQLite and its index in PostgreSQL. */
const SUFFIX = "__search";

/** The PostgreSQL collation folding case and classing letters and digits by Unicode. */
const COLLATION = "pg_c_utf8";

/** What PostgreSQL splits text into words at: any run of characters that is no letter or digit. */
const SEPARATOR = "[^[:alnum:]]+";

/** The SQLite FTS5 tokenizer: Unicode words, case-folded, their diacritics kept. */
const TOKENIZER = "unicode61 remove_diacritics 0";

/** The most keys one SQLite statement deletes or inserts, within SQLite's parameter limit. */
const KEEP_BATCH = 500;

/** A searched column's logged value: text, or nothing. */
const SearchedValue = schema.string().nullable();

/** A row's logged image: its values by SQL column name. */
const LoggedRow = schema.record(schema.string(), schema.json());

/** A table's full-text search: its searched text columns and the index of their words. */
export class FullTextIndex {
    /** The searched text properties, in index order. */
    readonly properties: readonly string[];
    /** The SQL names of the searched columns, in index order. */
    readonly columns: readonly string[];
    /** The SQL name of the table keeping the index, an alias's source. */
    readonly table: string;

    /** Search some text columns of a table. */
    constructor(properties: readonly string[], columns: readonly string[], table: string) {
        this.properties = properties;
        this.columns = columns;
        this.table = table;
    }

    /** Name a table's full-text index: an FTS5 table in SQLite, a GIN index in PostgreSQL. */
    static index(table: string): string {
        return boundedName(`${table}${SUFFIX}`);
    }

    /** Write the statements creating a searched table's full-text index and filling it. */
    static create(state: TableState, dialect: Dialect): string[] {
        // name the table, its index and its searched columns
        const columns = state.fullText ?? [];
        const table = quote(state.table.name);
        const index = quote(FullTextIndex.index(state.table.name));

        // index the PostgreSQL document of the columns
        if (dialect === "postgresql") {
            const identifiers = columns.map((column) => sql.identifier(column));
            const document = inline(postgresDocument(identifiers), dialect);

            return [`CREATE INDEX ${index} ON ${table} USING gin ((${document}))`];
        }

        // keep the columns in an FTS5 table keyed as the log keys rows, filled from the table
        const key = keyOf(state);
        const listed = columns.map((column) => quote(column)).join(", ");

        return [
            `CREATE VIRTUAL TABLE ${index} USING fts5("key" UNINDEXED, ${listed}, tokenize = '${TOKENIZER}')`,
            `INSERT INTO ${index} ("key", ${listed}) SELECT ${key}, ${listed} FROM ${table}`,
        ];
    }

    /** Write the statements dropping a table's full-text index. */
    static drop(state: TableState, dialect: Dialect): string[] {
        const index = quote(FullTextIndex.index(state.table.name));

        return [
            dialect === "postgresql"
                ? `DROP INDEX IF EXISTS ${index}`
                : `DROP TABLE IF EXISTS ${index}`,
        ];
    }

    /** Report whether a row's searched text contains each word of a search as a word's prefix. */
    matches(row: Row, words: readonly string[]): boolean {
        return SearchText.matches(
            this.properties.map((property) => row[property]),
            words,
        );
    }

    /** Write a search of the table's rows over bound columns, each word a word's prefix. */
    match(
        words: readonly string[],
        columns: readonly SQLWrapper[],
        key: readonly SQLWrapper[],
    ): SQL {
        const index = sql.relation(FullTextIndex.index(this.table));

        return dialectSQL({
            postgresql: sql`${postgresDocument(columns)} @@ ${sql.param(SearchText.tsquery(words))}::tsquery`,
            sqlite: sql`json_array(${sql.join(key, sql`, `)}) IN (SELECT "key" FROM ${index} WHERE ${index} MATCH ${sql.param(SearchText.fts5(words))})`,
        });
    }

    /** Keep the full-text rows of a SQLite transaction's changed searched rows, from its log. */
    static async keep(
        session: Session,
        searched: ReadonlyMap<string, Table>,
        namespace?: string,
    ): Promise<void> {
        // read the transaction's changes of searched tables, last change of each row winning
        if (searched.size === 0) {
            return;
        }
        const log = quote(relation(LOG, namespace));
        const marker = quote(relation(LOG_TRANSACTION, namespace));
        const tables = [...searched.keys()];
        const changes = await session.all(
            `SELECT "table", "key", operation, "row" FROM ${log}
                WHERE "transaction" = (SELECT id FROM ${marker} WHERE slot = 1)
                AND "table" IN (${tables.map(() => "?").join(", ")})
                ORDER BY sequence`,
            tables,
        );
        const latest = new Map<string, Map<string, { operation: string; row: string }>>();
        for (const change of changes) {
            const table = String(change["table"]);
            const rows = latest.get(table) ?? new Map<string, { operation: string; row: string }>();
            rows.set(String(change["key"]), {
                operation: String(change["operation"]),
                row: String(change["row"]),
            });
            latest.set(table, rows);
        }

        // replace each changed row's full-text row, leaving out deleted rows
        for (const [table, rows] of latest) {
            const searchedTable = present(searched.get(table), `searched table ${table}`);
            await present(searchedTable[TABLE].fullText, `the search of ${table}`).replace(
                session,
                rows,
                namespace,
            );
        }
    }

    /** Replace the full-text rows of the table's changed rows, leaving out deleted ones. */
    async replace(
        session: Session,
        rows: ReadonlyMap<string, { readonly operation: string; readonly row: string }>,
        namespace: string | undefined,
    ): Promise<void> {
        // name the full-text table, its columns and the changed keys
        const index = quote(relation(FullTextIndex.index(this.table), namespace));
        const columns = this.columns;
        const listed = columns.map((column) => quote(column)).join(", ");
        const keys = [...rows.keys()];

        // delete every changed row's full-text row
        for (let start = 0; start < keys.length; start += KEEP_BATCH) {
            const batch = keys.slice(start, start + KEEP_BATCH);
            await session.run(
                `DELETE FROM ${index} WHERE "key" IN (${batch.map(() => "?").join(", ")})`,
                batch,
            );
        }

        // insert the text of each row still there, a batch per statement
        const kept = [...rows]
            .filter(([, change]) => change.operation !== "delete")
            .map(([key, change]) => {
                const image = LoggedRow.parse(JSON.parse(change.row));

                return [key, ...columns.map((column) => SearchedValue.parse(image[column]))];
            });
        const placeholders = `(${["?", ...columns.map(() => "?")].join(", ")})`;
        for (let start = 0; start < kept.length; start += KEEP_BATCH) {
            const batch = kept.slice(start, start + KEEP_BATCH);
            await session.run(
                `INSERT INTO ${index} ("key", ${listed}) VALUES ${batch.map(() => placeholders).join(", ")}`,
                batch.flat(),
            );
        }
    }
}

/** Write the PostgreSQL document of some columns: their case-folded words as lexemes, unparsed. */
function postgresDocument(columns: readonly SQLWrapper[]): SQL {
    const text = sql.join(
        columns.map((column) => sql`coalesce(${column}, '')`),
        sql` || ' ' || `,
    );

    return sql`array_to_tsvector(array_remove(regexp_split_to_array(lower((${text}) COLLATE ${sql.raw(COLLATION)}), ${sql.raw(`'${SEPARATOR}'`)}), ''))`;
}

/** Write a table's key as SQLite's log records it: a JSON array of its key columns. */
function keyOf(state: TableState): string {
    const key = state.table.constraints.flatMap((constraint) =>
        constraint.kind === "primaryKey" ? constraint.columns : [],
    );

    return `json_array(${key.map((column) => quote(column)).join(", ")})`;
}
