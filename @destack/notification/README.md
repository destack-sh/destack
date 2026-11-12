# @destack/notification

Declare notifications, notify recipients, watch objects and deliver them as messages.

## Overview

`@destack/notification` is Apple's User Notifications for packages: declared notifications, per-person inboxes, and focus and summary settings.

Packages declare and send notifications:

- `NotificationType`: a kind of notification, declared with `defineNotification`.
- `Notice`: a notification about a source object, to recipients or an audience.
- `subscription`: a person watching an object.

Spaces keep what happened:

- `activity`: one notification for one recipient.
- `announcement`: one notification to an audience.

Homes keep inboxes:

- `notification`: a person's state of one activity: read, snoozed or archived.
- `Inbox`: a person's notifications, latest first.
- `focus`, `preferences` and `summary`: when and how notifications alert.

Declare a notification, then notify the people a comment mentions:

```ts
import { argument, plural, t } from "@destack/locale";
import { defineNotification } from "@destack/notification/declare";
import type { MethodContext } from "@destack/object";
import { schema } from "@destack/schema";
import type { ObjectReference, Subject } from "@destack/sync";

// declare the mention notification
export const mention = defineNotification({
    name: "mention",
    title: t`Mentions`,
    description: t`Someone mentions you in a comment.`,
    payload: schema.object({ author: schema.string(), excerpt: schema.string().max(280) }),
    interruption: "active",
    preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
    content: { title: t`${argument("author")} mentioned you`, body: t`${argument("excerpt")}` },
    summary: t`${plural(argument("count"), { one: "# mention", other: "# mentions" })}`,
});

/** Notify the people a comment mentions, from the method that posts it. */
export async function notifyMentions(
    call: MethodContext,
    comment: ObjectReference,
    mentioned: readonly Subject[],
    excerpt: string,
) {
    await call.mutate(call.caller, [
        mention.call({
            source: comment,
            recipients: mentioned,
            reason: "mention",
            payload: { author: "Ada", excerpt },
        }),
    ]);
}
```

## Reference

### defineNotification

`defineNotification(definition)` declares a notification, and `call(notice)` builds the call that sends it.
Each notification has a key in `preferences`, defaulting to its `preference`.

| Field                  | What it declares                                                                                  |
| ---------------------- | ------------------------------------------------------------------------------------------------- |
| `name`                 | the name within the package, in camel case                                                        |
| `title`, `description` | the label and the explanation settings show, as messages                                          |
| `payload`              | the schema of the values its content renders, at most 2 KiB as JSON                               |
| `interruption`         | `"passive"`, `"active"`, `"timeSensitive"` or `"critical"`                                        |
| `preference`           | the `channels`, of `"desktop"`, `"push"` and `"email"`, and `"immediate"` or `"summary"` delivery |
| `content`              | the `title`, `subtitle` and `body` messages, their arguments the payload's fields                 |
| `summary`              | the line summarizing a thread, its argument the `count`                                           |
| `actions`              | the buttons beside it, the first the default                                                      |
| `collapse`             | how long a thread's later notices join its first, a quarter hour by default                       |

An action in `actions` has these fields:

| Field            | What it is                                                          |
| ---------------- | ------------------------------------------------------------------- |
| `name`, `title`  | the action's name in camel case, and its button's label             |
| `isDestructive`  | whether it destroys or declines                                     |
| `text`           | the text field it asks for, with its `placeholder` and `button`     |
| `type`, `method` | the package's own object type, and the mutating method it calls     |
| `target`         | the payload field naming the object it calls, the source by default |

### Notice

A notice has a `source`, a `payload`, and `recipients` with a `reason` or an `audience`.

| Field                  | What it is                                                                                                                                          |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `source`               | the object it is about, which recipients must be able to read                                                                                       |
| `payload`              | the values its content renders                                                                                                                      |
| `recipients`, `reason` | up to 100 principals, and why they hear of it: `"author"`, `"mention"`, `"assigned"`, `"participating"` or `"subscribed"`, as code forges have them |
| `audience`, `excluded` | an audience, and the principals it skips besides the caller                                                                                         |
| `key`                  | the identity a later notice with the same key replaces                                                                                              |
| `thread`               | the thread it groups in, the source's identifier by default                                                                                         |

An `audience` is one of three, and `Subscription.subscribe(source, owner, reason)` builds a subscribing call:

| Audience                                     | Who it reaches                                   |
| -------------------------------------------- | ------------------------------------------------ |
| `{ kind: "subscribers" }`                    | the source's subscribers, each with their reason |
| `{ kind: "members", reason }`                | the space's members, as a chat's `@channel`      |
| `{ kind: "permission", permission, reason }` | the users with a permission on the source        |

### Inbox

`new Inbox(rows, now)` orders notifications latest first.
`active()` lists unarchived, unsnoozed entries, and `unread({ space, packageId })` counts unread ones.
`NotificationInbox` and `NotificationBadge`, from `@destack/notification/view`, show them in a view.

## Examples

### Notify an object's subscribers

Subscribe people to a document, then announce changes to its subscribers.

