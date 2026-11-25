import type { Dialect } from "../dialect/dialect.ts";
import { literal, quote } from "../dialect/quote.ts";
import type { DatabaseConnection } from "../database/connection.ts";
import type { Session } from "../database/session.ts";
import type { TableState } from "../migration/state.ts";
import { LOG, LOG_TRANSACTION } from "../log/schema.ts";
import { dialectSQL, inline, sql, type SQL, type SQLWrapper } from "../sql/index.ts";
import { boundedName, relation } from "../table/namespace.ts";
import { present, schema } from "@destack/schema";
import { TABLE, type Table } from "../table/table.ts";
import type { Row } from "../table/row.ts";
import { DatabaseError } from "../error/error.ts";
import { type Search, SEARCH_LATER, SEARCH_SCORE, startOf, wholeOf } from "./search.ts";
import { Tokenizer } from "./tokenizer.ts";

/** The suffix of a table's full-text table in SQLite and its index in PostgreSQL. */
const SUFFIX = "__search";

/** The suffix of the table that lists the words of a SQLite index. */
const TERMS_SUFFIX = "__search_terms";

/** The PostgreSQL collation folding case and classing letters and digits by Unicode. */
const COLLATION = "pg_c_utf8";

/** What PostgreSQL splits text into words at: any run of characters but letters and digits. */
const SEPARATOR = `[^[:alnum:]${Tokenizer.digits}]+`;

/** The version of the tokenizer's rules an index has, which a change of them raises. */
export const FULL_TEXT_VERSION = 2;

/** The SQLite FTS5 tokenizer, which takes written tokens apart at spaces. */
const TOKENIZER = "ascii";

/** The most rows one FTS5 match names, a list it parses in under a millisecond. */
const DELETE_BATCH = 500;

/** What the token of a row's key starts with, a control picture no word has. */
const ROW = "\u2404";

/** The ASCII characters SQLite's tokenizer splits at. */
const SEPARATORS = /[^a-zA-Z0-9\u0080-\u{10ffff}]/gu;

/** The token after a SQLite index's words, a control picture no word has. */
const END = "\u2403";

/** The token after each word of a script without spaces in a SQLite index. */
const WORD_END = "\u2401";

/** The rows a new index reads at once, about a megabyte of a kilobyte each. */
const FILL_BATCH = 1000;

/** A searched column's logged value: text, or nothing. */
const SearchedValue = schema.string().nullable();

/** A row's logged image: its values by SQL column name. */
const LoggedRow = schema.jsonObject();

/** A word an index has. */
export const Term = schema.object({
    /** The word. */
    term: schema.string(),
    /** How many rows have it. */
    rows: schema.union([schema.number(), schema.bigint(), schema.string()]).transform(Number),
});
/** A word an index has. */
export type Term = schema.Infer<typeof Term>;

/** A table's full-text search: its searched text columns and the index of their words. */
export class FullTextIndex {
    /** The searched text properties, in index order. */
    readonly properties: readonly string[];
    /** The SQL names of the searched columns, in index order. */
    readonly columns: readonly string[];
    /** The SQL name of the table that stores the index, an alias's source. */
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

    /** Name the table that lists the words of a table's SQLite index, which stores nothing. */
    static termTable(table: string): string {
        return boundedName(`${table}${TERMS_SUFFIX}`);
    }

    /** Write the statements creating a searched table's full-text index, SQLite's filled after. */
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

        // keep the columns' tokens in an FTS5 table, each row found by the token of its key
        const texts = columns.map((_, position) => `"${textColumn(position)}"`).join(", ");
        const terms = quote(FullTextIndex.termTable(state.table.name));

