import type { Permission } from "@destack/access";
import { type LogPosition, sql, type Row, type SQL, TABLE, type Table } from "@destack/db";
import type { Audience, Watch } from "@destack/sync";
import type { MethodContext } from "../method/context.ts";
import type { ObjectType } from "../object/object.ts";
import type { Execution } from "../method/execution.ts";
import type {} from "@destack/package/import-meta";

/** The rows one call's caller may read, decided once. */
export class CallAudience implements Audience {
    /** The tables whose changes decide visibility, none. */
    readonly watches: readonly Watch[] = [];
    /** The name of the decisions, one per call. */
    readonly key = "call";
    /** The call reading the objects. */
    readonly #context: MethodContext;
    /** Where the call executes. */
    readonly #execution: Execution;
    /** The object types the call reads, by table. */
    readonly #objects: ObjectTypeIndex;

    /** Decide rows as a call's caller may read them. */
    constructor(context: MethodContext, execution: Execution) {
        this.#context = context;
        this.#execution = execution;
        this.#objects = new ObjectTypeIndex(context.objects);
    }

    /** Match a superset of the rows the caller may list, which `visible` decides. */
    async candidates(table: Table, permission?: string): Promise<SQL> {
        // list every row of a prediction
        const { object, permission: listing } = this.#objects.listed(table);
        const execution = this.#execution;
        if (execution.kind === "client") {
            return sql`true`;
        }

        return execution.authorization.candidates(
            object,
            listing,
            this.#required(object, permission),
        );
    }

    /** Match exactly the rows the caller may list, for reads of related rows. */
    async readable(table: Table, permission?: string): Promise<SQL> {
        // read every related row of a prediction
        const { object, permission: listing } = this.#objects.listed(table);
        const execution = this.#execution;
        if (execution.kind === "client") {
            return sql`true`;
        }

        return execution.authorization.readable(
            object,
            listing,
            this.#required(object, permission),
        );
    }

    /** Decide which rows of a table the caller may list, under a query's permission too. */
    async visible(
        table: Table,
        rows: readonly Row[],
        _position: LogPosition,
        permission?: string,
    ): Promise<ReadonlySet<number>> {
        // show every row of a prediction
        const { object, permission: listing } = this.#objects.listed(table);
        const execution = this.#execution;
        if (execution.kind === "client") {
            return new Set(rows.keys());
        }
        const required = this.#required(object, permission);
        const { rows: allowed } = await execution.authorization.filter(object, listing, rows, {
            scope: this.#context.scope,
            ...(required === undefined ? {} : { required }),
        });
        const visible = new Set<Row>(allowed);

        return new Set(rows.flatMap((row, position) => (visible.has(row) ? [position] : [])));
    }

    /** List an object type's guarded fields. */
    concealable(table: Table): readonly string[] {
        return this.#objects.concealable(table);
    }

    /** List the guarded fields the caller may not read on each row. */
    async conceals(table: Table, rows: readonly Row[]): Promise<readonly (readonly string[])[]> {
        // conceal nothing from a prediction
        const { object } = this.#objects.listed(table);
        const execution = this.#execution;
        if (execution.kind === "client") {
            return rows.map(() => []);
        }
        const concealed = await execution.authorization.concealed(object, rows);

        return concealed.hidden;
    }

    /** Read no expiry. */
    async until(): Promise<number | undefined> {
        return undefined;
    }

    /** Decide nothing again. */
    async refresh(): Promise<void> {}

    /** Decide nothing beyond rows themselves. */
    async decides(): Promise<boolean> {
        return false;
    }

    /** Read a query's permission on a type, absent without one. */
    #required(object: ObjectType, permission: string | undefined): Permission | undefined {
        return permission === undefined ? undefined : object.permission(permission);
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
            throw new TypeError(`no listed object is stored in ${table[TABLE].name}`);
        }

        return { object, permission: object.listing };
    }

    /** List an object type's guarded fields. */
    concealable(table: Table): readonly string[] {
        return this.#objects.get(table)?.guarded ?? [];
    }
}
