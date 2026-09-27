import { sql, TABLE, type Table } from "@destack/db";
import { Condition, type Match } from "@destack/db/query";
import type { Audience } from "../audience.ts";
import type { Row } from "@destack/db";

/** An audience whose visibility and concealment conditions decide rows alike in SQL and in memory. */
export class ConditionAudience implements Audience {
    /** The tables whose changes decide visibility: none, since conditions read the row itself. */
    readonly watches = [];
    /** The name of the conditions deciding visibility and concealment, by table. */
    readonly key: string;
    /** The condition a table's visible rows meet, all rows of other tables being visible. */
    readonly #visible: ReadonlyMap<Table, Condition>;
    /** The conditions compiled so far, by condition. */
    readonly #matches = new Map<Condition, Match>();
    /** The columns hidden on a table's rows meeting a condition. */
    readonly #concealed: ReadonlyMap<
        Table,
        { readonly when: Condition; readonly columns: readonly string[] }
    >;

    /** Decide visibility and concealment by conditions per table. */
    constructor(
        visible: ReadonlyMap<Table, Condition>,
        concealed: ReadonlyMap<
            Table,
            { readonly when: Condition; readonly columns: readonly string[] }
        > = new Map(),
    ) {
        this.#visible = visible;
        this.#concealed = concealed;
        this.key = JSON.stringify([
            [...visible].map(([table, condition]) => [table[TABLE].sqlName, condition]),
            [...concealed].map(([table, hidden]) => [table[TABLE].sqlName, hidden]),
        ]);
    }

    /** Match the visible rows of a table in SQL. */
    where(table: Table) {
        const condition = this.#visible.get(table);

        return condition === undefined
            ? sql`true`
            : Condition.render(condition, Condition.bind(table));
    }

    /** Decide which rows of a table are visible. */
    async admits(table: Table, rows: readonly Row[]): Promise<ReadonlySet<number>> {
        return new Set(rows.flatMap((row, index) => (this.isVisible(table, row) ? [index] : [])));
    }

    /** List the columns hidden on some rows of a table. */
    concealable(table: Table): readonly string[] {
        return this.#concealed.get(table)?.columns ?? [];
    }

    /** List the columns hidden on each row. */
    async conceals(table: Table, rows: readonly Row[]): Promise<readonly (readonly string[])[]> {
        return rows.map((row) => this.hidden(table, row));
    }

    /** Decide the same at every time. */
    async until() {
        return undefined;
    }

    /** Keep deciding as before. */
    async refresh() {}

    /** Decide nothing beyond rows themselves. */
    async dependents() {
        return [];
    }

    /** Decide whether a row is visible, in memory. */
    isVisible(table: Table, row: Row): boolean {
        const condition = this.#visible.get(table);

        return condition === undefined || Condition.matches(this.#compiled(condition, table), row);
    }

    /** List the columns hidden on a row, in memory. */
    hidden(table: Table, row: Row): readonly string[] {
        const concealed = this.#concealed.get(table);

        return concealed !== undefined &&
            Condition.matches(this.#compiled(concealed.when, table), row)
            ? concealed.columns
            : [];
    }

    /** Compile a condition of a table once. */
    #compiled(condition: Condition, table: Table): Match {
        let match = this.#matches.get(condition);
        if (match === undefined) {
            match = Condition.compile(condition, table);
            this.#matches.set(condition, match);
        }

        return match;
    }
}

/** Decide a condition on an application row of a table. */
export function matches(condition: Condition, table: Table, row: Row): boolean {
    return Condition.matches(Condition.compile(condition, table), row);
}