        return [
            `CREATE VIRTUAL TABLE ${index} USING fts5("key" UNINDEXED, "row", ${texts}, tokenize = '${TOKENIZER}')`,
            `CREATE VIRTUAL TABLE ${terms} USING fts5vocab(${index}, 'row')`,
        ];
    }

    /** Write the tokens of every row of a searched table into its new SQLite index. */
    static async fill(session: Session, state: TableState): Promise<void> {
        // read the rows in batches, the tokens being this program's
        const columns = state.fullText ?? [];
        const listed = columns.map((column) => quote(column)).join(", ");
        const table = quote(state.table.name);
        let last: unknown = -1;
        for (;;) {
            const rows = await session.all(
                `SELECT rowid AS "row", ${keyOf(state)} AS "key", ${listed} FROM ${table}
                    WHERE rowid > ? ORDER BY rowid LIMIT ${FILL_BATCH}`,
                [Number(last)],
            );
            await insertTokens(
                session,
                state.table.name,
                columns,
                rows.map((row) => [String(row["key"]), row]),
                undefined,
            );
            if (rows.length < FILL_BATCH) {
                return;
            }
            last = rows.at(-1)?.["row"];
        }
    }

    /** Write the statements dropping a table's full-text index. */
    static drop(state: TableState, dialect: Dialect): string[] {
        const index = quote(FullTextIndex.index(state.table.name));

        return dialect === "postgresql"
            ? [`DROP INDEX IF EXISTS ${index}`]
            : [
                  `DROP TABLE IF EXISTS ${quote(FullTextIndex.termTable(state.table.name))}`,
                  `DROP TABLE IF EXISTS ${index}`,
              ];
    }

    /** Report whether a row's searched text contains each word of a search as a word's prefix. */
    matches(row: Row, search: Search): boolean {
        return search.isIn(
            this.properties.flatMap((property) => {
                const value = row[property];

                return typeof value === "string" ? [value] : [];
            }),
        );
    }

    /** Write a search of the table's rows over bound columns, each word a word's prefix. */
    match(search: Search, columns: readonly SQLWrapper[], key: readonly SQLWrapper[]): SQL {
        const index = sql.relation(FullTextIndex.index(this.table));

        return dialectSQL({
            postgresql: sql`${postgresDocument(columns)} @@ ${sql.param(tsqueryOf(search))}::tsquery`,
            sqlite: isFound(key, index, startsOf(search)),
        });
    }

    /** Read the words of the index that start with the same letter as a word. */
    async terms(database: DatabaseConnection, word: string): Promise<Term[]> {
        // list them from SQLite's index, and from the documents of PostgreSQL's
        const [first = ""] = word;
        const next = String.fromCodePoint((first.codePointAt(0) ?? 0) + 1);
        const documents = postgresDocument(this.columns.map((column) => sql.identifier(column)));
        const listed = dialectSQL({
            sqlite: sql`SELECT term, doc AS "rows" FROM ${sql.relation(FullTextIndex.termTable(this.table))}
                WHERE term >= ${sql.param(first)} AND term < ${sql.param(next)}`,
            postgresql: sql`SELECT term, count(*) AS "rows" FROM (
                    SELECT unnest(tsvector_to_array(${documents})) AS term FROM ${sql.relation(this.table)}
                ) AS listed WHERE starts_with(term, ${sql.param(first)}::text) GROUP BY term`,
        });

        return await database.execute(listed, Term);
    }

    /** Write how relevant searched columns are to a search, as `Search.relevance` scores them. */
    relevance(
        search: Search,
        scored: readonly { readonly property: string; readonly column: SQLWrapper }[],
        key: readonly SQLWrapper[],
    ): SQL {
        // test each column as every engine can
        const index = sql.relation(FullTextIndex.index(this.table));
        const tests = scored.map(({ property }, position) => {
            // refuse a column the index leaves out
            const indexed = this.properties.indexOf(property);
            if (indexed === -1) {
                throw DatabaseError.of(
                    "QUERY_INVALID",
                    `relevance is invalid: '${this.table}.${property}' has no full-text index`,
                );
            }

            return {
                later: position === 0 ? 0 : SEARCH_LATER,
                sqlite: new SqliteRelevance(search, textColumn(indexed), index, key),
                postgresql: new PostgresRelevance(search, sql.identifier(`text${position}`)),
            };
        });
        const cases = (dialect: Dialect) =>
            sql`CASE ${sql.join(
                (["whole", "opening", "words", "starts"] as const).flatMap((kind) =>
                    tests.map(
                        (test) =>
                            sql`WHEN ${test[dialect][kind]()} THEN ${sql.raw(String(SEARCH_SCORE[kind] - test.later))}`,
                    ),
                ),
                sql` `,
            )} ELSE ${sql.raw(String(SEARCH_SCORE.across))} END`;

        // fold each PostgreSQL column once for its tests
        const folded = scored.map(
            ({ column }, position) =>
                sql`${postgresLine(sql`coalesce(${column}, '')`)} AS ${sql.identifier(`text${position}`)}`,
        );

        return dialectSQL({
            sqlite: cases("sqlite"),
            postgresql: sql`(SELECT ${cases("postgresql")} FROM (SELECT ${sql.join(folded, sql`, `)}) AS folded)`,
        });
    }

    /** Update the full-text rows of a SQLite transaction's changed searched rows, from its log. */
    static async update(
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
        const keys = [...rows.keys()];

        // delete every changed row's full-text row, found by its key's token
        const size = Math.min(DELETE_BATCH, session.limits.variables - 1);
        for (let start = 0; start < keys.length; start += size) {
            const batch = keys.slice(start, start + size);
            const tokens = batch.map((key) => `"${rowToken(key)}"`).join(" OR ");
            await session.run(
                `DELETE FROM ${index} WHERE ${index} MATCH ? AND "key" IN (${batch.map(() => "?").join(", ")})`,
                [`"row" : (${tokens})`, ...batch],
            );
        }

        // insert the tokens of each row still there
        const kept = [...rows]
            .filter(([, change]) => change.operation !== "delete")
            .map(([key, change]) => [key, LoggedRow.parse(JSON.parse(change.row))] as const);
        await insertTokens(session, this.table, columns, kept, namespace);
    }
}

