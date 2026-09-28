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
    actions: { reply: { title: "Reply", text: { placeholder: "Reply", button: "Send" }, call: ({ source }, text) => replyTo(source, text) } },
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

The package defines five object types.

| Type | Holds |
|---|---|
| `notification` | One recipient's notification, with `read`, `unread`, `readAll` and `snooze` |
| `announcement` | A notification to the source's `subscribers`, the space's `members`, or the holders of a `permission` |
| `delivery` | One attempt to deliver a notification on a channel |
| `subscription` | A principal's subscription to a source, with its `Reason` |
| `pushEndpoint` | A browser's Web Push endpoint in the user's scope |

## Reading

A client answers an action and reads a thread.

```ts
await client.submit(mention.respond(row, "reply", "On it")).confirmed;
await client.mutate(notification).readAll({ where: { thread: row.thread, readAt: null }, readAt: Date.now() });
```

## Inbox

`NOTIFICATIONS` lists a person's notifications and `UNREAD` counts them per space and app.

```ts
const { object: _object, ...query } = UNREAD;
const badges = home.subscribe(notification, query);
for await (const groups of badges.watch(signal)) {
    renderBadges(groups);
}
```

## Delivery

A `Dispatcher` sends push and email from one controller.

```ts
import { Dispatcher, Vapid, WebPushTransport } from "@destack/notification/server";

const dispatcher = new Dispatcher({ notifications: [mention], recipients, push: new WebPushTransport(new Vapid(keys, "mailto:push@destack.app")), mail });
const server = new ObjectServer({ objects: { ...objects, subscription, ...dispatcher.objects }, database, context, journal, audit });
await new ControlLoop(database, [...server.controllers(), dispatcher.controller(server)], { report }).run(signal);
```

## Decisions

`decide` sends, defers or skips one delivery, on the server and the desktop.

```ts
const decision = decide({ channel: "push", notification: row, ...circumstances });
```
