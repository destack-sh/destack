import { none, principal, relation, through, union } from "@destack/access";
import { LogPosition, type Table } from "@destack/db";
import { Branch, BranchType, defineObject, field, type MethodContext } from "@destack/object";
import { approval, ApprovalQuorum, changeRequest } from "@destack/workflow";
import { type Identifier, schema } from "@destack/schema";
import { Call, ObjectTypeReference } from "@destack/sync";
import { SettingValue } from "@destack/setting/object";
import { branchApprovals } from "../setting/index.ts";
import { space } from "@destack/account/object";
import type {} from "@destack/package/import-meta";
import { StackError } from "../error/index.ts";

/** The longest branch title, a sentence's worth. */
const MAX_TITLE_LENGTH = 200;

/** The most calls one push appends, a long editing session's worth. */
const MAX_PUSHED_CALLS = 1000;

/** A line of calls off a space's main line, merged by replaying them. */
export const branch = defineObject(
    {
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
            tables: field.json(schema.array(schema.string())).default([]),
        },
        relations: {
            /** People who read, push to and merge the branch. */
            collaborator: { subjects: [principal.user] },
        },
        permissions: {
            read: union(relation("author"), relation("collaborator")),
            push: union(relation("author"), relation("collaborator")),
            merge: union(relation("author"), relation("collaborator")),
            approve: union(relation("author"), relation("collaborator")),
            discard: relation("author"),
            share: relation("author"),
            rebuild: none(),
        },
        shareable: { by: "share" },
        attachments: [changeRequest.attach({ by: "approve" })],
    },
    (method) => ({
        create: method.create("push", { fields: ["title"] }),
        /** Append calls to an open branch as the caller's, and put their rows over the branch's. */
        push: method.mutation({
            permission: "push",
            input: schema.object({
                /** The calls to append, in order. */
                calls: schema.array(Call).min(1).max(MAX_PUSHED_CALLS),
            }),
        }),

        /** Merge an open branch: run its calls in one mutation as their authors and close it. */
        merge: method.mutation({ permission: "merge" }),

        /** Close an open branch without replaying its calls. */
        discard: method.mutation({ permission: "discard" }),

        /** Rebuild an open branch's rows once the main line changed a table they read. */
        rebuild: method.mutation({ permission: "rebuild", visibility: "package" }),
    }),
).implement({
    methods: {
        create: async (call, next) => {
            // start the branch at the mutation's base position
            const position = call.base;

            return next(call.with({ input: { ...call.input, base: position, built: position } }));
        },
        push: async (call) => {
            // refuse a closed branch and calls the served types refuse
            const target = branchType.requireOpen(call.target);
            for (const entry of call.input.calls) {
                branchType.resolve(call.objects, entry, call.scope);
            }

            // append the calls and replay only them over the branch's rows
            const opened = new Branch(branchType, call, target.id);
            const appended = await opened.append(call, call.input.calls);

            return call.update(await opened.build(call, target, appended, await opened.rows()));
        },
        merge: {
            expand: (context) => new Branch(branchType, context, context.target.id).entries(),
            handle: async (context) => {
                // refuse a branch the space requires approvals of, unless its change request has them
                const target = branchType.requireOpen(context.target);
                await requireReviewed(context, target.id);

                // refuse calls pushed after the server's expansion read them
                const opened = new Branch(branchType, context, target.id);
                const expansion = context.expansion;
                if (expansion !== undefined && (await opened.count()) !== expansion.length) {
                    throw StackError.of("UNMERGED_CALLS", "branch has calls the merge did not run");
                }

                // drop the rows the merge wrote for real and close the branch
                await opened.store(context, []);

                return context.update({ state: "merged" });
            },
        },
        discard: async (call) => {
            // drop the branch's rows and close it
            const target = branchType.requireOpen(call.target);
            await new Branch(branchType, call, target.id).store(call, []);

            return call.update({ state: "discarded" });
        },
        rebuild: async (call) => {
            // leave a closed branch, which its controller found stale while open
            const target = call.target;
            if (target.state !== "open") {
                return target;
            }

            // replay every call over the main line
            const opened = new Branch(branchType, call, target.id);

            return call.update(await opened.build(call, target, await opened.entries(), []));
        },
    },
});