/** The tests of how relevant a column is to a search in SQLite, through its index. */
class SqliteRelevance {
    /** The search. */
    readonly #search: Search;
    /** The column's name in the index. */
    readonly #name: string;
    /** The index. */
    readonly #index: SQL;
    /** The row's key in the index. */
    readonly #key: readonly SQLWrapper[];

    /** Test a column by its index. */
    constructor(search: Search, name: string, index: SQL, key: readonly SQLWrapper[]) {
        // test the column by its name in the index, and the row by its key
        this.#search = search;
        this.#name = name;
        this.#index = index;
        this.#key = key;
    }

    /** Test whether the search is all of it. */
    whole(): SQL {
        return this.#found(`^ ${phraseOf([...marked(this.#search.words), END])}`);
    }

    /** Test whether it opens with the search. */
    opening(): SQL {
        // leave the last word open
        return this.#found(`^ ${phraseOf(marked(this.#search.words, "open"))} *`);
    }

    /** Test whether it has each token. */
    words(): SQL {
        return this.#has((token) => Tokenizer.isUnspaced(token));
    }

    /** Test whether it has the start of each token. */
    starts(): SQL {
        return this.#has(() => true);
    }

    /** Test whether it has every token, some as starts. */
    #has(isStart: (token: string) => boolean): SQL {
        return this.#found(
            this.#search.tokens
                .map((token) => `"${token}"${isStart(token) ? "*" : ""}`)
                .join(" AND "),
        );
    }

    /** Test whether the index has the row among those its column matches. */
    #found(match: string): SQL {
        return isFound(this.#key, this.#index, `${this.#name} : (${match})`);
    }
}

/** The tests of how relevant a column is to a search in PostgreSQL, over its folded words. */
class PostgresRelevance {
    /** The search. */
    readonly #search: Search;
    /** The column's folded words, each with a space around. */
    readonly #text: SQLWrapper;

