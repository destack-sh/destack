# @destack/social

`@destack/social` is Liveblocks' comments, reactions, presence and inbox notifications as object types any object attaches: `comment` threads with mentions and text anchors, resolved like a Liveblocks thread, plus `reaction`, `receipt`, `favourite` and `presence`.

```ts
attachments: [
    comment.attach({ by: "comment" }),
    reaction.attach({ by: "comment" }),
    presence.attach({ by: "read" }),
]; // a Liveblocks room's features
client
    .mutate(comment)
    .create({ parent, body: { text: "@bob, shorten this", mentions }, selection }); // createThread with a mention
client.mutate(comment).create({ parent, body: { text: "Done", mentions: [] }, threadId }); // createComment
client.mutate(comment).resolve({ id: threadId }); // markThreadAsResolved
```

## Attachments

`attach` adds a social type to an object type under a permission, and counts such as `commentCount` keep its totals.

```ts
export const article = defineObject({
    name: "article",
    plural: "articles",
    scope: space,
    fields: { title: field.string(schema.string().min(1)), commentCount: field.count() },
    shareable: {},
    attachments: [
        comment.attach({ by: "comment" }),
        receipt.attach({ by: "read" }),
        favourite.attach({ by: "read" }),
    ],
    methods: (method) => ({ get: method.get("read"), list: method.list("read") }),
});
```

## Notifications

`comment` from `@destack/social/server` sends the `mention`, `thread` and `reply` notifications through `@destack/notification`.

```ts
import { comment } from "@destack/social/server";

const objects = {
    ...serveActivities({ notifications: [mention, thread, reply] }),
    comment,
    reaction,
    subscription,
};
```

## Tables

`socialTables` lists the comment, reaction, receipt and favourite tables a space database includes.

```ts
import { socialTables } from "@destack/social/stack";

const space = defineDatabase({ name: "space", tables: [...socialTables, journal] });
```
