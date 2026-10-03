Attach comments, reactions, read receipts, favourites and presence to any object.

## Hosts

A host attaches each social type with the host permission it requires, such as `comment` to comment and react.

```ts
import { announcement, notification, subscription } from "@destack/notification";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { comment, favourite, presence, reaction, receipt } from "@destack/social";

export const article = defineObject({
    name: "article",
    plural: "articles",
    scope: space,
    fields: {
        title: field.string(schema.string().min(1)),
        commentCount: field.count(),
        reactionCount: field.count(),
    },
    shareable: {},
    attachments: [
        comment.attach({ by: "comment" }),
        reaction.attach({ by: "comment" }),
        notification.attach({ by: "read" }),
        announcement.attach({ by: "read" }),
        subscription.attach({ by: "read" }),
        receipt.attach({ by: "read" }),
        favourite.attach({ by: "read" }),
        presence.attach({ by: "read" }),
    ],
    methods: (method) => ({ get: method.get("read"), list: method.list("read") }),
});
```

## Object types

Comments, reactions, read receipts, favourites and presences each nest in any host that attaches them.

## Comments

A reply names its thread's first comment as `threadId`.

```ts
const parent = { packageId: article.package.id, type: "article", id: articleId };
const first = await client.mutate(comment).create({
    parent,
    body: { text: "@bob, tighten this", mentions: [{ offset: 0, length: 4, principal: bob }] },
    selection: { field: "body", anchor, head },
}).predicted;
client.mutate(comment).create({ parent, body: { text: "Done", mentions: [] }, threadId: first.id });
client.mutate(comment).resolve({ id: first.id });
```

## Notifications

The server's `comment` from `@destack/social/server` subscribes the author and sends the `mention`, `thread` and `reply` notifications.

```ts
import { NotificationServer } from "@destack/notification/server";
import { mention, reply, thread } from "@destack/social";
import { comment } from "@destack/social/server";

const notifications = new NotificationServer({
    notifications: [mention, thread, reply],
    recipients,
    push,
    mail,
});
const objects = { ...notifications.objects(), comment, reaction, subscription };
```