    /** Test a column by its folded words. */
    constructor(search: Search, text: SQLWrapper) {
        this.#search = search;
        this.#text = text;
    }

    /** Test whether the search is all of it. */
    whole(): SQL {
        return sql`${this.#text} = ${sql.param(` ${this.#search.words.join(" ")} `)}::text`;
    }

    /** Test whether it opens with the search. */
    opening(): SQL {
        return sql`starts_with(${this.#text}, ${sql.param(` ${this.#search.words.join(" ")}`)}::text)`;
    }

    /** Test whether it has each token. */
    words(): SQL {
        return this.#has((token) => wholeOf(token));
    }

    /** Test whether it has the start of each token. */
    starts(): SQL {
        return this.#has((token) => startOf(token));
    }

    /** Test whether it has every token as written. */
    #has(write: (token: string) => string): SQL {
        const found = this.#search.tokens.map(
            (token) => sql`position(${sql.param(write(token))}::text in ${this.#text}) > 0`,
        );

        return sql.join(found, sql` AND `);
    }
}

/** Test whether a SQLite index has a row among those it matches, looked up by the row's key. */
function isFound(key: readonly SQLWrapper[], index: SQL, match: string): SQL {
    const found = key.map((_, position) => sql.raw(`json_extract("key", '$[${position}]')`));

    return sql`(${sql.join(key, sql`, `)}) IN (SELECT ${sql.join(found, sql`, `)} FROM ${index} WHERE ${index} MATCH ${sql.param(match)})`;
}

/** Name a searched column in a SQLite index, by its place among them. */
function textColumn(position: number): string {
    return `text${position}`;
}

/** Write a row's key as the one token of it a SQLite index has. */
function rowToken(key: string): string {
    return `${ROW}${key.replace(SEPARATORS, "·")}`;
}

/** Mark the end of each word of a script without spaces, the last left open for a start. */
function marked(words: readonly string[], last: "open" | "ended" = "ended"): string[] {
    return words.flatMap((word, index) => {
        // leave a word of others as it is
        const pairs = Tokenizer.pairs(word);
        if (!Tokenizer.isUnspaced(word)) {
            return pairs;
        }

        // end its pairs, their last character being apart
        const ended = pairs.length > 1 ? pairs.slice(0, -1) : pairs;
        const isOpen = last === "open" && index === words.length - 1;

        return isOpen ? ended : [...ended, WORD_END];
    });
}

/** Write tokens as an FTS5 phrase. */
function phraseOf(tokens: readonly string[]): string {
    return tokens.map((token) => `"${token}"`).join(" + ");
}

/** Write a search as an FTS5 match: each token a quoted start, all required. */
function startsOf(search: Search): string {
    return search.tokens.map((token) => `"${token}"*`).join(" ");
}

/** Write a search as a PostgreSQL tsquery: each token a quoted start, all required. */
function tsqueryOf(search: Search): string {
    return search.tokens.map((token) => `'${token}':*`).join(" & ");
}

/** List what a SQLite index has of a text: its marked words, then each pair's last character. */
function written(text: string): string[] {
    const words = Tokenizer.words(text);
    const last = words.flatMap((word) => {
        const pairs = Tokenizer.pairs(word);

        return pairs.length > 1 ? pairs.slice(-1) : [];
    });

    return [...marked(words), END, ...last];
}

/** Write the tokens of some rows into a SQLite index, a batch per statement. */
async function insertTokens(
    session: Session,
    table: string,
    columns: readonly string[],
    rows: readonly (readonly [key: string, row: Readonly<Record<string, unknown>>])[],
    namespace: string | undefined,
): Promise<void> {
    // cut each column's text into tokens, as many rows at once as the engine binds values for
    const index = quote(relation(FullTextIndex.index(table), namespace));
    const texts = columns.map((_, position) => `"${textColumn(position)}"`).join(", ");
    const width = columns.length + 2;
    const placeholders = `(${Array.from({ length: width }, () => "?").join(", ")})`;
    const size = Math.floor(session.limits.variables / width);
    for (let start = 0; start < rows.length; start += size) {
        const batch = rows.slice(start, start + size).map(([key, row]) => [
            key,
            rowToken(key),
            ...columns.map((column) => {
                const text = SearchedValue.parse(row[column]);

                return text === null ? null : written(text).join(" ");
            }),
        ]);
        await session.run(
            `INSERT INTO ${index} ("key", "row", ${texts}) VALUES ${batch.map(() => placeholders).join(", ")}`,
            batch.flat(),
        );
    }
}

