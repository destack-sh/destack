# @app/tasks

Plan projects as tasks with assignees, states and comment threads.

## Projects

`task.create` with a project's `parentId` creates a task in it, and the task inherits the grants of the project.

```ts
const launch = await client.project.create({ spaceId, requestId, name: "Launch" });
await client.project.grant({ spaceId, id: launch.id, requestId, relation: "editor", subject: bob });
const draft = await client.task.create({
    spaceId,
    requestId,
    parentId: launch.id,
    title: "Draft the announcement",
    assigneeId: dave.id,
});
```

## Assignees

The `assignee` relation lets a person read and work a task without a role on its project.

```ts
permissions: {
    read: union(relation("assignee"), through("parent", "read")),
    work: union(relation("assignee"), through("parent", "edit")),
},
```

## Trash

`project.delete` moves a project to the trash for 30 days, and `project.restore` brings it back for a person who manages it.

```ts
await client.project.delete({ spaceId, id: launch.id, requestId });
await client.project.restore({ spaceId, id: launch.id, requestId });
```

## Workflow

Each `status` transition names the states it leaves, the state it enters and the permission it requires.

```ts
status: field.state({
    initial: "open",
    transitions: {
        start: { from: ["open"], to: "active", permission: "work" },
        complete: { from: ["open", "active"], to: "done", permission: "work" },
        cancel: { from: ["open", "active"], to: "cancelled", permission: "edit" },
        reopen: { from: ["done", "cancelled"], to: "open", permission: "work" },
    },
}),
```

## Budgets

`guard` restricts reads and writes of a task's `budget` to the project's managers.

```ts
budget: field.number().optional().guard({ read: "manage", write: "manage" }),
```

## Comments

`comment.create` adds a comment to a task for anyone who reads the task, and each mention notifies the mentioned person.

```ts
const first = await client.comment.create({
    spaceId,
    requestId,
    parent: { packageId: task.package.id, type: "task", id: draft.id },
    body: { text: "@bob, which date?", mentions: [{ offset: 0, length: 4, principal: bob }] },
});
```

## Installation

`install(tasks, …)` installs the package with its `main` database, and `tasksTables` adds the task tables to another package's database.

```ts
import tasks from "@app/tasks/package";

export const personal = defineSpace({ installations: { tasks: install(tasks, { main: "main" }) } });
```
