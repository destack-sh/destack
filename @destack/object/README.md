# @destack/object

Declare object types with access, methods and sync, and call them from anywhere.

## Overview

`@destack/object` is Convex's model with access and standard methods: object types, traits, methods, and typed clients that sync.

Packages declare object types:

- `defineObject`: an object type with fields, access, traits and methods.
- `field`: a field's type and modifiers, such as `field.string().optional()`.
- Traits, such as `shareable` and `nested`: columns, permissions and methods a type adopts.
- `method`: a type's methods, standard or its own.
- `definePackage`: a package's object types, views, mounts and resources.

Servers implement them:

- `implementPackage`: a package's implementation.
- `MethodContext`: one call: its input, target and caller, and its writes.

Clients call them:

- `createClient` and `ClientOf`: a typed client of a package.
- `ObjectError`: the reasons a call fails, checked with `ObjectError.is`.

Declare projects and tasks, then create, update and list tasks:

```ts
import { argument, plural, t } from "@destack/locale";
import { defineObject, definePackage, field, shareable } from "@destack/object";
import type { ClientOf } from "@destack/object/client";
import { schema } from "@destack/schema";

// declare projects, each a scope
const project = defineObject({
    name: "project",
    plural: "projects",
    title: t`${plural(argument("count"), { one: "Project", other: "Projects" })}`,
    container: "scope",
    fields: { name: field.string() },
});

// declare shareable tasks inside projects
const task = defineObject({
    name: "task",
    plural: "tasks",
    title: t`${plural(argument("count"), { one: "Task", other: "Tasks" })}`,
    scope: project,
    fields: {
        title: field.string(schema.string().max(200)),
        isDone: field.boolean().default(false),
    },
    traits: [shareable({})],
});

// declare the package
const tasks = definePackage({
    title: t`Tasks`,
    description: t`Track tasks in projects.`,
    icon: "check-square",
    color: "orange",
    objects: [project, task],
});

/** Create a task, finish it, and list the open ones. */
export async function finishTask(
    client: ClientOf<typeof tasks>,
    projectId: string,
    requestId: string,
) {
    const created = await client.task.create({
        projectId,
        title: "Write the README",
        requestId,
    });
    await client.task.update({
        projectId,
        id: created.id,
        isDone: true,
        requestId,
    });
    const open = await client.task.list({ projectId, where: { isDone: false } });

    return open;
}
```

## Reference

### defineObject

`defineObject(options, methods)` declares an object type, and `implement({ methods })` implements its own methods.

| Option                    | What it is                                                     |
| ------------------------- | -------------------------------------------------------------- |
| `name`, `plural`, `title` | the type's names, and its title by count                       |
| `container`               | `"scope"` or `"partition"` for a type that holds others        |
| `scope`                   | the scope types the objects live in                            |
| `fields`                  | the objects' fields                                            |
| `key`                     | the field an object is addressed by, unique among live objects |
| `access`                  | permissions, relations and elevation, as in `@destack/access`  |
| `traits`                  | the traits the type adopts                                     |
| `constraints`, `indexes`  | checks, indexes, unique constraints and foreign keys           |
| `aggregates`              | counts and sums kept on referenced objects                     |
| `storage`                 | where the objects live, such as the event log                  |
| `convert`                 | how each release converts stored rows, by version              |
| `mapping`                 | another package's table the objects live in                    |
| `predefined`              | the rows every database has                                    |

### field

| Kind                                               | What it stores                                     |
| -------------------------------------------------- | -------------------------------------------------- |
| `string`, `integer`, `bigint`, `number`, `boolean` | a scalar                                           |
| `time`                                             | an instant in epoch milliseconds                   |
| `enum`, `state`                                    | one of some values, or a state machine's state     |
| `json`                                             | a JSON value checked by a schema                   |
| `message`                                          | a translatable `@destack/locale` message           |
| `text`                                             | a collaborative text, changed by edits             |
| `binary`, `blob`, `blobs`, `blobMap`, `file`       | bytes, content-addressed blobs, or a bucket's file |
| `subject`, `object`, `reference`                   | a principal, or another object, by reference       |
| `count`, `sum`, `min`, `max`                       | an aggregate over objects belonging to this one    |
| `fractionalIndex`                                  | a position among siblings                          |