/** Write the PostgreSQL document of some columns: their tokens as lexemes, unparsed. */
function postgresDocument(columns: readonly SQLWrapper[]): SQL {
    // cut scripts without spaces into pairs from their first character, and from their second
    const { unspaced } = Tokenizer;
    const text = sql.join(
        columns.map((column) => sql`coalesce(${column}, '')`),
        sql` || ' ' || `,
    );
    const folded = postgresFolded(text);
    const pairs = (from: SQL) => replaced(from, `([${unspaced}]{1,2})`, "\\1 ");
    const later = replaced(folded, `(^|[^${unspaced}])[${unspaced}]`, "\\1");
    const words = sql`${postgresWords(pairs(folded))} || ${postgresWords(pairs(later))}`;

    return sql`array_to_tsvector(${unlessASCII(text, postgresWords(lowered(text)), words)})`;
}

/** Write the folded words of a PostgreSQL text, each with a space around. */
function postgresLine(text: SQL): SQL {
    const words = postgresWords(postgresFolded(text));

    return sql`(' ' || array_to_string(${unlessASCII(text, postgresWords(lowered(text)), words)}, ' ') || ' ')`;
}

/** Read a PostgreSQL text of ASCII alone by a shorter way, as long in bytes as in characters. */
function unlessASCII(text: SQL, ascii: SQL, other: SQL): SQL {
    return sql`CASE WHEN octet_length(${text}) = length(${text}) THEN ${ascii} ELSE ${other} END`;
}

/** Lower a PostgreSQL text by Unicode. */
function lowered(text: SQL): SQL {
    return sql`lower((${text}) COLLATE ${sql.raw(COLLATION)})`;
}

/** Write a PostgreSQL text as the tokenizer folds it, with a space around scripts without any. */
function postgresFolded(text: SQL): SQL {
    // fold the marks, the letters that are others, and those that are two
    const { letters, ligatures, marks, unspaced } = Tokenizer;
    const unmarked = replaced(lowered(sql`normalize(${text}, NFKD)`), `[${marks}]`, "");
    const translated = sql`translate(${unmarked}, ${literalOf(letters.from)}, ${literalOf(letters.to)})`;
    const folded = ligatures.reduce(
        (left, [letter, pair]) => sql`replace(${left}, ${literalOf(letter)}, ${literalOf(pair)})`,
        translated,
    );

    return replaced(folded, `([${unspaced}]+)`, " \\1 ");
}

/** Write the words of a folded PostgreSQL text as an array. */
function postgresWords(folded: SQL): SQL {
    return sql`array_remove(regexp_split_to_array(${folded}, ${literalOf(SEPARATOR)}), '')`;
}

/** Write a PostgreSQL text with every match of a pattern replaced. */
function replaced(text: SQL, pattern: string, replacement: string): SQL {
    return sql`regexp_replace(${text}, ${literalOf(pattern)}, ${literalOf(replacement)}, 'g')`;
}

/** Write a constant of a PostgreSQL index expression. */
function literalOf(text: string): SQL {
    return sql.raw(literal(text));
}

/** Write a table's key as SQLite's log records it: a JSON array of its key columns. */
function keyOf(state: TableState): string {
    const key = state.table.constraints.flatMap((constraint) =>
        constraint.kind === "primaryKey" ? constraint.columns : [],
    );

    return `json_array(${key.map((column) => quote(column)).join(", ")})`;
}
