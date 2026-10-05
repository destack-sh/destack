# @destack/notification

Declare notifications and deliver them to each person's inbox.

## Declarations

`defineNotification` declares a notification with its payload, content and actions.

```ts
import { plural, t } from "@destack/locale";
import { defineNotification } from "@destack/notification/declare";

export const mention = defineNotification({
    name: "mention",
    title: "Mentions",
    description: "Someone mentions you in a comment.",
    payload: schema.object({
        author: schema.string(),
        excerpt: schema.string().max(280),
    }),
    interruption: "active",
    preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
    content: (payload) => ({
        title: t`${payload.author} mentioned you`,
        body: payload.excerpt,
    }), // rendered for each recipient as notify posts it
    summary: (count) => t`${plural(count, { one: "# mention", other: "# mentions" })}`, // rendered for each recipient as notify posts it
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

### Interruption levels

`interruption` sets when a notification alerts, from `passive` to `critical`.

```text
passive         goes into the summary and never alerts alone
active          alerts, and waits out a focus
timeSensitive   alerts, skips the summary, and passes a focus that allows time-sensitive notifications
critical        alerts at once, whatever the preference or focus
```

### Activities

`activity.attach` lets an object type record activities in the app's space.

```ts
attachments: [
    activity.attach({ by: "read" }),
    announcement.attach({ by: "read" }),
    subscription.attach({ by: "read" }),
],
```

## Objects

Each person's home copies their activities into `notification` rows.

```text
# a home a person leaves drops the rows it copied for them, and their new home copies the activities again
activity       the app's space   what happened for one recipient, answered with act
announcement   the app's space   an activity to subscribers, members or holders of a permission
subscription   the app's space   a principal's subscription to a source, with its Reason
notification   the home          a person's copy of one activity, with read, readAll, snooze and archive
delivery       the home          one attempt to deliver a notification on a channel
```

### Reading

`act` runs an action on an activity, and `readAll` marks notifications read.

```ts
await space.mutate(activity).act({ id: row.source.id, action: "reply", text: "On it" }).confirmed;
await home
    .mutate(notification)
    .readAll({ where: { thread: row.thread, readAt: null }, readAt: Date.now() });
```

### Inbox

`NOTIFICATIONS` lists the inbox latest first, and `UNREAD` counts unread notifications.

```ts
const notifications = home.query.notification.findMany(NOTIFICATIONS);
const badges = home.query.notification.aggregate(UNREAD).subscribe();
for await (const groups of badges.watch(signal)) {
    renderBadges(groups);
}
```

## Service

`inboxService` holds the `notification` and `delivery` objects of each person's home.

```ts
import { inboxService } from "@destack/notification/service";

const server = ObjectServer.serve(inboxService, {
    database,
    callKey,
    handled: Object.values(serveInbox(options)),
    installation,
});
```

### Notifying

`notify` records an activity for each recipient, rendered in their locale.

```ts
create: async (call, next) => {
    const row = await next();
    await Subscription.add(call, source, call.requireCaller(), "participating");
    await mention.notify(call, { source, recipients: mentioned, reason: "mention", payload });
    await update.announce(call, { source, audience: { kind: "subscribers" }, payload });

    return row;
},
```

### Serving notifications

`serveNotifications` serves an app's `activity` and `announcement` objects.

```ts
const objects = {
    ...serveNotifications({ notifications: [mention, update] }),
    document,
};
```

### Delivery

`serveInbox` sends each due email or push delivery as one `message`.

```ts
const objects = serveInbox({ catalogs: await Catalog.read(build) });
// delivery { channel: "push", state: "sending", message: "message-…" } → "sent" once the message's Sent condition is true
```

### Settling

`delivery.settle` records a delivery as sent or failed from its message's `Sent` condition.

```ts
// message { conditions: { Sent: { status: "false" } }, error: { code: "gone" } } → delivery { state: "skipped", reason: "gone" }
await call.change(pushEndpoint, "delete", { id: endpoint, userId });
```

### Desktops

`serveInbox` marks a due desktop delivery sent for the desktop to show.

```ts
// delivery { channel: "desktop", device, state: "sent" } with its notification's content and actions
```

### Contacts

`Contact.read` reads a person's desktops, push endpoints, email, time zone and locale.

```ts
const contact = await Contact.read(database, person); // { desktops, endpoints, email, timeZone: "Europe/Vienna", locale: "de-AT" }
```

### Decisions

`decide` returns whether to send, defer or skip one delivery under its recipient's contact and settings.

```ts
const decision = decide(
    row,
    notification,
    contact,
    { preference, focus, summary },
    emailDelay,
    now,
);
```

## Views

`NotificationInbox` lists the person's inbox, and `NotificationBadge` counts unread notifications.

```tsx
import { NotificationBadge, NotificationInbox } from "@destack/notification/view";

<NotificationBadge space={space} />; // the unread notifications of one space, every space without it
<NotificationInbox onOpen={(entry) => open(entry)} />; // reads useHome({ notification })
```

## Workload

`inboxWorkload` serves each home's inbox as an installation.

```ts
import { inboxWorkload } from "@destack/notification/workload";

const instance = await WorkloadInstance.start(inboxWorkload, {
    resources,
    installation,
    runs,
    callKey,
    report,
    service,
});
```

## Tables

`inboxDatabase` holds each home's notifications and deliveries.

```ts
import { inboxDatabase } from "@destack/notification/stack";

const database = inboxDatabase.get(resources);
```

## Settings

`focus` and `summary` set when notifications stay quiet and when summaries go out.

```ts
import { focus, summary } from "@destack/notification/setting";

const quiet = {
    schedules: [{ days: [6, 7], from: "00:00", to: "00:00" }],
    allowed: [],
    isTimeSensitiveAllowed: true,
};
const evening = { times: ["19:00"], channels: ["email"] };
```

## Tests

`NotificationFixture` serves an app's objects and its people's inboxes with in-memory delivery.

```ts
import { NotificationFixture } from "@destack/notification/test";

const fixture = await NotificationFixture.open({
    origin: { package: task.package, service: "tasks" },
    objects: { project, task, subscription, activity, announcement },
    people: { alice, bob },
    actor: "alice",
});
await fixture.reach("bob", { email: "bob@example.com", timeZone: "Europe/Vienna" });
const inbox = fixture.inbox("bob");
await fixture.call(task, "create", { parentId, title: "Draft", assigneeId: bob.id });
await inbox.until((state) => state.unread === 1);
fixture.wait(15);
await fixture.dispatch(); // fixture.mails.sent, fixture.pushes.requests
```
