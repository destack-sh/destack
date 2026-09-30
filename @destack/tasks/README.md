Plan projects as tasks with assignees, states and comment threads.

A project's owner shares it with members, who plan its tasks, and viewers, who follow them.

| Relation | Project `read` | Task `read` and comment | Task `edit` | Task `work` | Project `manage` |
|---|---|---|---|---|---|
| project `owner` | yes | yes | yes | yes | yes |
| project `member` | yes | yes | yes | yes | |
| project `viewer` | yes | yes | | | |
| task `assignee` | | yes | | yes | |

Everyone who reads a task comments on it through `@destack/social`, and the people a comment mentions hear of it through `@destack/notification`.

```ts
const first = await client.comment.create({
    spaceId,
    parent: { packageId: task.package.id, type: "task", id: taskId },
    body: { text: "@bob, which date?", mentions: [{ offset: 0, length: 4, principal: bob }] },
});
```

A stack installs the package with the database its tasks live in, and another package embeds `tasksTables` in a database of its own.

```ts
import tasks from "@destack/tasks/package";

export const personal = defineSpace({ installations: { tasks: install(tasks, { main: "main" }) } });
```
