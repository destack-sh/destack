import { principal } from "@destack/access";
import { client } from "@destack/account/object";
import { t } from "@destack/locale";
import { type Content, Destination, MessageKey, message } from "@destack/message";
import type { MethodContext, MethodContextOf, ResultOf } from "@destack/object";
import { Duration, type Identifier, nth, present, TimeZone } from "@destack/schema";
import { ObjectReference, Scope } from "@destack/sync";
import { ScheduledCall } from "@destack/workflow";
import {
    type Decision,
    Recipient,
    type RecipientDestination,
    Routing,
    Settings,
} from "../message/index.ts";
import type { NotificationContent } from "../notification/content.ts";
import { Inbox, InboxEntry } from "../notification/inbox.ts";
import { Activity, activity } from "../object/activity.ts";
import {
    deliver,
    Notification,
    notification,
    scheduled,
    summarize,
} from "../object/notification.ts";
import type { InterruptionLevel } from "../preference/preference.ts";
import type { NotificationServer } from "./server.ts";

/** How urgently each interruption level's push goes out, as RFC 8030's urgencies. */
const URGENCY = {
    passive: "very-low",
    active: "normal",
    timeSensitive: "high",
    critical: "high",
} as const satisfies Record<InterruptionLevel, string>;

/** The notification a push shows, with its activity. */
interface PushNotice {
    /** The notification. */
    readonly notification: Notification["id"];
    /** Its activity's last occurrence. */
    readonly occurrence: Activity;
}

/** Implement a home's notifications: start, decide and send an occurrence's delivery. */
export function implementNotification(server: NotificationServer): typeof notification {
    return notification.implement({
        methods: {
            plan,
            decide: (context) => decide(context, server),
            send: (context) => send(context, server),
            schedule,
            summarize: (context) => summarizeAt(context, server),
            forget,
        },
    });
}

/** Start the delivery of the activity's last occurrence, once per occurrence. */
async function plan(
    context: MethodContextOf<typeof notification, "plan">,
): Promise<ResultOf<typeof notification, "plan">> {
    // start one run per occurrence, keyed by the notification and the occurrence's time
    const row = context.target;
    const entry = present(await InboxEntry.read(context, row), `the activity of ${row.id}`);
    const { occurredAt } = entry.activity;
    const start = deliver.start(
        { notification: row.id, occurredAt },
        { key: `${row.id}/${occurredAt}`, actor: context.self.subject },
    );
    await context.mutate(context.self, [start]);

    return context.update({ plannedAt: context.now });
}

/** Decide what each waiting destination does with the occurrence now. */
async function decide(
    context: MethodContextOf<typeof notification, "decide">,
    server: NotificationServer,
): Promise<ResultOf<typeof notification, "decide">> {
    // decide nothing once the home lost the activity, as after a revoked share
    const entry = await InboxEntry.read(context, context.target);
    if (entry === undefined) {
        return { items: [] };
    }

    // route the occurrence under the person's settings to the waiting destinations
    const recipient = await Recipient.read(context, entry.person);
    const settings = nth(await Settings.read(context, entry.person, [entry.activity]), 0);
    const emailDelay = Duration.milliseconds(server.emailDelay);
    const routing = new Routing(entry, recipient, settings, { emailDelay }, context.now);
    const { destinations, occurredAt } = context.input;
    const waiting = recipient
        .reachable(context)
        .filter((to) => destinations === undefined || destinations.includes(Destination.key(to)));

    // skip a replaced occurrence, else decide each destination
    const isReplaced = entry.activity.occurredAt > occurredAt;
    const replaced: Decision = { action: "skip", reason: "replaced" };
    const items = waiting.map((to) => ({
        key: Destination.key(to),
        to,
        decision: isReplaced ? replaced : routing.decide(to),
    }));

    return { items };
}

/** Send the occurrence to one destination now. */
async function send(
    context: MethodContextOf<typeof notification, "send">,
    server: NotificationServer,
): Promise<ResultOf<typeof notification, "send">> {
    // compose the content for the recipient's destination
    const entry = present(await InboxEntry.read(context, context.target), "the activity");
    const recipient = await Recipient.read(context, entry.person);
    const to = recipient.destination(context.input.to);
    const notice = { notification: context.target.id, occurrence: entry.activity };
    const content = compose(server, to, entry.activity.content, notice, context.scope);
    const source = notification.reference(context.scope, context.target.id);

    return { message: await createMessage(context, to, content, source) };
}

/** Keep the occurrence for a scheduled summary, starting the summary's run once per time. */
async function schedule(
    context: MethodContextOf<typeof notification, "schedule">,
): Promise<ResultOf<typeof notification, "schedule">> {
    // start one summary run per home and time
    const { at } = context.input;
    const start = summarize.start({ at }, { key: String(at), actor: context.self.subject });
    await context.mutate(context.self, [start]);

    return context.update({ summaryAt: at });
}

