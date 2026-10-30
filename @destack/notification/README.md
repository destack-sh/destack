# @destack/notification

`@destack/notification` declares notifications, records them as activities for each recipient, stores each person's inbox in their home, and sends them as messages under the person's focus and summary settings.

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
    Subscription.subscribe(parent, caller, "participating"),
    update.call({ source, audience: { kind: "subscribers" }, payload }),
]);
```

## Inbox

A person's home replicates their activities and stores one `notification` of each with its read, snooze and archive state.

```ts
const inbox = new Inbox(notifications, activities, now);
const unread = inbox.unread({ space: spaceId });
await home.mutate.notification.snooze({ id, until: tomorrow });
await home.mutate.notification.readMany({ items: [{ id, readAt: now }] });
```

## Delivery

A notification's controller delivers each new occurrence once its `deliveryAt` is due: `Routing.decide` sends, defers, saves for the scheduled summary, or skips each destination from the person's preference, focus and summary schedule, and a `summary` per home and time goes out once due.

```ts
const weekend: Focus = {
    schedules: [{ days: [6, 7], from: "00:00", to: "00:00" }],
    allowed: [],
    isTimeSensitiveAllowed: true,
};
const evening: SummarySchedule = { times: ["19:00"], channels: ["email"] };
const decision = new Routing(entry, recipient, settings, { emailDelay }, now).decide(to); // { action: "defer", until }
```

## Server

The package's server, its default export of `./server`, serves a space's activities, announcements and subscriptions, and a home's notifications and summaries with their controllers.

```ts
import notificationServer from "@destack/notification/server";

// the package requests in its space: notification.plan, summary.plan, announcement.expand, message.*
// and in the universe, acting for its home under the home's grant: user.recipient
// its optional `push` capability pushes only where an installation allows it; message forgets gone browsers
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

`TestNotification` hosts a space of a test universe with a documents application and notifications installed, and five people calling with their own tokens.

```ts
await using fixture = await TestNotification.open({ batch: 2 });
await fixture.call(
    "alice",
    document.call("broadcast", { ...fixture.scope, id, audience: "members" }),
);
await fixture.simulation.idle();
const activities = await fixture.activities();
```
