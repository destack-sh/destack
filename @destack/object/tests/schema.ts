import { defineObject, field, type ObjectType } from "../src/index.ts";
import { journal } from "@destack/audit";
import { relation, through, union, principal } from "@destack/access";
import { defineDatabase } from "@destack/db";
import { schema } from "@destack/schema";
import { space } from "./fixture/space.ts";

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
export const task = defineObject({
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
        origin: field.reference("self", { qualified: true }).optional(),
        current: field
            .reference("task-version", (): ObjectType => taskVersion, { delete: "null" })
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
    },
    shareable: { by: "write" },
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("write", { fields: ["title"] }),
        update: method.update("write", { fields: ["title", "estimate", "currentId"] }),
        archive: method.mutation({ permission: "write" }),
        export: method.mutation({ permission: "write", prepared: schema.string() }),
    }),
});

/** Comments on a task, deleted with it. */
export const comment = defineObject({
    name: "comment",
    plural: "comments",
    scope: space,
    nested: { in: task, delete: "cascade", receive: "write" },
    fields: { text: field.string(schema.string().min(1)) },
    permissions: ["read", "write"],
    methods: (method) => ({ list: method.list("read") }),
});

/** Immutable versions of a task's title. */
export const taskVersion = defineObject({
    name: "task-version",
    plural: "taskVersions",
    scope: space,
    nested: { in: task, delete: "cascade", receive: "write" },
    versioned: true,
    fields: { title: field.string(schema.string().min(1)) },
    permissions: { read: through("parent", "read"), write: through("parent", "write") },
    methods: (method) => ({ create: method.create("write") }),
});

/** Copies of a task's title kept in an external store, which each creation prepares. */
export const taskCopy = defineObject({
    name: "task-copy",
    plural: "taskCopies",
    scope: space,
    nested: { in: task, delete: "cascade", receive: "write" },
    fields: { title: field.string(schema.string().min(1)) },
    permissions: { read: through("parent", "read"), write: through("parent", "write") },
    methods: (method) => ({
        create: method.create("write", {
            prepared: schema.string(),
        }),
    }),
});

/** Folders nested in folders. */
export const folder = defineObject({
    name: "folder",
    plural: "folders",
    scope: space,
    nested: { in: "self", optional: true, receive: "write" },
    fields: { name: field.string(schema.string().min(1)) },
    permissions: ["read", "write"],
});

/** The example database with the objects, their access and the journal journal. */
export const objectDatabase = defineDatabase({
    name: "main",
    tables: [
        note.table,
        label.table,
        journal,
        ...task.tables,
        ...team.tables,
        ...comment.tables,
        ...taskVersion.tables,
        ...taskCopy.tables,
        ...folder.tables,
    ],
    copies: [],
});
