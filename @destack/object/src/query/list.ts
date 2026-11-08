import { Caller } from "@destack/access";
import { ObjectError } from "../error/error.ts";
import { type Snapshot, TABLE, Transaction, Table } from "@destack/db";
import { nth, canonicalize, schema } from "@destack/schema";
import { Dataflow, Replica, View } from "@destack/sync";
import type * as sync from "@destack/sync";
import { Page } from "../method/page.ts";
import { PageFields } from "../method/procedure.ts";
import { QueryFields, ViewFields } from "../replica/replica.ts";
import { EventLog } from "./event.ts";
import { nestItem } from "./item.ts";
import type { MethodContext } from "../method/context.ts";
import { Execution } from "../method/execution.ts";
import { CallAudience } from "./audience.ts";
import type {} from "@destack/package/import-meta";

/** The query, page and view of a list's input, beside its scope fields. */
const LIST_INPUT = schema.looseObject({ ...QueryFields, ...PageFields, ...ViewFields });
/** The query, page and view of a list's input. */
export type ListInput = schema.Infer<typeof LIST_INPUT>;

/** Read a page of a list call's query rows or its aggregate groups. */
export async function list(call: MethodContext, execution: Execution) {
    // read an events type's rows from the event log on a server, hot and flushed alike
    const input = LIST_INPUT.parse(call.input);
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
        ...call.relations.compile(
            call.object,
            options.aggregate === undefined ? { ...options, limit: listing.limit + 1 } : options,
        ),
        scopes: [...new Set([...call.object.readScopes(call.chain), ...replicated])],
    };
    const dataflow = listDataflow(compiled, new CallAudience(call, execution), execution);
    const node = nth(dataflow.roots, 0);

    // read groups at the latest position, or rows
    const requested = requestedView(execution);
    const position = requested?.position;
    const after = listing.after === undefined ? undefined : Table.decode(node.table, listing.after);
    const view = Execution.snapshot(execution);
    await dataflow.load(await View.of(view.database, requested), after);
    if (node.aggregate !== undefined && position !== undefined) {
        throw ObjectError.of(
            "AGGREGATE_AT_POSITION",
            "aggregates can only read the latest position, where access is decided",
        );
    } else if (node.aggregate !== undefined) {
        return { items: [], cursor: null, groups: await dataflow.results("list") };
    }

    // filter the items the caller could read and split includes and computed values
    const readable = await readableAt(await dataflow.read("list"), call, execution);
    const listed = listing.result(readable, (item) =>
        node.table[TABLE].encode(node.position(item.row)),
    );

    return { ...listed, ...split(listed.items, node, compiled) };
}

/** Build the dataflow listing a call's query, materialized for the call's audience. */
function listDataflow(
    compiled: sync.Query,
    audience: CallAudience,
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
    const snapshot = requestedView(execution);
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
function requestedView(execution: Execution): Snapshot | undefined {
    return execution.kind === "store" && !(execution.view instanceof Transaction)
        ? execution.view
        : undefined;
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
