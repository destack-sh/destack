import { type ObjectOf, defineObject, field } from "../src/index.ts";

import { relation, through, union, principal } from "@destack/access";
import { defineDatabase } from "@destack/db";
import { schema } from "@destack/schema";
import { space } from "./fixture/space.ts";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";

/** People, the user principal the example objects reference. */
export const user = defineObject({
    name: "user",
    plural: "users",
    scope: "universe",
    represents: principal.user,
    fields: { name: field.string(schema.string().min(1)) },
    permissions: [],
});

/** Notes stacks declare by name. */
export const note = defineObject({
    name: "note",
    plural: "notes",
    scope: space,
    declarable: { schema: schema.object({ title: schema.string() }) },
    fields: { title: field.string() },
    permissions: [],
});

/** Labels referencing declared notes by name. */
export const label = defineObject({
    name: "label",
    plural: "labels",
    scope: space,
    declarable: { schema: schema.object({ note: schema.string(), text: schema.string() }) },
    fields: { note: field.reference(note, { delete: "restrict" }), text: field.string() },
    permissions: [],
});

/** Teams of users within a space that tasks may list as viewers. */
export const team = defineObject({
    name: "team",
    plural: "teams",
    scope: space,
    fields: { owner: field.reference(principal.user).caller() },
    relations: { member: { subjects: [user] } },
    permissions: { read: relation("owner"), write: relation("owner") },
    shareable: { by: "write" },
});

/** Tasks callers manage through methods within a space. */
export const task = defineObject(
    {
        name: "task",
        plural: "tasks",
        scope: space,
        declarable: { schema: schema.object({ title: schema.string().min(1) }) },
        fields: {
            owner: field.reference(principal.user).caller(),
            title: field.string(schema.string().min(1).max(512)),
            archived: field.boolean().default(false),
            token: field.string().sensitive().optional(),
            estimate: field.integer().optional().guard({ read: "plan", write: "plan" }),
            origin: field.object().optional(),
            current: field
                .reference((): ObjectOf<{ identity: "task-version" }> => taskVersion, {
                    delete: "null",
                })
                .optional(),
            status: field.state({
                initial: "open",
                transitions: {
                    complete: { from: ["open"], to: "done", permission: "write" },
                    reopen: { from: ["done"], to: "open", permission: "write" },
                },
            }),
        },
        recoverable: { within: { minutes: 1 }, by: "write" },
        relations: {
            viewer: { subjects: [user, principal.installation, team.members("member")] },
            editor: { subjects: [user] },
        },
        permissions: {
            read: union(relation("owner"), relation("viewer"), relation("editor")),
            write: union(relation("owner"), relation("editor")),
            plan: relation("owner"),
            tidy: through("scope", "share"),
        },
        shareable: { by: "write" },
    },
    (method) => ({
        create: method.create("write", { fields: ["title"] }),
        update: method.update("write", { fields: ["title", "estimate", "currentId"] }),
        archive: method.mutation({ permission: "write" }),
        export: method.mutation({ permission: "write", prepared: schema.string() }),
        archiveAll: method.mutation({
            permission: "tidy",
            target: "scope",
            output: schema.object({
                /** The tasks archived. */
                archived: schema.number().int().nonnegative(),
            }),
        }),
    }),
);

/** Comments on a task, deleted with it. */
export const comment = defineObject({
    name: "comment",
    plural: "comments",
    scope: space,
    nested: { in: task, delete: "cascade", receive: "write" },
    fields: { text: field.string(schema.string().min(1)) },
    permissions: ["read", "write"],
});

/** Immutable versions of a task's title. */
export const taskVersion = defineObject(
    {
        name: "task-version",
        plural: "taskVersions",
        scope: space,
        nested: { in: task, delete: "cascade", receive: "write" },
        versioned: true,
        fields: { title: field.string(schema.string().min(1)) },
        permissions: { read: through("parent", "read"), write: through("parent", "write") },
    },
    (method) => ({ create: method.create("write") }),
);

/** Copies of a task's title stored in an external store, which each creation prepares. */
export const taskCopy = defineObject(
    {
        name: "task-copy",
        plural: "taskCopies",
        scope: space,
        nested: { in: task, delete: "cascade", receive: "write" },
        fields: { title: field.string(schema.string().min(1)) },
        permissions: { read: through("parent", "read"), write: through("parent", "write") },
    },
    (method) => ({
        create: method.create("write", {
            prepared: schema.string(),
        }),
    }),
);

/** Folders nested in folders. */
export const folder = defineObject(
    {
        name: "folder",
        plural: "folders",
        scope: space,
        nested: { in: "self", optional: true, receive: "write" },
        fields: { name: field.string(schema.string().min(1)) },
        permissions: ["read", "write"],
    },
    (method) => ({
        draw: method.mutation({
            permission: "write",
            target: "scope",
            output: schema.object({
                /** A task identifier the call generated. */
                id: schema.string(),
                /** The numbers the call drew. */
                draws: schema.array(schema.number()),
            }),
        }),
    }),
);

/** The example database with the objects, their access and the journal journal. */
export const objectDatabase = defineDatabase({
    name: "main",
    ...tablesOf({
        packages: [
            objectPackage,
            { objects: { task, team, comment, taskVersion, taskCopy, folder }, tables: [] },
        ],
        follows: [],
        tables: [space.table, note.table, label.table],
    }),
});
