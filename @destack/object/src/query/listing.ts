import type { Permission } from "@destack/access";
import { sql, TABLE, type Row, type SQL, type Table } from "@destack/db";
import type { RowKey, Audience, Watch } from "@destack/sync";
import type { Call } from "../method/call.ts";
import type { ObjectType } from "../object/object.ts";

/** The rows one call's caller may list, decided once. */
export class Listing implements Audience {
    /** The tables whose changes decide visibility, none. */
    readonly watches: readonly Watch[] = [];
    /** The name of the decisions, one per call. */
    readonly key = "call";
    /** The call listing the objects. */
    readonly #call: Call;
    /** The object types the call lists, by table. */
    readonly #objects: ObjectTypeIndex;

    /** List objects as a call's caller may list them. */
    constructor(call: Call) {
        this.#call = call;
        this.#objects = new ObjectTypeIndex(call.objects);
    }

    /** Match the rows the caller may list. */
    where(table: Table): SQL | "memory" {
        // list every row of a prediction
        const { object, permission } = this.#objects.listed(table);
        if (this.#call.isPredicted) {
            return sql`true`;
        }

        return this.#call.requireAuthorization().listable(object, permission);
    }

    /** Decide which rows of a table the caller may list, at once. */
    async admits(table: Table, rows: readonly Row[]): Promise<ReadonlySet<number>> {
        // admit every row of a prediction
        const { object, permission } = this.#objects.listed(table);
        if (this.#call.isPredicted) {
            return new Set(rows.keys());
        }
        const authorization = this.#call.requireAuthorization();
        const admitted = await authorization.admitRows(object, permission, this.#call.scope, rows);

        return admitted.permitted;
    }

    /** List an object type's guarded fields. */
    concealable(table: Table): readonly string[] {
        return this.#objects.concealable(table);
    }

    /** List the guarded fields the caller may not read on each row. */
    async conceals(table: Table, rows: readonly Row[]): Promise<readonly (readonly string[])[]> {
        // conceal nothing from a prediction
        const { object } = this.#objects.listed(table);
        if (this.#call.isPredicted) {
            return rows.map(() => []);
        }
        const concealed = await this.#call.requireAuthorization().concealed(object, rows);

        return concealed.hidden;
    }

    /** Read no expiry. */
    async until(): Promise<number | undefined> {
        return undefined;
    }

    /** Decide nothing again. */
    async refresh(): Promise<void> {}

    /** List no dependents. */
    async dependents(): Promise<readonly RowKey[]> {
        return [];
    }
}

/** Object types by their tables. */
export class ObjectTypeIndex {
    /** The object types, by table. */
    readonly #objects: ReadonlyMap<Table, ObjectType>;

    /** Index object types by their tables. */
    constructor(objects: readonly ObjectType[]) {
        this.#objects = new Map(objects.map((object) => [object.table, object]));
    }

    /** Read the object type of a table, absent for another table. */
    get(table: Table): ObjectType | undefined {
        return this.#objects.get(table);
    }

    /** Read a table's listed object type and listing permission. */
    listed(table: Table): { readonly object: ObjectType; readonly permission: Permission } {
        const object = this.#objects.get(table);
        if (object?.listing === undefined) {
            throw new TypeError(`no listed object lives in ${table[TABLE].name}`);
        }

        return { object, permission: object.listing };
    }

    /** List an object type's guarded fields. */
    concealable(table: Table): readonly string[] {
        return this.#objects.get(table)?.guarded ?? [];
    }
}
