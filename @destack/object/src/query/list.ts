import { ReplicaSet } from "@destack/sync";
import { Caller } from "@destack/access";
import { ObjectError } from "../error/error.ts";
import { TABLE, Table } from "@destack/db";
import type { Snapshot } from "@destack/db/database";
import { nth, canonicalize, schema, type JsonObject } from "@destack/schema";
import { Dataflow, EVERYONE, View } from "@destack/sync/dataflow";
import type * as sync from "@destack/sync";
import type * as syncDataflow from "@destack/sync/dataflow";
import { Page } from "../method/page.ts";
import { PageFields } from "../method/procedure.ts";
import { QueryFields, ViewFields } from "../replica/replica.ts";
import { EventLog } from "./event.ts";
import { nestItem } from "./item.ts";
import type { MethodContext } from "../method/context.ts";
import { Execution } from "../method/execution.ts";
import type {} from "@destack/package/import-meta";

/** The query, page and view of a list's input, beside its scope fields. */
const LIST_INPUT = schema.looseObject({ ...QueryFields, ...PageFields, ...ViewFields });
/** The query, page and view of a list's input. */
export type ListInput = schema.Infer<typeof LIST_INPUT>;

/** Read a page of a list call's query rows or its aggregate groups. */
export async function list(call: MethodContext, execution: Execution) {
    // read an events type's rows from the event log at the home, hot and flushed alike
    const input = LIST_INPUT.parse(call.input);
    if (call.object.storage === "events" && !execution.isPredicted) {
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
    const replicated = await new ReplicaSet(Execution.snapshot(execution).database).scopes(
        call.object.table,
        call.scope,
        Caller.principal(call.caller),
    );
    const compiled = {
        ...call.relations.compile(
            call.object,
            options.aggregate === undefined ? { ...options, limit: listing.limit + 1 } : options,
        ),
        scopes: [...new Set([...call.object.readScopes(call.chain), ...replicated])],
    };
    const requested = execution.view;
    const position = requested?.position;
    const read = await readList(
        call,
        execution,
        compiled,
        listing.after,
        requested,
        async (dataflow) => {
            // take the root and its groups or rows
            const node = nth(dataflow.roots, 0);

            return node.aggregate === undefined
                ? { node, items: await dataflow.read("list") }
                : { node, groups: await dataflow.results("list") };
        },
    );
    const { node } = read;

    // answer groups at the latest position, refusing an aggregate at another
    if (node.aggregate !== undefined && position !== undefined) {
        throw ObjectError.of(
            "AGGREGATE_AT_POSITION_INVALID",
            "aggregate position is invalid: aggregates can only read the latest position, where access is decided",
        );
    } else if ("groups" in read) {
        return { items: [], cursor: null, groups: read.groups };
    }

    // filter the items the caller could read and split includes and computed values
    const readable = await readableAt(read.items, call, execution);
    const listed = listing.result(readable, (item) =>
        node.table[TABLE].encode(node.position(item.row)),
    );

    return { ...listed, ...split(listed.items, node, compiled) };
}

/** Read a call's query through the feed at the committed head, else over its own view. */
async function readList<Value>(
    call: MethodContext,
    execution: Execution,
    compiled: sync.Query,
    cursor: JsonObject | undefined,
    requested: Snapshot | undefined,
    use: (dataflow: Dataflow) => Promise<Value>,
): Promise<Value> {
    // read the committed head through the shared dataflow when the call's view is the committed state
    const after = cursor === undefined ? undefined : Table.decode(call.object.table, cursor);
    const view = Execution.snapshot(execution);
    const isCommitted =
        !execution.isPredicted &&
        requested === undefined &&
        call.object.storage === "durable" &&
        view.database.state === execution.store.database.state &&
        execution.transaction.changes.writes("every") === 0;
    if (isCommitted) {
        const audience = await execution.store.audience(call.scope, execution.authorization);

        return execution.store.feed.read(
            { list: compiled },
            audience,
            after === undefined ? undefined : { row: after, cursor: canonicalize(cursor) },
            use,
        );
    }

    // read the call's own view: a prediction, a requested position, or its uncommitted writes
    const audience = execution.isPredicted
        ? EVERYONE
        : await execution.store.audience(call.scope, execution.authorization, call.object.storage);
    const dataflow = listDataflow(compiled, audience, execution);
    await dataflow.load(await View.of(view.database, requested), after);

    return use(dataflow);
}

/** Build the dataflow listing a call's query, materialized for the call's audience. */
function listDataflow(
    compiled: sync.Query,
    audience: syncDataflow.Audience,
    execution: Execution,
): Dataflow {
    const { database } = Execution.snapshot(execution);

    return new Dataflow(
        { list: compiled },
        {
            audience,
            database,
            changesThrough: Dataflow.changesThrough(database),
            isMaterialized: true,
        },
    );
}

/** Filter the items whose rows the caller could read at the snapshot position. */
async function readableAt(
    read: readonly sync.Item[],
    call: MethodContext,
    execution: Execution,
): Promise<readonly sync.Item[]> {
    // take every item at the latest position
    const snapshot = execution.view;
    if (snapshot?.position === undefined || execution.isPredicted) {
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

/** Split listed items into their rows, their included rows and their computed values. */
function split(listed: readonly sync.Item[], node: sync.Node, compiled: sync.Query) {
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