| Modifier                                  | What it does                                             |
| ----------------------------------------- | -------------------------------------------------------- |
| `optional()`, `default(v)`                | allows absence, or fills a value at creation             |
| `sensitive()`, `personal()`               | encrypts the value at rest, or marks it as personal data |
| `immutable()`, `caller()`                 | writes it only at creation, or fills it with the creator |
| `guard(read, write)`, `guardWrite(write)` | requires permissions to read or write it                 |
| `open()`                                  | accepts subjects other types contribute                  |

### Traits

A trait adds fields, methods and rules to a type, each declared with its options.

| Trait         | What it does                                               | Options                                         | Example                                                     |
| ------------- | ---------------------------------------------------------- | ----------------------------------------------- | ----------------------------------------------------------- |
| `shareable`   | shares objects through relationships and invitations       | `{ by }` or `{ isPublic }`, and `read`          | `shareable({})`                                             |
| `nested`      | nests objects under a parent, or in a tree of their type   | `{ in, add, delete, optional, move }`           | `nested({ in: notebook, delete: "cascade", add: "write" })` |
| `searchable`  | finds objects by text, and indexes filter and sort fields  | `{ text, filter, sort }`, each a list of fields | `searchable({ text: ["title"], sort: ["title"] })`          |
| `orderable`   | orders objects among their siblings                        | none, or `{ within }`                           | `orderable()`                                               |
| `presentable` | shows objects with a title, an icon, a cover and an accent | none, or `{ title, subtitle, icon, color }`     | `presentable({ subtitle: "summary" })`                      |
| `versioned`   | keeps immutable, numbered versions of a parent             | none                                            | `versioned()`                                               |
| `recoverable` | deletes objects to a restorable trash                      | `{ within, by, purge, retain }`                 | `recoverable({ within: { days: 30 }, by: "delete" })`       |
| `expiring`    | removes objects once a rule's window passes                | a list of `{ after, from }` rules               | `expiring([{ after: { days: 7 }, from: "expiresAt" }])`     |
| `tracked`     | records objects' history in activities                     | `{ activity, by, session }`                     | `tracked({ activity, by: "update" })`                       |
| `controlled`  | reconciles records with a controller                       | none, or `{ approval: true }`                   | `controlled()`                                              |
| `bindable`    | lets installations and machines bind to objects            | `{ by }`                                        | `bindable({ by: "manage" })`                                |
| `suspendable` | lets scope objects be suspended                            | `{ by }`                                        | `suspendable({ by: "update" })`                             |
| `fencing`     | fences a scope while it moves between machines             | `{ by }`                                        | `fencing({ by: "cancel" })`                                 |
| `provisioned` | provisions objects as a kind's resources                   | `{ kind }`                                      | `provisioned({ kind: ConnectionKind })`                     |
| `propagated`  | replicates objects into the scopes inside theirs           | `{ where }`                                     | `propagated({})`                                            |
| `audited`     | records reads in the audit, beside writes                  | `{ reads }`                                     | `audited({ reads: true })`                                  |
| `declarable`  | lets stacks declare the objects                            | the fields a stack declares                     | `declarable(["release"])`                                   |

### Constraints and indexes

`constraints` are rules each database keeps over the objects' columns, by name.
`indexes` are keys unique across databases, claimed in the universe or an enclosing type, such as an account's handle.

| Constraint   | What it keeps                                                                | Example                                                                                    |
| ------------ | ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| `check`      | a condition every object meets                                               | `{ kind: "check", condition: { name: { characters: "a-z0-9.*-" } } }`                      |
| `index`      | an ordered index, `unique` for one object per key, `where` for a partial one | `{ kind: "index", on: ["scope"], unique: true, where: { kind: "personal" } }`              |
| `unique`     | columns whose non-null values are distinct together                          | `{ kind: "unique", on: ["clientId"] }`                                                     |
| `foreignKey` | a reference to another type's columns, with its delete action                | `{ kind: "foreignKey", on: ["scope"], references: { object: () => account, on: ["id"] } }` |

| Index option | What it is                                                    | Example          |
| ------------ | ------------------------------------------------------------- | ---------------- |
| `on`         | the indexed fields, in key order                              | `["handle"]`     |
| `across`     | the scope of the keys' uniqueness: the universe or a type     | `() => universe` |
| `namespace`  | the namespace shared with other types' same-named indexes     | `"handle"`       |
| `fold`       | characters keys leave out, so lookalike values claim one name | `"hyphens"`      |