```ts
import { argument, plural, t } from "@destack/locale";
import { defineNotification } from "@destack/notification/declare";
import { Subscription } from "@destack/notification/object";
import type { MethodContext } from "@destack/object";
import { schema } from "@destack/schema";
import type { ObjectReference, Subject } from "@destack/sync";

// declare the change notification, replacing earlier ones per document
const change = defineNotification({
    name: "change",
    title: t`Changes`,
    description: t`A document you follow changed.`,
    payload: schema.object({ summary: schema.string() }),
    interruption: "active",
    preference: { channels: ["desktop", "email"], delivery: "summary" },
    content: { title: t`Document changed`, body: t`${argument("summary")}` },
    summary: t`${plural(argument("count"), { one: "# change", other: "# changes" })}`,
});

/** Subscribe an assignee to a document, and announce a change to its subscribers. */
export async function assignAndAnnounce(
    call: MethodContext,
    document: ObjectReference,
    assignee: Subject,
) {
    await call.mutate(call.caller, [
        Subscription.subscribe(document, assignee, "assigned"),
        change.call({
            source: document,
            audience: { kind: "subscribers" },
            key: "change",
            payload: { summary: "Renamed" },
        }),
    ]);
}
```

### Reply from a notification

An action calls a method of the source's type, here with text the person types in the banner.
`timeSensitive` breaks through a focus, after `passive` and `active` and before `critical`.

```ts
import { argument, plural, t } from "@destack/locale";
import { defineNotification } from "@destack/notification/declare";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";

// declare documents people reply to
const document = defineObject(
    {
        name: "document",
        plural: "documents",
        title: t`${plural(argument("count"), { one: "Document", other: "Documents" })}`,
        container: "scope",
        fields: { title: field.string() },
        access: { permissions: ["read"] },
    },
    (method) => ({
        reply: method.mutation({
            title: t`Reply`,
            permission: "read",
            input: schema.object({ text: schema.string().min(1) }),
        }),
    }),
);

// ask for a review that breaks through focus, answered with a reply
export const review = defineNotification({
    name: "review",
    title: t`Review requests`,
    description: t`A document waits for your review.`,
    payload: schema.object({ title: schema.string() }),
    interruption: "timeSensitive",
    preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
    content: { title: t`Review requested`, body: t`${argument("title")}` },
    summary: t`${plural(argument("count"), { one: "# review", other: "# reviews" })}`,
    actions: [
        {
            name: "reply",
            title: "Reply",
            text: { placeholder: "Reply", button: "Send" },
            type: document,
            method: "reply",
        },
    ],
});
```

### Show the inbox

`NotificationBadge` counts unread notifications, and `NotificationInbox` lists them.

```tsx
import { NotificationBadge, NotificationInbox } from "@destack/notification/view";

/** Show the badge of one space's unread notifications above the person's inbox. */
export function InboxPanel(properties: { readonly space: string }) {
    return (
        <section>
            {/* Badge */}
            <NotificationBadge space={properties.space} />

            {/* Inbox */}
            <NotificationInbox onOpen={(entry) => console.log(entry.activity.content.title)} />
        </section>
    );
}
```

### Test notifications

`TestNotification` serves a space and its people's homes.

```ts
import { TestNotification } from "@destack/notification/test";
import { expect, test } from "@destack/test";

test("start bob with an empty inbox", async () => {
    // serve the space, and make bob a member
    await using notifications = await TestNotification.open();
    await notifications.join("bob");

    // read his inbox
    const inbox = await notifications.inbox("bob");

    expect(inbox.unread()).toBe(0);
});
```

## Comparison

### Apple's User Notifications

Interruption levels, focus, scheduled summaries, threads and actions follow Apple's User Notifications.
Only people who may read a notification's source receive it.

| Apple                             | `@destack/notification` | Difference                                                 |
| --------------------------------- | ----------------------- | ---------------------------------------------------------- |
| `UNNotificationInterruptionLevel` | `interruption`          | The same four levels.                                      |
| `UNNotificationCategory`, actions | `actions`               | Each calls a mutating method of the declaring package.     |
| `threadIdentifier`                | `thread`                | Later notices within `collapse` join the first, counted.   |
| Focus, scheduled summary          | `focus`, `summary`      | Settings of the person, followed on every channel.         |
| notification settings per app     | `preferences`           | Per notification, with desktop, push and email channels.   |
| Notification Center               | `Inbox`                 | Lives in the person's home, across every space and device. |

### GitHub notifications

Reasons and subscriptions follow GitHub's notifications.

| GitHub                                                   | `@destack/notification`     | Difference                                               |
| -------------------------------------------------------- | --------------------------- | -------------------------------------------------------- |
| `reason`: `author`, `mention`, `assign`, `subscribed`, … | `reason`                    | Five reasons, `assigned` and `participating` among them. |
| watching a repository or thread                          | `subscription`              | Any object, with its reason.                             |
| mark as read, done, unsubscribe                          | `read`, `archive`, `delete` | Plus `snooze`.                                           |
