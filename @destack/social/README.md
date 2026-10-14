# @destack/social

`@destack/social` adds comments, reactions, read receipts, favourites and presence to any object type.

```ts
export const article = defineObject({
    name: "article",
    plural: "articles",
    scope: space,
    fields: { title: field.string(schema.string().min(1)), commentCount: field.count() },
    attachments: [
        comment.attach({ by: "comment" }),
        reaction.attach({ by: "comment" }),
        receipt.attach({ by: "read" }),
        favourite.attach({ by: "read" }),
        presence.attach({ by: "read" }),
    ],
}).methods((method) => ({ create: method.create("manage") }));
```

## Comments

A comment starts a thread on its parent or replies in one, mentions principals at spans of its text, and its thread is resolved and reopened through the first comment.

```ts
const thread = await client
    .mutate(comment)
    .create({ parent, body: { text: "Shorten this", mentions } }).confirmed;
await client
    .mutate(comment)
    .create({ parent, body: { text: "Done", mentions: [] }, threadId: thread.id });
await client.mutate(comment).resolve({ id: thread.id });
```

## Server

The package's server, its default export of `./server`, implements comments that send the `mention`, `thread` and `reply` notifications.

```ts
import socialServer from "@destack/social/server";

const { store } = ObjectExtension.compose([socialServer, notificationServer], options);
```

## Tables

`socialTables` lists the tables of comments, reactions, receipts and favourites.

```ts
import { socialTables } from "@destack/social/stack";

const database = defineDatabase({ name: "main", tables: socialTables });
```
