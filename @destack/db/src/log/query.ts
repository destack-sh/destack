import type { QueryOptions } from "../query/option.ts";
import { Key } from "../query/key.ts";
import type { RowImage, Table } from "../table/table.ts";

/** The relational queries of one table's rows as a view shows them, as Drizzle's `db.query`. */
export class RelationalQueryBuilder<Definition extends Table, Read = RowImage<Definition>> {
    /** The queried table. */
    readonly #table: Definition;
    /** Read the rows meeting a query through the view. */
    readonly #rows: (query: QueryOptions<RowImage<Definition>>) => Promise<Read[]>;
    /** Read one row by its key through the view, null when absent. */
    readonly #row: (key: Key) => Promise<Read | null>;

    /** Query a table through a view's scan and point reads. */
    constructor(
        table: Definition,
        rows: (query: QueryOptions<RowImage<Definition>>) => Promise<Read[]>,
        row: (key: Key) => Promise<Read | null>,
    ) {
        this.#table = table;
        this.#rows = rows;
        this.#row = row;
    }

    /** Read the rows meeting a condition, ordered and bounded as asked. */
    findMany(options: QueryOptions<RowImage<NoInfer<Definition>>> = {}): Promise<Read[]> {
        return this.#rows(options);
    }

    /** Read the first row meeting a condition, a key's row by one point read. */
    async findFirst(
        options: Omit<QueryOptions<RowImage<NoInfer<Definition>>>, "limit"> = {},
    ): Promise<Read | undefined> {
        // read a key's row by one point read
        const key =
            options.orderBy === undefined ? Key.from(this.#table, options.where ?? {}) : undefined;
        if (key !== undefined) {
            return (await this.#row(key)) ?? undefined;
        }

        // read the first of the ordered rows
        const [first] = await this.#rows({ ...options, limit: 1 });

        return first;
    }
}