### Conditions

Conditions are `@destack/db`'s JSON conditions, with every comparison its `Condition` reference lists.
A type takes them in `list` and `updateMany` filters, `check` constraints, partial indexes, aggregates and `propagated`.

### method

Every type has the standard methods, as AIP-131 to AIP-135 have them, each with its permission.

| Method                                | What it does                                           |
| ------------------------------------- | ------------------------------------------------------ |
| `get`, `list`                         | reads one object, or a page of them                    |
| `create`, `createMany`                | creates one object, or one per item                    |
| `update`, `delete`                    | changes or deletes one object                          |
| `updateMany`, `deleteMany`            | changes or deletes every matching object               |
| `updateItems`                         | updates listed objects at their revisions, all or none |
| `query(options)`, `mutation(options)` | declares a method of the type's own, as in tRPC        |

`list` takes a `where` condition, `orderBy`, a `limit` of 50 by default and at most 1000, and a `cursor`.
Calls take a `requestId`, so a retried call runs once.

### MethodContext

An implementation's method takes a `MethodContext`.

| Member                             | What it is                                            |
| ---------------------------------- | ----------------------------------------------------- |
| `input`, `target`, `caller`, `now` | the input, the loaded object, who calls, and the time |
| `update`, `insert`, `remove`       | writes the target or other rows                       |
| `mutate(caller, calls)`            | runs other methods in this transaction                |
| `query`, `read`                    | reads through methods, or reads rows                  |
| `check`, `require`, `permits`      | decides permissions                                   |

### Call phases

A method's implementation gives a handler, or the phases a call runs in this order.

```text
authorize(call)          refuse the call before any outside work
relate(call, object)     a creation's owner and the subjects it starts related to
expand(call)             the calls to run first, each as its own caller
prepare(call)            outside work before the transaction, under call.idempotencyKey, returning call.prepared
handle(call, next)       the rows the call writes inside the transaction, next() running the standard behaviour
confirm(call, prepared)  after the transaction committed, at least once
cancel(call, prepared)   after it failed, at least once, prepared absent when prepare crashed before returning
```

### definePackage

`definePackage({ title, description, icon, color, objects, views, commands, mounts, resources, access, errors })` declares a package's handle, with how people recognise it: its title, its description, its Phosphor icon and the swatch the stack draws it in.
`implementPackage(handle, code)` implements it, from code or per installation.

## Where calls run

### On a server

A space's store runs a call: it checks access, runs the phases, writes one transaction, and logs the change for replicas.

```text
request ─► authorize ─► relate ─► expand ─► prepare (settlement reserved) ─► transaction: handle ─► log ─► confirm or cancel
                                                                                 └ lost race: run again
```

A settlement row keeps a prepared call's outside work, so a controller confirms or cancels it after a crash.

### On a client

An `ObjectClient` follows a scope into a local replica, predicts its own writes there, and sends them to the store.

```text
client.mutate(...)  ─► predicted on the replica at once ─► sent ─► the store's change replaces the prediction
                                                         └ refused: the prediction is rolled back
```

`prepare`, `confirm` and `cancel` run on the server only, so a method with outside work is never predicted.

### In a sandbox

An installation running in a sandbox serves its types' phases from its own process, one request per phase in its call's session.

```text
store ─► sandbox: authorize, relate, expand, prepare, handle(next ─► store) ─► store: confirm, cancel
```

`confirm` and `cancel` run at the store, where the settlement is kept.

## Examples

### Create, read, update and delete

Every type has the standard methods, and `recoverable` adds `restore` to bring a deleted object back.

