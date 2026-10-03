Plan projects as tasks with assignees, states and comment threads.

## Projects

Projects use the default shareable roles, and their tasks inherit them.

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

A task's assignee reads and works it without any role on the project, and a deleted project stays in the trash for 30 days.

## Workflow

Each task moves through its `status` by `start`, `complete`, `cancel` and `reopen`.
Cancelling a task requires `edit`, and the other moves require `work`.
Editors and the assignee hold `work`, and only managers read and write a task's `budget`.

## Comments

Everyone who reads a task comments on it through `@destack/social`, and the people a comment mentions hear of it through `@destack/notification`.

```ts
const first = await client.comment.create({
    spaceId,
    requestId,
    parent: { packageId: task.package.id, type: "task", id: draft.id },
    body: { text: "@bob, which date?", mentions: [{ offset: 0, length: 4, principal: bob }] },
});
```

## Installation

A stack installs the package with the database its tasks live in, and another package embeds `tasksTables` in a database of its own.

```ts
import tasks from "@destack/tasks/package";

export const personal = defineSpace({ installations: { tasks: install(tasks, { main: "main" }) } });
```
