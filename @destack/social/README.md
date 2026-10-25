# @destack/social

`@destack/social` adds comments, reactions, read receipts, favourites and presence to any object type.

```ts
export const article = defineObject(
    {
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
    },
    (method) => ({ create: method.create("manage") }),
);
```

## Comments

A comment starts a thread on its parent, `reply` answers the thread through its first comment, and comments mention principals at spans of their text.

```ts
const thread = await client
    .mutate(comment)
    .create({ parent, body: { text: "Shorten this", mentions } }).confirmed;
await client.mutate(comment).reply({ id: thread.id, text: "Done" });
await client.mutate(comment).resolve({ id: thread.id });
```

## Marks and presence

A receipt and a favourite are one per principal and object, so marking again moves the receipt to now and keeps the star; presence says what a writer does, at most 1 KiB of place outside text.

```ts
await client.receipt.create({ ...host }); // the same receipt, read at now
await client.favourite.create({ ...host }); // the same favourite
await client.presence.update({ id, status: "typing", selection }); // viewing, editing, typing or idle
```

## Access

A comment's access follows its host, as in Notion and Figma.

```ts
read: through("parent", "read"), // whoever reads a host reads its threads
comment: through("parent", "comment"), // whoever comments on it comments and reacts
write: intersection(relation("author"), through("parent", "comment")), // authors who still comment on the host
resolve: union(permission("write"), through("parent", "write")), // authors and the host's writers
delete: union(permission("write"), through("parent", "manage")), // authors and the host's managers
```

## Server

The package's server, its default export of `./server`, implements comments that send the `mention`, `thread` and `reply` notifications.

```ts
import socialServer from "@destack/social/server";

const store = new ObjectStore({ servers: [socialServer, notificationServer], ...options });
```