```ts
import { argument, plural, t } from "@destack/locale";
import { defineObject, definePackage, field, recoverable, shareable } from "@destack/object";
import type { ClientOf } from "@destack/object/client";

// declare projects, each a scope
const project = defineObject({
    name: "project",
    plural: "projects",
    title: t`${plural(argument("count"), { one: "Project", other: "Projects" })}`,
    container: "scope",
    fields: { name: field.string() },
});

// declare todos in projects, restorable for 30 days
const todo = defineObject({
    name: "todo",
    plural: "todos",
    title: t`${plural(argument("count"), { one: "Todo", other: "Todos" })}`,
    scope: project,
    fields: { title: field.string(), isDone: field.boolean().default(false) },
    traits: [shareable({}), recoverable({ within: { days: 30 }, by: "write" })],
});

// declare the package
const todos = definePackage({
    title: t`Todos`,
    description: t`Track todos in projects.`,
    icon: "check-square",
    color: "grass",
    objects: [project, todo],
});

/** Create a todo, read and finish it, list the open ones, then delete and restore it. */
export async function walkTodo(
    client: ClientOf<typeof todos>,
    projectId: string,
    requests: {
        readonly create: string;
        readonly update: string;
        readonly delete: string;
        readonly restore: string;
    },
) {
    // create a todo, then read it back by its identifier
    const created = await client.todo.create({
        projectId,
        title: "Write the README",
        requestId: requests.create,
    });
    const read = await client.todo.get({ projectId, id: created.id });

    // finish it, then list the open todos, newest first
    await client.todo.update({
        projectId,
        id: read.id,
        isDone: true,
        requestId: requests.update,
    });
    const open = await client.todo.list({
        projectId,
        where: { isDone: false },
        orderBy: { createdAt: "desc" },
        limit: 20,
    });

    // delete it to the trash, then restore it
    await client.todo.delete({
        projectId,
        id: read.id,
        requestId: requests.delete,
    });
    const restored = await client.todo.restore({
        projectId,
        id: read.id,
        requestId: requests.restore,
    });

    return { open, restored };
}
```

### Share an object

`shareable` gives a type `grant`, `revoke`, `relationships` and `explain`.
A grant relates a person to the object by a relation, such as `editor`.

```ts
import { argument, plural, t } from "@destack/locale";
import { principal } from "@destack/access";
import { definePackage, defineObject, field, shareable } from "@destack/object";
import type { ClientOf } from "@destack/object/client";

// declare shareable notebooks
const notebook = defineObject({
    name: "notebook",
    plural: "notebooks",
    title: t`${plural(argument("count"), { one: "Notebook", other: "Notebooks" })}`,
    container: "scope",
    fields: { name: field.string() },
    traits: [shareable({})],
});
const notes = definePackage({
    title: t`Notes`,
    description: t`Keep notes.`,
    icon: "note",
    color: "yellow",
    objects: [notebook],
});

/** Let Bob edit a notebook. */
export async function shareWithBob(
    client: ClientOf<typeof notes>,
    scope: string,
    notebookId: string,
    bob: string,
    requestId: string,
) {
    await client.notebook.grant({
        scope,
        id: notebookId,
        requestId,
        relation: "editor",
        subject: principal.user.reference("universe", bob),
    });
}
```

### Nest notes with a count and a trash

`nested` keeps notes inside a notebook, `aggregates` counts them on it, and `recoverable` keeps deleted notes for 30 days.

```ts
import { argument, plural, t } from "@destack/locale";
import { defineObject, field, nested, recoverable } from "@destack/object";

// declare notebooks with a count of their notes
const notebook = defineObject({
    name: "notebook",
    plural: "notebooks",
    title: t`${plural(argument("count"), { one: "Notebook", other: "Notebooks" })}`,
    container: "scope",
    fields: { name: field.string(), noteCount: field.count() },
});

// declare notes in notebooks, counted while not deleted, and restorable for 30 days
export const note = defineObject({
    name: "note",
    plural: "notes",
    title: t`${plural(argument("count"), { one: "Note", other: "Notes" })}`,
    scope: notebook,
    fields: { title: field.string().default("") },
    aggregates: { noteCount: { function: "count", where: { deletedAt: null } } },
    traits: [
        nested({ in: notebook, delete: "cascade", add: "write" }),
        recoverable({ within: { days: 30 }, by: "write" }),
    ],
});
```

### Address a board by its name

`key` lets calls address an object by a field instead of its identifier.
The value is unique among the live objects of its scope, and deleting the object frees it.

