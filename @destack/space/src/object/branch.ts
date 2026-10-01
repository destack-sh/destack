import { principal, relation, through, union } from "@destack/access";
import { type Select, type Table, unique } from "@destack/db";
import { LogPosition } from "@destack/db/log";
import { Branch, BranchType, defineObject, field, method } from "@destack/object";
import { ServiceError } from "@destack/service/error";
import { schema } from "@destack/schema";
import { Call } from "@destack/sync";
import { space } from "./space.ts";

/** The longest branch title, a sentence's worth. */
const MAX_TITLE_LENGTH = 200;
/** The most calls one push appends, a long editing session's worth. */
const MAX_PUSHED_CALLS = 1000;

/** Append calls to an open branch as the caller's, and put their rows over the branch's. */
const push = method({
    permission: "push",
    input: schema.object({
        /** The calls to append, in order. */
        calls: schema.array(Call).min(1).max(MAX_PUSHED_CALLS),
    }),
}).handle(async (call) => {
    // refuse a closed branch, and calls the served types refuse
    const target = branchType.requireOpen(call.target as Target);
    const calls = (call.input as { readonly calls: readonly Call[] }).calls;
    for (const entry of calls) {
        branchType.resolve(call.objects, entry, call.scope);
    }

    // append the calls, and replay only them over the branch's rows
    const opened = new Branch(branchType, call.database, target.id);
    const appended = await opened.append(call, calls);

    return call.revise(await opened.build(call, target, appended, await opened.rows()));
});

/** Merge an open branch: its calls run first in the same mutation, each as its author, then the branch closes. */
const merge = method({ permission: "merge" }).handle({
    expand: (call) => new Branch(branchType, call.database, call.id!).calls(),
    effect: async (call) => {
        // refuse calls pushed after the server's expansion read them
        const target = branchType.requireOpen(call.target as Target);
        const opened = new Branch(branchType, call.database, target.id);
        const expansion = call.expansion;
        if (expansion !== undefined && (await opened.calls()).length !== expansion.length) {
            throw new ServiceError("CONFLICT", {
                message: "branch has calls the merge did not run",
            });
        }

        // drop the rows the merge wrote for real, and close the branch
        await opened.store(call, []);

        return call.revise({ state: "merged" });
    },
});

/** Close an open branch without replaying its calls. */
const discard = method({ permission: "discard" }).handle(async (call) => {
    // drop the branch's rows, and close it
    const target = branchType.requireOpen(call.target as Target);
    await new Branch(branchType, call.database, target.id).store(call, []);

    return call.revise({ state: "discarded" });
});

/** Rebuild an open branch's rows once the main line changed a table they reach. */
const rebuild = method({ permission: null, isSystem: true }).handle(async (call) => {
    // keep a closed branch, and rows the main line has not moved under
    const target = call.target as Target;
    const opened = new Branch(branchType, call.database, target.id);
    if (target.state !== "open" || !(await opened.isStale(call.objects, target))) {
        return target;
    }

    // replay every call over the main line

    return call.revise(await opened.build(call, target, await opened.calls(), []));
});

/** A line of calls off a space's main line, merged by replaying them. */
export const branch = defineObject({
    name: "branch",
    plural: "branches",
    scope: space,
    fields: {
        /** The branch's title. */
        title: field.string(schema.string().min(1).max(MAX_TITLE_LENGTH)),
        /** The person who created the branch. */
        author: field.reference(principal.user).caller(),
        /** Whether the branch is open, or merged or discarded for good. */
        state: field.enum(["open", "merged", "discarded"]).default("open"),
        /** The log position of the main line the branch starts from. */
        base: field.json(LogPosition),
        /** The log position the branch's rows were last built at. */
        built: field.json(LogPosition),
        /** The SQL names of the tables the branch's calls read or write. */
        reach: field.json(schema.array(schema.string())).default([]),
    },
    relations: {
        /** People who read, push to and merge the branch. */
        collaborator: { subjects: [principal.user] },
    },
    permissions: {
        read: union(relation("author"), relation("collaborator")),
        push: union(relation("author"), relation("collaborator")),
        merge: union(relation("author"), relation("collaborator")),
        discard: relation("author"),
        share: relation("author"),
    },
    shareable: { by: "share" },
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("push", { fields: ["title"] }).handle(async (call, next) => {
            const position = await call.database.log.position();

            return next(call.with({ input: { ...call.input, base: position, built: position } }));
        }),
        push,
        merge,
        discard,
        rebuild,
    },
});

/** One call of a branch, in the order it replays. */
export const branchCall = defineObject({
    name: "branch-call",
    plural: "branchCalls",
    scope: space,
    nested: { in: branch, delete: "cascade", receive: "push" },
    fields: {
        /** The call's place on the branch. */
        position: field.integer(),
        /** The object type and method, such as page.create. */
        method: field.string(),
        /** The method's input, as the call recorded it. */
        input: field.json(schema.record(schema.string(), schema.json())),
        /** The release of the object type's package the call was made against. */
        release: field.string(),
        /** The principal who made the call, whom a merge records as its caller. */
        author: field.subject(),
    },
    constraints: (entry) => [unique("branch_call_position").on(entry.parentId, entry.position)],
    permissions: { read: through("parent", "read") },
    methods: {
        list: method.list("read"),
        create: method.create(null, {
            isSystem: true,
            fields: ["position", "method", "input", "release", "author"],
        }),
    },
});

/** A row a branch changes over the main line, as its calls leave it. */
export const branchRow = defineObject({
    name: "branch-row",
    plural: "branchRows",
    scope: space,
    nested: { in: branch, delete: "cascade", receive: "push" },
    fields: {
        /** The SQL name of the row's table. */
        table: field.string(),
        /** The row's key columns. */
        key: field.json(schema.record(schema.string(), schema.json())),
        /** The row as the branch leaves it, absent when the branch removes it. */
        row: field.json(schema.record(schema.string(), schema.json())).optional(),
        /** The main line's row the branch replaces, absent when the branch creates it. */
        before: field.json(schema.record(schema.string(), schema.json())).optional(),
    },
    permissions: { read: through("parent", "read") },
    methods: {
        list: method.list("read"),
        create: method.create(null, {
            isSystem: true,
            fields: ["table", "key", "row", "before"],
        }),
        update: method.update(null, { isSystem: true, fields: ["row", "before"] }),
        delete: method.delete(null, { isSystem: true }),
    },
});

/** A space's branch types. */
export const branchType = new BranchType({ object: branch, call: branchCall, row: branchRow });

/** The objects of a space's branches, served by every app beside its own. */
export const branchObjects = { branch, branchCall, branchRow } as const;

/** The tables of a space's branches, in every app's database. */
export const branchTables: readonly Table[] = [
    ...branch.tables,
    ...branchCall.tables,
    ...branchRow.tables,
];

/** A branch as its table stores it. */
type Target = Select<typeof branch.table>;
