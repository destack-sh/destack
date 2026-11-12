# @destack/social

Attach comments, reactions, receipts, favourites and presence to any object, notifying mentioned and participating principals.

## Overview

`@destack/social` is Liveblocks' comments and presence as objects: comments, reactions, receipts, favourites and presence for any object type.

Object types attach social types:

- `comment`: a thread with replies, mentions and a resolution.
- `reaction`: one emoji from one principal.
- `receipt`: when a principal last read a host.
- `favourite`: a principal's star on a host.
- `presence`: what someone is doing on a host now, such as typing.

Notifications:

- `mention`, `thread` and `reply`: a mention, a new thread, and a reply.
- `SocialError`: the reasons a comment or presence is refused, checked with `SocialError.is`.

Attach every social type to an article, each allowed by one of its permissions:

```ts
import { space } from "@destack/account/object";
import { argument, plural, t } from "@destack/locale";
import { defineObject, field, shareable } from "@destack/object";
import { schema } from "@destack/schema";
import { comment, favourite, presence, reaction, receipt } from "@destack/social/object";

// attach comments and reactions for commenters, the rest for readers
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

## Reference

### Attachments

`attach({ by })` attaches a social type to a host, granted by the host's permission `by`.
A host's `commentCount` field counts its comments, and a comment's `reactionCount` its reactions.

| Type        | Who may do what                                                                                        |
| ----------- | ------------------------------------------------------------------------------------------------------ |
| `comment`   | the host's readers read threads, holders of `by` comment, authors edit, and the host's writers resolve |
| `reaction`  | the host's readers read reactions, and holders of `by` react, one of each emoji each                   |
| `receipt`   | holders of `by` mark the host read, each seeing only their own                                         |
| `favourite` | holders of `by` star the host, each seeing only their own                                              |
| `presence`  | holders of `by` see and write presence, kept in memory                                                 |

## Examples

### Comment and resolve

Start a thread with a mention, reply, and resolve it.

```ts
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";
import type { Mention } from "@destack/social/object";
import type socialPackage from "@destack/social/package";
import type { ObjectReference } from "@destack/sync";

/** Ask a person about an article, then answer and resolve the thread as them. */
export async function askAndResolve(
    client: ClientOf<typeof socialPackage>,
    spaceId: Identifier<"space">,
    article: ObjectReference,
    carol: Mention["principal"],
    requestId: string,
) {
    // start a thread mentioning carol
    const thread = await client.comment.create({
        spaceId,
        requestId,
        parent: { packageId: article.packageId, type: article.type, id: article.id },
        body: {
            text: "@carol, tighten the intro",
            mentions: [{ offset: 0, length: 6, principal: carol }],
        },
    });

    // reply in the thread, and resolve it
    await client.comment.reply({ spaceId, id: thread.id, requestId, text: "Done" });
    await client.comment.resolve({ spaceId, id: thread.id, requestId });
}
```

### React with an emoji

A principal reacts once per emoji to a host or a comment, and each is counted on it.

```ts
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";
import type socialPackage from "@destack/social/package";
import type { ObjectReference } from "@destack/sync";

/** Give an article a thumbs up. */
export async function thumbsUp(
    client: ClientOf<typeof socialPackage>,
    spaceId: Identifier<"space">,
    article: ObjectReference,
    requestId: string,
) {
    const parent = { packageId: article.packageId, type: article.type, id: article.id };
    await client.reaction.create({ spaceId, requestId, parent, emoji: "👍" });
}
```

### Mark read and star

Each reader has one receipt and one favourite per host, private to them.

```ts
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";
import type socialPackage from "@destack/social/package";
import type { ObjectReference } from "@destack/sync";

/** Mark an article read, and star it. */
export async function readAndStar(
    client: ClientOf<typeof socialPackage>,
    spaceId: Identifier<"space">,
    article: ObjectReference,
    requestId: string,
) {
    const parent = { packageId: article.packageId, type: article.type, id: article.id };
    await client.receipt.create({ spaceId, requestId, parent });
    await client.favourite.create({ spaceId, requestId, parent });
}
```

### Show who is editing

A presence is ephemeral, keyed by the writing client's `writerId`, and says what the person does on a host and where.

```ts
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";
import type socialPackage from "@destack/social/package";
import type { ObjectReference } from "@destack/sync";

/** Show that the person is editing an article's body at a position. */
export async function showEditing(
    client: ClientOf<typeof socialPackage>,
    spaceId: Identifier<"space">,
    article: ObjectReference,
    run: string,
    offset: number,
    writerId: string,
) {
    // place the cursor after a character of the body's text
    const at = { element: { run, offset }, side: "after" } as const;
    const selection = { field: "body", anchor: at, head: at };
    const parent = { packageId: article.packageId, type: article.type, id: article.id };

    return await client.presence.create({
        spaceId,
        writerId,
        parent,
        status: "editing",
        selection,
    });
}
```

## Comparison

### Liveblocks

Threads, replies, mentions, reactions, resolution and presence basically follow Liveblocks.
Social rows are objects in the space's database, with their host's access.

| Liveblocks                      | `@destack/social`            | Difference                                                          |
| ------------------------------- | ---------------------------- | ------------------------------------------------------------------- |
| `createThread`, `createComment` | `comment.create`, `reply`    | The same, on any object, with mentions as spans of plain text.      |
| `markThreadAsResolved`          | `resolve`, `reopen`          | The same.                                                           |
| `addReaction`                   | `reaction.create`            | The same, one row per emoji and principal.                          |
| inbox notifications             | `mention`, `thread`, `reply` | Delivered by `@destack/notification`, under each person's settings. |
| `useMyPresence`, `useOthers`    | `presence`                   | An in-memory object, read by the host's readers.                    |
| none                            | `receipt`, `favourite`       | A principal's own read mark, as Matrix's `m.read`, and star.        |