```ts
import { argument, plural, t } from "@destack/locale";
import { defineObject, definePackage, field, recoverable, shareable } from "@destack/object";
import type { ClientOf } from "@destack/object/client";

// declare projects, each a scope
const project = defineObject({
    name: "project",
    plural: "projects",
    title: t`${plural(argument("count"), { one: "Project", other: "Projects" })}`,
    container: "scope",
    fields: { name: field.string() },
});

// declare shareable boards named uniquely within their project, restorable for 30 days
const board = defineObject(
    {
        name: "board",
        plural: "boards",
        title: t`${plural(argument("count"), { one: "Board", other: "Boards" })}`,
        scope: project,
        key: "name",
        fields: { name: field.string() },
        traits: [shareable({}), recoverable({ within: { days: 30 }, by: "write" })],
    },
    (method) => ({
        get: method.get("read"),
        create: method.create("write", { fields: ["name"] }),
    }),
);

// declare the package
const boards = definePackage({
    title: t`Boards`,
    description: t`Plan work on boards.`,
    icon: "kanban",
    color: "iris",
    objects: [project, board],
});

/** Get the roadmap board by its name, delete it, and create a new one under the same name. */
export async function replaceRoadmap(
    client: ClientOf<typeof boards>,
    projectId: string,
    requests: { readonly deleting: string; readonly creating: string },
) {
    const roadmap = await client.board.get({ projectId, id: "roadmap" });
    await client.board.delete({
        projectId,
        id: roadmap.id,
        requestId: requests.deleting,
    });

    return await client.board.create({
        projectId,
        name: "roadmap",
        requestId: requests.creating,
    });
}
```

### Guard a field

A guarded field needs its own permission to read, beyond the object's.

```ts
import { argument, plural, t } from "@destack/locale";
import { defineObject, field } from "@destack/object";

// show cards' titles to readers, and their notes only to managers
export const card = defineObject({
    name: "card",
    plural: "cards",
    title: t`${plural(argument("count"), { one: "Card", other: "Cards" })}`,
    container: "scope",
    fields: {
        title: field.string(),
        note: field.string().optional().guard({ read: "manage" }),
    },
    access: { permissions: ["read", "manage"] },
});
```

### Search, filter and order

`searchable` indexes text, and `list` combines a `SEARCH` with other conditions.

```ts
import { argument, plural, t } from "@destack/locale";
import { definePackage, defineObject, field, searchable } from "@destack/object";
import type { ClientOf } from "@destack/object/client";

// declare articles searched by title
const article = defineObject({
    name: "article",
    plural: "articles",
    title: t`${plural(argument("count"), { one: "Article", other: "Articles" })}`,
    container: "scope",
    fields: { title: field.string(), status: field.enum(["draft", "published"]) },
    traits: [searchable({ text: ["title"], sort: ["title"] })],
});
const blog = definePackage({
    title: t`Blog`,
    description: t`Publish articles.`,
    icon: "article",
    color: "tomato",
    objects: [article],
});

/** List the published articles about Ada, by title. */
export async function findAboutAda(client: ClientOf<typeof blog>, scope: string) {
    const where = { AND: [{ SEARCH: "ada" }, { status: "published" }] };

    return await client.article.list({ scope, where, orderBy: { title: "asc" } });
}
```

### Add a method

A type declares its own method for a change `update` can't make.

```ts
import { argument, plural, t } from "@destack/locale";
import { defineObject, field, shareable } from "@destack/object";

// declare lists, archived by their own method
export const list = defineObject(
    {
        name: "list",
        plural: "lists",
        title: t`${plural(argument("count"), { one: "List", other: "Lists" })}`,
        container: "scope",
        fields: { name: field.string(), archivedAt: field.time().optional() },
        traits: [shareable({})],
    },
    (method) => ({
        archive: method.mutation({ title: t`Archive`, permission: "write" }),
    }),
).implement({
    methods: {
        archive: async (call) => call.update({ archivedAt: call.now }),
    },
});
```

### Run outside work in a method

A method reserves outside work in `prepare`, records it in `handle`, and confirms or cancels it once its transaction ends.

```ts
import { argument, plural, t } from "@destack/locale";
import { defineObject, field, shareable } from "@destack/object";
import { schema } from "@destack/schema";

/** A print service that reserves, confirms and cancels prints by key. */
interface Printer {
    /** Reserve a print once per key, answering its identifier. */
    readonly reserve: (key: string, text: string) => Promise<string>;
    /** Print a reserved print. */
    readonly confirm: (id: string) => Promise<void>;
    /** Drop a reservation by its key. */
    readonly cancel: (key: string) => Promise<void>;
}

/** Declare labels, printed through a print service. */
export function labels(printer: Printer) {
    return defineObject(
        {
            name: "label",
            plural: "labels",
            title: t`${plural(argument("count"), { one: "Label", other: "Labels" })}`,
            container: "scope",
            fields: { text: field.string(), printId: field.string().optional() },
            traits: [shareable({})],
        },
        (method) => ({
            print: method.mutation({
                title: t`Print`,
                permission: "write",
                prepared: schema.string(),
            }),
        }),
    ).implement({
        methods: {
            print: {
                // reserve the print outside the transaction, once per call
                prepare: async (call) =>
                    await printer.reserve(call.idempotencyKey, call.target.text),
                // record the reserved print inside the transaction
                handle: async (call) => await call.update({ printId: call.prepared }),
                // print it once the transaction committed
                confirm: async (_call, printId) => await printer.confirm(printId),
                // drop the reservation by its key once the transaction failed
                cancel: async (call) => await printer.cancel(call.idempotencyKey),
            },
        },
    });
}
```

