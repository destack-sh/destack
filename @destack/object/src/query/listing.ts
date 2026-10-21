import { Caller, type Permission } from "@destack/access";
import {
    type LogPosition,
    type Snapshot,
    sql,
    TABLE,
    Transaction,
    type Row,
    type SQL,
    Table,
} from "@destack/db";
import { CallError } from "../error/call.ts";
import { nth, canonicalize, schema } from "@destack/schema";
import { type Audience, Dataflow, Replica, View, type Watch } from "@destack/sync";
import type * as sync from "@destack/sync";
import { Page } from "../method/page.ts";
import { PageShape } from "../method/procedure.ts";
import { QueryShape, ViewShape } from "../replica/replica.ts";
import { EventLog } from "./event.ts";
import { nestItem } from "./item.ts";
import type { MethodContext } from "../method/context.ts";
import type { ObjectType } from "../object/object.ts";
import { Execution } from "../method/execution.ts";

/** The query, page and view of a list's input, beside its scope fields. */
const LIST_INPUT = schema.looseObject({ ...QueryShape, ...PageShape, ...ViewShape });
/** The query, page and view of a list's input. */
export type ListInput = schema.Infer<typeof LIST_INPUT>;

/** The rows one call's caller may list, decided once. */
export class Listing implements Audience {
    /** The tables whose changes decide visibility, none. */
    readonly watches: readonly Watch[] = [];
    /** The name of the decisions, one per call. */
    readonly key = "call";
    /** The call listing the objects. */
    readonly #context: MethodContext;
    /** Where the call executes. */
    readonly #execution: Execution;
    /** The object types the call lists, by table. */
    readonly #objects: ObjectTypeIndex;

    /** List objects as a call's caller may list them. */
    constructor(context: MethodContext, execution: Execution) {
        this.#context = context;
        this.#execution = execution;
        this.#objects = new ObjectTypeIndex(context.objects);
    }

    /** Read a page of the call's query rows or its aggregate groups. */
    async list() {
        // read an events type's rows from the event log on a server, hot and flushed alike
        const call = this.#context;
        const input = LIST_INPUT.parse(call.input);
        const execution = this.#execution;
        if (call.object.storage === "events" && execution.kind === "store") {
            return EventLog.page(call, execution, input);
        }

        // bind the cursor to the query
        const { cursor, limit, at: _at, branch: _branch, ...options } = input;
        const listing = new Page(
            {
                ...(cursor === undefined ? {} : { cursor }),
                ...(limit === undefined ? {} : { limit }),
            },
            [call.object.name, call.scope, canonicalize(options)],
            schema.jsonObject(),
        );

        // compile the query with one extra row over the scopes the caller sees
        const replicated = await Replica.scopes(
            Execution.snapshot(execution).database,
            call.object.table,
            call.scope,
            Caller.principal(call.caller),
        );
        const compiled = {
            ...call.object.compile(
                options.aggregate === undefined
                    ? { ...options, limit: listing.limit + 1 }
                    : options,
                call.objects,
            ),
            scopes: [...new Set([...call.object.readScopes(call.chain), ...replicated])],
        };
        const dataflow = this.#dataflow(compiled);
        const node = nth(dataflow.roots, 0);

        // read groups at the latest position, or rows
        const requested = this.#requested();
        const position = requested?.position;
        const after =
            listing.after === undefined ? undefined : Table.decode(node.table, listing.after);
        const view = Execution.snapshot(execution);
        await dataflow.load(await View.of(view.database, requested), after);
        if (node.aggregate !== undefined && position !== undefined) {
            throw new CallError("BAD_REQUEST", {
                message: "aggregates read only the latest position, where access decides them",
            });
        } else if (node.aggregate !== undefined) {
            return { items: [], cursor: null, groups: await dataflow.results("list") };
        }

        // filter the items the caller could read and split includes and computed values
        const readable = await this.#readable(await dataflow.read("list"));
        const listed = listing.result(readable, (item) =>
            node.table[TABLE].encode(node.position(item.row)),
        );

        return { ...listed, ...Listing.#split(listed.items, node, compiled) };
    }

    /** Build the dataflow listing a call's query, materialized for the call's audience. */
    #dataflow(compiled: sync.Query): Dataflow {
        const { database } = Execution.snapshot(this.#execution);

        return new Dataflow(
            { list: compiled },
            {
                audience: this,
                database,
                changesThrough: Dataflow.changesThrough(database),
                isMaterialized: true,
            },
        );
    }

    /** Filter the items whose rows the caller could read at the snapshot position. */
    async #readable(read: readonly sync.Item[]): Promise<readonly sync.Item[]> {
        // take every item at the latest position
        const call = this.#context;
        const execution = this.#execution;
        const snapshot = this.#requested();
        if (snapshot?.position === undefined || execution.kind !== "store") {
            return read;
        }

        // filter the items whose rows access permits at the snapshot position
        const permitted = new Set(
            await execution.authorization.filterAt(
                read.map((item) => item.row),
                call.permission(),
                snapshot,
                call.scope,
            ),
        );

        return read.filter((item) => permitted.has(item.row));
    }

    /** Read the snapshot a store's call asked to read at a position or over a branch. */
    #requested(): Snapshot | undefined {
        const execution = this.#execution;

        return execution.kind === "store" && !(execution.view instanceof Transaction)
            ? execution.view
            : undefined;
    }

    /** Split listed items into their rows, their included rows and their computed values. */
    static #split(listed: readonly sync.Item[], node: sync.Node, compiled: sync.Query) {
        // list the included relations and the computed values
        const names = node.children
            .filter((child) => child.kind === "include")
            .map((child) => child.name.slice(node.name.length + 1));
        const extras = Object.keys(node.extras);
        const values = Object.fromEntries(
            listed.map(({ row }) => [
                schema.string().parse(row["id"]),
                Object.fromEntries(extras.map((name) => [name, node.scalar(name, row[name])])),
            ]),
        );

        // index the included rows by relation and item
        const nestedRows = listed.map((item) => nestItem(item, compiled));
        const included = Object.fromEntries(
            names.map((name) => [
                name,
                Object.fromEntries(
                    nestedRows.map((row) => [
                        schema.string().parse(row["id"]),
                        schema.json().parse(row[name]),
                    ]),
                ),
            ]),
        );

        // strip the computed values from the rows
        const items = listed.map(({ row }) =>
            Object.fromEntries(Object.entries(row).filter(([name]) => !extras.includes(name))),
        );

        return {
            items,
            ...(names.length === 0 ? {} : { included }),
            ...(extras.length === 0 ? {} : { extras: values }),
        };
    }

    /** Match the rows the caller may list, under a query's own permission too. */
    where(table: Table, permission?: string): SQL | "memory" {
        // list every row of a prediction
        const { object, permission: listing } = this.#objects.listed(table);
        const execution = this.#execution;
        if (execution.kind === "client") {
            return sql`true`;
        }
        const required = permission === undefined ? undefined : object.permission(permission);

        return execution.authorization.where(object, listing, required);
    }

    /** Decide which rows of a table the caller may list, under a query's own permission too. */
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
        const { authorization } = execution;
        const { rows: allowed } = await authorization.filter(object, listing, rows, {
            scope: this.#context.scope,
            ...(permission === undefined ? {} : { required: object.permission(permission) }),
        });
        const kept = new Set<Row>(allowed);

        return new Set(rows.flatMap((row, position) => (kept.has(row) ? [position] : [])));
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
