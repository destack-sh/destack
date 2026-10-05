# @destack/social

Attach comments, reactions, read receipts, favourites and presence to objects.

## Hosts

`attach` adds a social type to an object type under a permission.

```ts
import { activity, announcement, subscription } from "@destack/notification";
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
        activity.attach({ by: "read" }),
        announcement.attach({ by: "read" }),
        subscription.attach({ by: "read" }),
        receipt.attach({ by: "read" }),
        favourite.attach({ by: "read" }),
        presence.attach({ by: "read" }),
    ],
    methods: (method) => ({ get: method.get("read"), list: method.list("read") }),
});
```

## Comments

`threadId` on a reply points to the first comment of its thread.

```ts
const parent = { packageId: article.package.id, type: "article", id: articleId };
const first = await client.mutate(comment).create({
    parent,
    body: { text: "@bob, shorten this", mentions: [{ offset: 0, length: 4, principal: bob }] },
    selection: { field: "body", anchor, head },
}).predicted;
client.mutate(comment).create({ parent, body: { text: "Done", mentions: [] }, threadId: first.id });
client.mutate(comment).resolve({ id: first.id });
```

## Notifications

`comment` from `@destack/social/server` sends the `mention`, `thread` and `reply` notifications.

```ts
import { serveNotifications } from "@destack/notification/server";
import { mention, reply, thread } from "@destack/social";
import { comment } from "@destack/social/server";

const activities = serveNotifications({ notifications: [mention, thread, reply] });
const objects = { ...activities, comment, reaction, subscription };
```