### Follow a query and write offline

An `ObjectClient` replicates a scope into a local database.
Its queries follow the replica, and its writes show at once and settle later.

```ts
import { argument, plural, t } from "@destack/locale";
import { defineObject, field } from "@destack/object";
import type { ObjectClient } from "@destack/object/client";

// declare notes that can be starred
const note = defineObject(
    {
        name: "note",
        plural: "notes",
        title: t`${plural(argument("count"), { one: "Note", other: "Notes" })}`,
        container: "scope",
        fields: { title: field.string(), starred: field.boolean().default(false) },
        access: { permissions: ["read", "write"] },
    },
    (method) => ({ list: method.list("read"), update: method.update("write") }),
);

/** Print the note titles as they change, star a note, and undo it. */
export async function starAndUndo(
    client: ObjectClient<{ readonly note: typeof note }>,
    id: string,
    signal: AbortSignal,
) {
    // follow the notes in title order
    void (async () => {
        for await (const notes of client.query.note
            .findMany({ orderBy: { title: "asc" } })
            .watch(signal)) {
            console.log(notes.map((item) => item.title));
        }
    })();

    // star a note, shown at once, then undo it
    await client.mutate(note).update({ id, starred: true }).predicted;
    await client.undo().confirmed;
}
```

### Rename a field

`convert` reads stored rows of an earlier release into the new field.

```ts
import { Expression } from "@destack/db";
import { argument, plural, t } from "@destack/locale";
import { defineObject, field } from "@destack/object";

// read the old name column as the new field
export const contact = defineObject({
    name: "contact",
    plural: "contacts",
    title: t`${plural(argument("count"), { one: "Contact", other: "Contacts" })}`,
    container: "scope",
    fields: { fullName: field.string() },
    convert: { "2026.11.0": { fullName: Expression.column("name") } },
});
```

## Comparison

### Convex

Methods, reactive queries and optimistic writes basically follow Convex.

| Convex                  | `@destack/object`                 | Difference                                              |
| ----------------------- | --------------------------------- | ------------------------------------------------------- |
| `query`, `mutation`     | `method.query`, `method.mutation` | Methods belong to a type, with a permission each.       |
| functions check access  | `access`                          | Access is declared on the type and decided per row.     |
| one app's tables        | the space's database              | Apps read each other's objects.                         |
| `useQuery`              | clients and replicas              | Clients sync a scope and predict their writes.          |
| `action`                | `prepare`, `confirm`, `cancel`    | Outside work belongs to a mutation, kept until it ends. |
| durable `action` chains | `@destack/workflow`               | Workflows replay recorded steps.                        |
| `scheduler.runAfter`    | `workflow.start(..., { when })`   | A method starts a run in its own transaction.           |
| `crons`                 | workflow schedule triggers        | Cron expressions or intervals, with overlap policy.     |
| `httpAction`            | package mounts                    | Mounts answer below their package's path.               |
| `internalMutation`      | `visibility: "package"`           | The same.                                               |
| `ctx.storage`           | `field.file`, `field.blob`        | Files are fields of their object.                       |
| vector search           | none yet                          | Text search only.                                       |

### Google AIP

Standard methods follow Google's API Improvement Proposals.

| AIP                         | `@destack/object`                           | Difference                                         |
| --------------------------- | ------------------------------------------- | -------------------------------------------------- |
| AIP-122 resource names      | `key`                                       | A name is unique among live objects only.          |
| AIP-131 to AIP-135          | `get`, `list`, `create`, `update`, `delete` | The same.                                          |
| AIP-160 filters             | `where`                                     | Conditions as JSON, or a filter string in the CLI. |
| AIP-155 request identifiers | `requestId`                                 | Required on every write.                           |
