# @destack/object

Declare object types with access, methods and sync, and call them from anywhere.

## Overview

A package declares its object types, and Destack derives their API, access checks, audit and sync from them.

- **Object type**: rows of one kind in a scope, with fields, access and methods; every type has `get`, `list`, `create`, `update` and `delete`.
- **Method**: a verb on a type, called by request id, so a retried call runs once; a mutation is one transaction.
- **Trait**: an adjective adding columns, permissions and methods to a type, such as `shareable` or `nested`.
- **Package**: a package's types, views, mounts and resources; its API lives at `/<package>/<type>/<method>`.
- **Client**: a caller of a package's methods, which keeps a scope's objects synced and predicts its own writes.

```ts
import { argument, plural, t } from "@destack/locale";
import { defineObject, definePackage, field, shareable } from "@destack/object";
import type { ClientOf } from "@destack/object/client";
import { schema } from "@destack/schema";

const project = defineObject({
    name: "project",
    plural: "projects",
    title: t`${plural(argument("count"), { one: "Project", other: "Projects" })}`,
    container: "scope",
    fields: { name: field.string() },
});
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
const tasks = definePackage({ objects: [project, task] });

async (client: ClientOf<typeof tasks>, projectId: string, requestId: string) => {
    const created = await client.task.create({ projectId, title: "Write the README", requestId });
    await client.task.update({ projectId, id: created.id, isDone: true, requestId });
    const open = await client.task.list({ projectId, where: { isDone: false } });
};
```

## Examples

### Add a method

A type declares a method only for a change `update` cannot make, and implements it.

```ts
import { argument, plural, t } from "@destack/locale";
import { defineObject, field, shareable } from "@destack/object";

const list = defineObject(
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

### Nest objects

`nested` keeps a type's rows inside a parent's, moving and deleting them with it.

```ts
import { argument, plural, t } from "@destack/locale";
import { defineObject, field, nested } from "@destack/object";

const folder = defineObject({
    name: "folder",
    plural: "folders",
    title: t`${plural(argument("count"), { one: "Folder", other: "Folders" })}`,
    container: "scope",
    fields: { name: field.string() },
});
const page = defineObject({
    name: "page",
    plural: "pages",
    title: t`${plural(argument("count"), { one: "Page", other: "Pages" })}`,
    scope: folder,
    fields: { title: field.string() },
    traits: [nested({ in: folder, delete: "cascade", add: "write" })],
});
```

### Search objects

`searchable` indexes a type's text and the fields callers filter and sort by.

```ts
import { argument, plural, t } from "@destack/locale";
import { defineObject, field, searchable } from "@destack/object";

const article = defineObject({
    name: "article",
    plural: "articles",
    title: t`${plural(argument("count"), { one: "Article", other: "Articles" })}`,
    container: "scope",
    fields: { title: field.string(), body: field.text(), publishedAt: field.time() },
    traits: [searchable({ text: ["title", "body"], sort: ["publishedAt"] })],
});
```

### Rename a field

A release renames a field by converting the stored rows from the old column.

```ts
import { Expression } from "@destack/db";
import { argument, plural, t } from "@destack/locale";
import { defineObject, field } from "@destack/object";

const contact = defineObject({
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

Methods, reactive queries and optimistic transactions follow Convex; Destack differs in three ways.

- Access is declared on the type and decided per row, so no method checks access by hand.
- Every write is a method of the type keeping the state, deriving its route, audit, CLI command and agent tool.
- A space's objects live in one database with every installed package's, so apps join each other's rows.

### Rails resources

Every type gets the standard verbs, as Rails routes a resource; two things differ.

- `get`, `list`, `create`, `update` and `delete` are methods of the type, each with its permission, never controller actions.
- A type declares a method of its own only for a change `update` cannot make, such as `archive`.
