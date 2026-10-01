# @destack/notification

Declare notifications and deliver them to the inbox, desktop, push and email.

## Notifications

`defineNotification` declares a notification.

```ts
import { defineNotification } from "@destack/notification/declare";

export const mention = defineNotification({
    name: "mention",
    title: "Mentions",
    description: "Someone mentions you in a comment.",
    payload: schema.object({ author: schema.string(), excerpt: schema.string().max(280) }),
    interruption: "active",
    preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
    content: (payload) => ({ title: `${payload.author} mentioned you`, body: payload.excerpt }),
    summary: (count) => `${count} mentions`,
    actions: {
        reply: {
            title: "Reply",
            text: { placeholder: "Reply", button: "Send" },
            effect: ({ source }, call, text) =>
                call.invoke(remark, "create", replyTo(source, text)),
        },
    },
});
```

## Interruption levels

The interruption level decides when a notification alerts.

| Level | Alerts | Breaks through a focus |
|---|---|---|
| `passive` | Never, only the badge and the summary | No |
| `active` | Yes | No |
| `timeSensitive` | Yes, never summarized | Unless the focus refuses |
| `critical` | Yes, whatever the preference | Yes |

## Sources

A source type takes notifications, announcements and subscriptions as attachments.

```ts
attachments: [
    notification.attach({ by: "read" }),
    announcement.attach({ by: "read" }),
    subscription.attach({ by: "read" }),
],
```

## Posting

A server handler notifies recipients or announces to an audience.

```ts
create: async (call, next) => {
    const row = await next();
    await Subscription.add(call, source, call.caller!, "participating");
    await mention.notify(call, { source, recipients: mentioned, reason: "mention", payload });
    await update.announce(call, { source, audience: { kind: "subscribers" }, payload });

    return row;
},
```

## Object types

| Type | What it is |
|---|---|
| `notification` | One recipient's notification, with `read`, `unread`, `readAll` and `snooze` |
| `announcement` | A notification to the source's `subscribers`, the space's `members`, or the users with a `permission` |
| `delivery` | One attempt to deliver a notification on a channel |
| `subscription` | A principal's subscription to a source, with its `Reason` |
| `pushEndpoint` | A browser's Web Push endpoint in the user's scope |

## Reading

A client answers an action and reads a thread.

```ts
await client.mutate(notification).act({ id: row.id, action: "reply", text: "On it" }).confirmed;
await client
    .mutate(notification)
    .readAll({ where: { thread: row.thread, readAt: null }, readAt: Date.now() });
```

## Inbox

`NOTIFICATIONS` lists a person's notifications and `UNREAD` counts them per space and app.

```ts
const badges = home.subscribe(notification, UNREAD);
for await (const groups of badges.watch(signal)) {
    renderBadges(groups);
}
```

## Delivery

A `NotificationServer` serves the notification objects, each under its own controller.

```ts
const push = new WebPushTransport(new Vapid(keys, "mailto:push@destack.app"));
const server = new ObjectServer({
    objects: {
        ...new NotificationServer({ notifications: [mention], recipients, push, mail }).objects(),
        subscription,
    },
    database,
    context,
    journal,
    audit,
});
await new ControlLoop(database, server.controllers(), { report }).run(signal);
```

## Decisions

`decide` sends, defers or skips one delivery.

```ts
const decision = decide({ channel: "push", notification: row, ...circumstances });
```
