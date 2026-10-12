# @destack/notification

`defineNotification` is Apple's `UNNotificationContent` with its `interruptionLevel` and text input actions, `focus` and `summary` are Apple's Focus and Scheduled Summary as settings, a `subscription` carries GitHub's notification reasons (`author`, `mention`, `assigned`, `participating`, `subscribed`), and each person's inbox is GitHub's notification inbox.

```ts
// UNNotificationInterruptionLevel
defineNotification({ name: "mention", interruption: "timeSensitive", content, summary, actions });
// UNTextInputNotificationAction
actions: { reply: { title: "Reply", text: { placeholder: "Reply", button: "Send" }, effect } };
// GitHub's participating
subscription.call("add", { parent, owner: Subject.key(caller), reason: "participating" });
// a Focus
const weekend: Focus = {
    schedules: [{ days: [6, 7], from: "00:00", to: "00:00" }],
    allowed: [],
    isTimeSensitiveAllowed: true,
};
const evening: Summary = { times: ["19:00"], channels: ["email"] }; // Scheduled Summary
// GitHub's mark all as read
home.mutate.notification.readAll({ where: { readAt: null }, readAt: Date.now() });
```

## Declarations

`defineNotification` declares a notification with its payload, content, summary and actions.

```ts
import { defineNotification } from "@destack/notification/declare";

export const mention = defineNotification({
    name: "mention",
    title: "Mentions",
    description: "Someone mentions you in a comment.",
    payload: schema.object({ author: schema.string(), excerpt: schema.string().max(280) }),
    interruption: "active",
    preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
    content: (payload) => ({ title: t`${payload.author} mentioned you`, body: payload.excerpt }),
    summary: (count) => t`${plural(count, { one: "# mention", other: "# mentions" })}`,
});
```

## Notifying

An object type attaches `activity`, `announcement` and `subscription`, a declaration's `call` notifies recipients or an audience, and `serveActivities` serves the notices, actions, announcements and subscriptions.

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
    mention.call({ source, recipients: mentioned, reason: "mention", payload }),
    update.call({ source, audience: { kind: "subscribers" }, payload }),
]);
const { items } = await call.query(
    call.caller,
    subscription.call("subscribers", { parent, limit }),
);
const objects = { page, ...serveActivities({ notifications: [mention, update] }) };
```

## Inbox

Each person's home copies their activities into `notification` objects, and `NOTIFICATIONS` and `UNREAD` read the inbox.

```ts
const notifications = home.query.notification.findMany(NOTIFICATIONS); // latest first
const unread = home.query.notification.aggregate(UNREAD);
```

## Delivery

`implementInbox` extends each home's store with its inbox, and `decide` sends, defers or skips each delivery under the recipient's contact, focus and summary.

```ts
const extension = implementInbox({ catalogs, key: (space) => messageKey(space) });
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

<NotificationBadge space={space} />;
<NotificationInbox onOpen={(entry) => open(entry)} />;
```

## Tests

`NotificationFixture` serves an app's objects and its people's inboxes with in-memory delivery, on a clock the test moves.

```ts
await using fixture = await NotificationFixture.open({
    origin,
    database,
    objects,
    people: { alice, bob },
    actor: "alice",
});
await fixture.inbox("bob").until((state) => state.unread === 1);
await fixture.dispatch();
fixture.mails.sent;
```
