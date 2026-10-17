# @destack/notification

`@destack/notification` declares notifications, records them as activities for each recipient, keeps each person's inbox in their home, and sends them as messages under the person's focus and summary settings.

```ts
await call.mutate(call.caller, [
    mention.call({ source, recipients: [bob], reason: "mention", payload: { author, excerpt } }),
]);
await home.mutate.notification.read({ id });
```

## Declarations

`defineNotification` declares a notification: its payload, its content as catalog messages over the payload's fields, its summary over `count`, and the actions it offers.

```ts
import { defineNotification } from "@destack/notification/declare";

export const mention = defineNotification({
    name: "mention",
    title: "Mentions",
    description: "Someone mentions you in a comment.",
    payload: schema.object({ author: schema.string(), excerpt: schema.string().max(280) }),
    interruption: "active",
    preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
    content: { title: t`${argument("author")} mentioned you`, body: t`${argument("excerpt")}` },
    summary: t`${plural(argument("count"), { one: "# mention", other: "# mentions" })}`,
    actions: [{ name: "open", title: "Open", type: page, method: "open" }],
});
```

## Notifying

An object type attaches `activity`, `announcement` and `subscription`; a notification's `call` notifies named recipients or an audience.

```ts
export const page = defineObject({
    ...definition,
    attachments: [
        activity.attach({ by: "read" }),
        announcement.attach({ by: "read" }),
        subscription.attach({ by: "read" }),
    ],
});
await call.mutate(call.caller, [
    subscription.call("add", { parent, owner: Subject.key(caller), reason: "participating" }),
    update.call({ source, audience: { kind: "subscribers" }, payload }),
]);
```

## Inbox

A person's home replicates their activities and keeps one `notification` of each with its read, snooze and archive state.

```ts
const entries = Inbox.entries(notifications, activities, Date.now());
const unread = Inbox.unread(entries, { space: spaceId });
await home.mutate.notification.snooze({ id, until: tomorrow });
```

## Delivery

Each new occurrence starts a `deliver` workflow run in the person's home, which decides every destination in rounds: `Decision.of` sends, defers, keeps for the scheduled summary, or skips, from the person's preference, focus and summary; the `summarize` workflow sends one summary per home and summary time.

```ts
const weekend: Focus = {
    schedules: [{ days: [6, 7], from: "00:00", to: "00:00" }],
    allowed: [],
    isTimeSensitiveAllowed: true,
};
const evening: Summary = { times: ["19:00"], channels: ["email"] };
const decision = Decision.of(to, entry, recipient, settings, { emailDelay }, now); // { action: "defer", until }
```

## Server

The package's server, its default export of `./server`, serves a space's activities, announcements and subscriptions, and a home's notifications with their `deliver` and `summarize` workflows.

```ts
import notificationServer from "@destack/notification/server";

// the package requests in its space: notification.plan, announcement.expand, message.*
// and in the universe, acting for its home under the home's grant: user.recipient
// its optional `push` capability adds client.unsubscribe, pushing only where an installation allows it
const store = new ObjectStore({ servers: [notificationServer, messageServer], ...options });
```

## Views

`NotificationInbox` lists the person's inbox, and `NotificationBadge` counts their unread notifications.

```tsx
import { NotificationBadge, NotificationInbox } from "@destack/notification/view";

<NotificationBadge space={space} />;
<NotificationInbox onOpen={(entry) => open(entry)} />;
```

## Tests

`NotificationFixture` serves an app's objects in a space and each person's home in a database of its own, on a clock the test moves.

```ts
await using fixture = await NotificationFixture.open({
    origin,
    database,
    objects,
    people: { alice, bob },
    actor: "alice",
});
await fixture.dispatch();
Inbox.unread(await fixture.entries("bob"));
```
