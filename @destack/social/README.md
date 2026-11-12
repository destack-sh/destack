# @destack/social

Attach comments, reactions, receipts, favourites and presence to any object, notifying mentioned and participating principals.

## Overview

An object type attaches the social types it wants, and each attached row's access follows its host, as in Notion and Figma.

- **Comment**: a thread on its host, with replies and mentions at spans of its text; `create`, `reply`, `resolve` and `reopen`.
- **Reaction**: one emoji from one principal on a host or a comment.
- **Receipt**: when a principal last read a host, one per principal and host.
- **Favourite**: a principal's star on a host.
- **Presence**: what a writer does on a host now, such as viewing or typing, with their selection.

```ts
import { space } from "@destack/account/object";
import { argument, plural, t } from "@destack/locale";
import { defineObject, field, shareable } from "@destack/object";
import { schema } from "@destack/schema";
import { comment, favourite, presence, reaction, receipt } from "@destack/social/object";

export const article = defineObject(
    {
        name: "article",
        plural: "articles",
        title: t`${plural(argument("count"), { one: "Article", other: "Articles" })}`,
        scope: space,
        fields: {
            title: field.string(schema.string().min(1).max(200)),
            commentCount: field.count(),
        },
        attachments: [
            comment.attach({ by: "comment" }),
            reaction.attach({ by: "comment" }),
            receipt.attach({ by: "read" }),
            favourite.attach({ by: "read" }),
            presence.attach({ by: "read" }),
        ],
        traits: [shareable({})],
    },
    (method) => ({ create: method.create("manage") }),
);
```

## Examples

### Install social

A space installs social, whose server sends the `mention`, `thread` and `reply` notifications.

```sh
destack install @destack/social
```

### Comment on an object

A comment starts a thread on its host, and `reply` answers the thread through its first comment.

```ts
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";
import type socialPackage from "@destack/social/package";

async (
    client: ClientOf<typeof socialPackage>,
    spaceId: Identifier<"space">,
    thread: Identifier<"comment">,
    requestId: string,
) => {
    await client.comment.reply({ spaceId, id: thread, requestId, text: "Done" });
    await client.comment.resolve({ spaceId, id: thread, requestId });
};
```

### Test social features

`TestSocial` serves a space with social and an articles package installed.

```ts
import { TestSocial } from "@destack/social/test";

await using social = await TestSocial.open();
```

## Comparison

### Liveblocks comments

Threads, replies, mentions, reactions and presence follow Liveblocks; three things differ.

- Social rows are objects in the space's database, attached to any object type with one line.
- A comment's access follows its host: whoever reads the host reads its threads, and whoever comments on it comments and reacts.
- Mentions and replies notify through the notification package, under each person's settings.