/** One call of a branch, in the order it replays. */
export const branchEntry = defineObject(
    {
        name: "branch-entry",
        plural: "branchEntries",
        scope: space,
        nested: { in: branch, delete: "cascade", receive: "push" },
        fields: {
            /** The call's place on the branch. */
            position: field.integer(),
            /** The package-qualified object type. */
            type: field.json(ObjectTypeReference),
            /** The method's name, such as create. */
            method: field.string(),
            /** The method's input, as the call recorded it. */
            input: field.json(schema.jsonObject()),
            /** The release of the object type's package the call was made against. */
            release: field.string(),
            /** The principal who made the call, whom a merge records as its caller. */
            author: field.subject(),
        },
        permissions: { read: through("parent", "read"), record: through("parent", "push") },
        constraints: {
            position: { kind: "unique", on: ["parentId", "position"] },
        },
    },
    (method) => ({
        create: method.create("record", {
            visibility: "package",
            fields: [
                "position",
                "type",
                "method",
                "input",
                "release",
                "authorPackageId",
                "authorType",
                "authorScope",
                "authorId",
            ],
        }),
    }),
);

/** A change a branch makes to one row over the main line, as its calls leave the row. */
export const branchChange = defineObject(
    {
        name: "branch-change",
        plural: "branchChanges",
        scope: space,
        nested: { in: branch, delete: "cascade", receive: "push" },
        fields: {
            /** The SQL name of the row's table. */
            table: field.string(),
            /** The row's key columns. */
            key: field.json(schema.jsonObject()),
            /** The row as the branch leaves it, absent when the branch removes it. */
            after: field.json(schema.jsonObject()).optional(),
            /** The main line's row the branch replaces, absent when the branch creates it. */
            before: field.json(schema.jsonObject()).optional(),
        },
        permissions: { read: through("parent", "read"), record: through("parent", "push") },
    },
    (method) => ({
        create: method.create("record", {
            visibility: "package",
            fields: ["table", "key", "after", "before"],
        }),
        update: method.update("record", { visibility: "package", fields: ["after", "before"] }),
        delete: method.delete("record", { visibility: "package" }),
    }),
);

/** A space's branch types. */
export const branchType = new BranchType({
    object: branch,
    entry: branchEntry,
    change: branchChange,
});

/** Refuse merging a branch into a space requiring approvals unless a change request has them. */
async function requireReviewed(
    context: Pick<MethodContext, "read" | "scope">,
    branchId: Identifier<"branch">,
): Promise<void> {
    // read how many approvals the space requires in the call's view, merging directly at none
    const view = { read: <Read extends Table>(table: Read) => context.read(table) };
    const required = await SettingValue.resolve(view, branchApprovals, { scope: context.scope });
    if (required === 0) {
        return;
    }

    // read the branch's open change requests
    const requested = await context.read(changeRequest).findMany({
        where: {
            parentPackageId: branch.package.id,
            parentType: branch.name,
            parentId: branchId,
            state: "open",
        },
    });

    // merge once a change request has the approvals the space and the request require, and none rejects
    for (const each of requested) {
        const decided = await context.read(approval).findMany({
            where: {
                parentPackageId: changeRequest.package.id,
                parentType: changeRequest.name,
                parentId: each.id,
            },
        });
        if (new ApprovalQuorum(Math.max(required, each.count)).decide(decided) === "approved") {
            return;
        }
    }
    throw StackError.of(
        "CHANGE_REQUEST_REQUIRED",
        `branch ${branchId} can only merge through an approved change request`,
    );
}
