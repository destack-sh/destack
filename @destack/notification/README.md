# @destack/notification

Declare notifications, notify recipients, watch objects and deliver them as messages.

## Overview

A package declares its notifications, and its methods notify recipients; each person's inbox lives in their home, and delivery follows their focus and summary settings.

- **Notification**: a kind of notice a package sends, with its payload, content, summary and actions; `call` notifies named recipients or an audience.
- **Activity**: one notice for its recipients about a source object, kept beside the object.
- **Subscription**: a person watching an object, so its `subscribers` audience reaches them.
- **Inbox**: a person's notifications in their home, each with its read, snooze and archive state.
- **Summary**: the notices a person's schedule saves, sent together once due.

```ts
import { defineNotification } from "@destack/notification/declare";
import { argument, plural, t } from "@destack/locale";
import { schema } from "@destack/schema";
import type { ObjectReference, Subject } from "@destack/sync";

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

(source: ObjectReference, bob: Subject) =>
    mention.call({
        source,
        recipients: [bob],
        reason: "mention",
        payload: { author: "Ada", excerpt: "Can you check this?" },
    });
```

## Examples

### Install notification

A space installs notification for its activities, and each person's home keeps their inbox.

```sh
destack install @destack/notification
```

### Notify an object's subscribers

A notification's `call` reaches an audience instead of named recipients, such as the source's subscribers.

```ts
import { defineNotification } from "@destack/notification/declare";
import { argument, plural, t } from "@destack/locale";
import { schema } from "@destack/schema";
import type { ObjectReference } from "@destack/sync";

const change = defineNotification({
    name: "change",
    title: t`Changes`,
    description: t`A document you follow changed.`,
    payload: schema.object({ summary: schema.string() }),
    interruption: "active",
    preference: { channels: ["desktop", "email"], delivery: "immediate" },
    content: { title: t`Document changed`, body: t`${argument("summary")}` },
    summary: t`${plural(argument("count"), { one: "# change", other: "# changes" })}`,
});

(source: ObjectReference) =>
    change.call({ source, audience: { kind: "subscribers" }, payload: { summary: "Renamed" } });
```

### Test notifications

`TestNotification` serves a space and its people's homes with notification installed, mail and push in process.

```ts
import { TestNotification } from "@destack/notification/test";

await using notifications = await TestNotification.open();
await notifications.join("bob");
const inbox = await notifications.inbox("bob");
```

## Comparison

### Apple's User Notifications

Interruption levels, focus, scheduled summaries and actions follow Apple's User Notifications; three things differ.

- A package declares its notifications, and each person chooses their channels for each: desktop, push or email.
- Each notice refers to its source object, so only people who may read the source receive it.
- A person's inbox lives in their home, so it follows them across every space and device.