/** Send the scheduled summary due at a time to each summary destination, once. */
async function summarizeAt(
    context: MethodContextOf<typeof notification, "summarize">,
    server: NotificationServer,
): Promise<ResultOf<typeof notification, "summarize">> {
    // read the notifications waiting for the summary, with their activities
    const { at } = context.input;
    const states = await context.read(notification).findMany({ where: { summaryAt: at } });
    const activities = await context.read(activity).findMany({
        where: { id: { in: states.map((state) => Notification.activity(state.id)) } },
    });
    const { entries } = new Inbox(states, activities, context.now);
    const [lead] = entries;
    if (lead === undefined) {
        return {};
    }

    // read where the person receives notifications, and each entry's settings
    const { person } = lead;
    const recipient = await Recipient.read(context, person);
    const settings = await Settings.read(
        context,
        person,
        entries.map((entry) => entry.activity),
    );

    // send each destination the entries it takes in the summary, in the person's language
    const own = await server.catalog(context, import.meta.destack.package.id);
    const locale = own.localization(recipient.locale);
    const options = { emailDelay: Duration.milliseconds(server.emailDelay), summaryAt: at };
    const routings = entries.map(
        (entry, index) => new Routing(entry, recipient, nth(settings, index), options, context.now),
    );
    const decided = recipient.reachable(context).map((to) => ({
        to,
        decisions: routings.map((routing) => routing.decide(to)),
    }));
    for (const { to, decisions } of decided) {
        // send the entries the destination takes, for the latest of them
        const included = entries.filter((_entry, index) => nth(decisions, index).action === "send");
        const [latest] = included;
        if (latest !== undefined) {
            const content: NotificationContent = {
                title: locale.render(t`Scheduled summary`),
                body: Activity.summarize(included.map((entry) => entry.activity)).join("\n"),
            };
            const composed = compose(server, to, content, undefined, context.scope);
            const source = notification.reference(context.scope, latest.notification.id);
            await createMessage(context, to, composed, source);
        }
    }

    // schedule a snoozed entry for the summary after its snooze, and release the rest
    const next = entries.flatMap((entry, index) => {
        // read the latest time a destination defers the entry to
        const deferred = decided.flatMap(({ decisions }) => {
            const decision = nth(decisions, index);

            return decision.action === "defer" ? [decision.until] : [];
        });
        if (deferred.length === 0) {
            return [];
        }

        // join the first summary after the latest deferral
        const after = Math.max(...deferred);
        const nextAt = TimeZone.next(recipient.timeZone, nth(settings, index).summary.times, after);

        return [
            notification.call("schedule", {
                ...notification.target(entry.notification),
                at: nextAt,
            }),
        ];
    });
    await context.mutate(context.self, [
        ...next,
        notification.call("reschedule", {
            ...notification.in(context.scope),
            where: { summaryAt: at },
            summaryAt: null,
        }),
    ]);

    return {};
}

/** Forget a browser whose push service forgot it, as long as the recipient lists it. */
async function forget(
    context: MethodContextOf<typeof notification, "forget">,
): Promise<ResultOf<typeof notification, "forget">> {
    // send the account the browser's unsubscription, acting for the space
    const entry = present(await InboxEntry.read(context, context.target), "the activity");
    const { person } = entry;
    const recipient = await Recipient.read(context, person);
    const unsubscriptions = recipient.browsers
        .filter((each) => each.url === context.input.url)
        .map((browser) =>
            ScheduledCall.send(
                scheduled,
                client.call("unsubscribe", { id: browser.id, userId: person.id }),
                { actor: principal.space.reference(Scope.universe.id, context.scope) },
            ),
        );
    await context.mutate(context.self, unsubscriptions);

    return context.target;
}

/** Send a home's message for a notification, its content sealed to the home's message key. */
async function createMessage(
    context: MethodContext,
    to: RecipientDestination,
    content: Content | undefined,
    source: ObjectReference,
): Promise<Identifier<"message">> {
    // seal the content to the space's message content key, showing a desktop message without content
    const sealing = message.call("contentKey", message.in(context.scope));
    const ciphertext =
        content === undefined
            ? undefined
            : await MessageKey.of((await context.query(context.self, sealing)).key).seal(
                  content,
                  context.scope,
              );

    // create the message for its notification
    const [created] = await context.mutate(context.self, [
        message.call("create", {
            ...message.in(context.scope),
            to,
            ...ObjectReference.columns("source", source),
            ...(ciphertext === undefined ? {} : { ciphertext }),
        }),
    ]);

    return created.id;
}

/** Compose a message's content for its destination: none to a desktop, an email, or a push. */
function compose(
    server: NotificationServer,
    to: RecipientDestination,
    content: NotificationContent,
    notice: PushNotice | undefined,
    home: string,
): Content | undefined {
    // show a desktop message without content
    if (to.channel === "desktop") {
        return undefined;
    }
    // email the content
    else if (to.channel === "email") {
        const text = [content.subtitle, content.body].filter((line) => line !== undefined);

        return { channel: "email", subject: content.title, text: text.join("\n\n") };
    }

    // push a summary, or one notification with its activity and actions
    const data =
        notice === undefined
            ? { scope: home, content: content, actions: [] }
            : {
                  notification: notice.notification,
                  scope: home,
                  content: content,
                  activity: { scope: notice.occurrence.scope, id: notice.occurrence.id },
                  actions: notice.occurrence.actions.map((action) => ({
                      name: action.name,
                      title: action.title,
                      isDestructive: action.isDestructive === true,
                      hasText: action.text !== undefined,
                  })),
              };
    const level = notice === undefined ? "passive" : notice.occurrence.interruption;

    return {
        channel: "push",
        data,
        urgency: URGENCY[level],
        ttl: Duration.seconds(server.pushTtl),
        ...(notice === undefined ? {} : { topic: Notification.topic(notice.notification) }),
    };
}
