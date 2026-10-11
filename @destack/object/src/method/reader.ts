import {
    type Condition,
    type Key,
    Transaction,
    Order,
    type OrderBy,
    type RowImage,
    type Snapshot,
    type Table,
} from "@destack/db";

/** How a read of several rows orders and bounds them. */
export interface ReadOptions<Read> {
    /** How the rows sort, completed by their key. */
    readonly orderBy?: OrderBy<Read>;
    /** The most rows read. */
    readonly limit?: number;
}

/** A handler's reads of one table's logged columns in its call's view: the transaction it runs in on a server, or a snapshot such as a client's copy or a view at a position. */
export class ObjectReader<Definition extends Table> {
    /** The table read. */
    readonly table: Definition;
    /** The transaction or snapshot the reads see. */
    readonly #view: Transaction | Snapshot;

    /** Read a table in a transaction or a snapshot. */
    constructor(table: Definition, view: Transaction | Snapshot) {
        this.table = table;
        this.#view = view;
    }

    /** Read one row by its key, null when absent. */
    row(key: Key<Definition>): Promise<RowImage<Definition> | null> {
        return this.#view instanceof Transaction
            ? this.#view.image(this.table, key)
            : this.#view.row(this.table, key);
    }

    /** Read the rows meeting a condition, every row without one, ordered and bounded as asked. */
    async rows(
        where: Condition<RowImage<NoInfer<Definition>>> = {},
        options: ReadOptions<RowImage<NoInfer<Definition>>> = {},
    ): Promise<RowImage<Definition>[]> {
        // read through the transaction, which orders and bounds itself
        if (this.#view instanceof Transaction) {
            return this.#view.images(this.table, where, options);
        }

        // order and bound a snapshot's rows in memory
        const rows = await this.#view.rows(this.table, where);
        const order = Order.complete(Order.of(options.orderBy ?? {}), this.table);
        const sorted = rows.toSorted((left, right) => Order.rows(order, left, right));

        return options.limit === undefined ? sorted : sorted.slice(0, options.limit);
    }

    /** Count the rows meeting a condition. */
    async count(where: Condition<RowImage<NoInfer<Definition>>> = {}): Promise<number> {
        return (await this.rows(where)).length;
    }
}
