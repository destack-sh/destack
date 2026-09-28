# @destack/social

Attach comments, reactions, receipts, favourites and presence to any object.

## Hosts

A host spreads `hostRoles` into its definition and attaches the social types.

```ts
const roles = hostRoles([user, group.members("member")]);

export const doc = defineObject({
    ...,
    fields: { ...roles.fields, commentCount: field.count(), reactionCount: field.count() },
    relations: roles.relations,
    permissions: roles.permissions,
    shareable: { by: roles.grantedBy },
    attachments: [
        comment.attach({ by: "comment" }),
        reaction.attach({ by: "comment" }),
        ...[receipt, favourite, presence, notification, announcement, subscription].map((type) => type.attach({ by: "read" })),
    ],
});
```

## Roles

`hostRoles` defines the permissions the attachments read.

| Relation | `read` | `comment` | `edit` | `manage` |
|---|---|---|---|---|
| `owner` | yes | yes | yes | yes |
| `editor` | yes | yes | yes | |
| `commenter` | yes | yes | | |
| `viewer` | yes | | | |

## Object types

Each type nests in any host that attaches it.

| Type | Holds | Methods |
|---|---|---|
| `comment` | A thread's first comment or a reply, with a `Body` and an optional `Selection` of the host's text | `get`, `list`, `create`, `update`, `delete`, `resolve`, `reopen` |
| `reaction` | One author's emoji on a host or a comment | `list`, `create`, `delete` |
| `receipt` | When its owner last read the host | `list`, `create`, `update`, `delete` |
| `favourite` | Its owner's favourite mark on the host | `list`, `create`, `delete` |
| `presence` | A client's ephemeral `status`, text `selection` and other `location` | `list`, `create`, `update`, `delete` |

## Comments

Replies name a thread's first comment as their `thread`.

```ts
const first = await client.mutate(comment).create({
    parent,
    body: { text: "@bob, tighten this", mentions: [{ offset: 0, length: 4, principal: bob }] },
    selection: { field: "body", anchor, head },
}).predicted;
client.mutate(comment).create({ parent, body: { text: "Done", mentions: [] }, thread: first.id });
client.mutate(comment).resolve({ id: first.id });
```

## Notifications

The server `comment` subscribes and notifies through three notifications.

| Notification | Reaches |
|---|---|
| `mention` | The principals a comment mentions |
| `thread` | The host's subscribers, for a thread's first comment |
| `reply` | The thread's subscribers, for a reply |

## Delivery

A host's dispatcher delivers them.

```ts
const dispatcher = new Dispatcher({ notifications: [mention, thread, reply], recipients, push, mail });
```
